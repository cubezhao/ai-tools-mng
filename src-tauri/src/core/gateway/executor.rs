//! 网关渠道执行器
//!
//! 按 `ChannelKind` 选取凭证并向对应上游发起单次请求：CodexOauth 复用 OpenAI OAuth
//! 账号与 `codex::upstream` 工具函数（不经 `CodexPool`），OpenaiCompat / Anthropic 走
//! Base URL + Key。失败切换（failover）由路由层依据候选优先级编排，本层只负责单渠道执行。

use bytes::Bytes;
use reqwest::Method;
use serde_json::{json, Value};
use warp::http::HeaderMap;

use super::canonical::CanonicalRequest;
use super::config::{ChannelKind, GatewayChannel};
use super::translate::stream_bridge::outbound_for;
use crate::platforms::openai::codex::upstream::{
    apply_forward_headers, build_upstream_url, format_transport_error, CODEX_UPSTREAM_ORIGIN,
};
use crate::platforms::openai::modules::{account as account_mod, storage as account_storage};
use crate::proxy_helper::ProxyClient;

/// OAuth token 续期窗口：到期前 5 分钟内提前刷新
const TOKEN_REFRESH_WINDOW_SECS: i64 = 300;

/// 渠道执行错误
#[derive(Debug)]
pub enum GatewayError {
    /// 凭证缺失/解析失败（账号不存在、缺少 Base URL/Key 等）
    Credential(String),
    /// 传输层错误（连接/超时等）
    Transport(String),
}

impl GatewayError {
    pub fn message(&self) -> String {
        match self {
            GatewayError::Credential(m) => m.clone(),
            GatewayError::Transport(m) => m.clone(),
        }
    }
}

/// 网关渠道执行器
pub struct GatewayExecutor {
    app_handle: tauri::AppHandle,
    client: ProxyClient,
}

impl GatewayExecutor {
    /// 复用调用方传入的客户端（由 `AppState` 缓存），避免每请求重建客户端与重复握手
    pub fn new(app_handle: tauri::AppHandle, client: ProxyClient) -> Self {
        Self { app_handle, client }
    }

    /// 经渠道出站转换器把 canonical 请求构造为渠道请求体并发送（标准跨协议路径）
    pub async fn send_canonical(
        &self,
        channel: &GatewayChannel,
        req: &CanonicalRequest,
    ) -> Result<reqwest::Response, GatewayError> {
        let body = outbound_for(channel.wire()).build_request(req);
        let bytes = Bytes::from(serde_json::to_vec(&body).map_err(|e| {
            GatewayError::Credential(format!("渠道请求体序列化失败: {}", e))
        })?);
        self.send(channel, bytes).await
    }

    /// 将已序列化的渠道请求体发往指定渠道，返回上游响应（含非成功状态码，交由路由层判定）
    pub async fn send(
        &self,
        channel: &GatewayChannel,
        body: Bytes,
    ) -> Result<reqwest::Response, GatewayError> {
        match channel.kind {
            ChannelKind::CodexOauth => self.send_codex(channel, body).await,
            ChannelKind::OpenaiCompat => self.send_openai_compat(channel, body).await,
            ChannelKind::Anthropic => self.send_anthropic(channel, body).await,
        }
    }

    /// CodexOauth：复用 OpenAI OAuth 账号与 codex 上游构造工具，转发前校验/刷新 token
    async fn send_codex(
        &self,
        channel: &GatewayChannel,
        body: Bytes,
    ) -> Result<reqwest::Response, GatewayError> {
        let account_id = channel
            .account_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| GatewayError::Credential("Codex 渠道未绑定账号".into()))?;

        let mut account = account_storage::load_account(&self.app_handle, account_id)
            .await
            .map_err(GatewayError::Credential)?;

        // 转发前 token 有效性校验与刷新（复用 openai 平台逻辑），刷新成功则回存
        match account_mod::refresh_token_if_needed(&mut account, TOKEN_REFRESH_WINDOW_SECS, false)
            .await
        {
            Ok(true) => {
                let _ = account_storage::save_account(&self.app_handle, &account).await;
            }
            Ok(false) => {}
            Err(e) => return Err(GatewayError::Credential(format!("Token 刷新失败: {}", e))),
        }

        let access_token = account
            .token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| GatewayError::Credential("OAuth 账号缺少 token".into()))?;
        let chatgpt_account_id = account
            .chatgpt_account_id
            .clone()
            .unwrap_or_else(|| account.email.clone());

        let url = build_upstream_url(CODEX_UPSTREAM_ORIGIN, "/backend-api/codex/responses", None);
        let builder = self.client.request(Method::POST, &url);
        // 规整为 codex 后端可接受的 Responses 请求体（input 数组化 / 补 instructions / stream=true）
        let body = normalize_codex_body(body);
        // 空入站头：仅注入 codex 鉴权与默认头（User-Agent / OpenAI-Beta / originator）
        let builder = apply_forward_headers(builder, &HeaderMap::new(), &access_token, &chatgpt_account_id)
            .header("Content-Type", "application/json")
            .body(body);
        send_builder(builder).await
    }

    /// OpenaiCompat：Base URL + Bearer Key，按线型选 /chat/completions 或 /responses
    async fn send_openai_compat(
        &self,
        channel: &GatewayChannel,
        body: Bytes,
    ) -> Result<reqwest::Response, GatewayError> {
        let base = require_base(channel)?;
        let key = require_key(channel)?;
        let path = if channel.wire.as_deref() == Some("responses") {
            "/responses"
        } else {
            "/chat/completions"
        };
        let url = format!("{}{}", base, path);
        let builder = self
            .client
            .post(&url)
            .bearer_auth(key)
            .header("Content-Type", "application/json")
            .body(body);
        send_builder(builder).await
    }

    /// Anthropic：Base URL + x-api-key，POST /v1/messages
    async fn send_anthropic(
        &self,
        channel: &GatewayChannel,
        body: Bytes,
    ) -> Result<reqwest::Response, GatewayError> {
        let base = require_base(channel)?;
        let key = require_key(channel)?;
        let url = format!("{}/v1/messages", base);
        let builder = self
            .client
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .body(body);
        send_builder(builder).await
    }
}

/// 发送请求并将传输层错误映射为 `GatewayError`
async fn send_builder(builder: reqwest::RequestBuilder) -> Result<reqwest::Response, GatewayError> {
    builder
        .send()
        .await
        .map_err(|e| GatewayError::Transport(format_transport_error(&e)))
}

/// 规整 Base URL（去尾斜杠）并校验非空
fn require_base(channel: &GatewayChannel) -> Result<String, GatewayError> {
    let base = channel
        .base_url
        .as_deref()
        .map(|s| s.trim().trim_end_matches('/'))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| GatewayError::Credential("渠道缺少 Base URL".into()))?;
    Ok(base.to_string())
}

/// 校验 API Key 非空
fn require_key(channel: &GatewayChannel) -> Result<String, GatewayError> {
    channel
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| GatewayError::Credential("渠道缺少 API Key".into()))
}

/// 规整 Responses 请求体以兼容 ChatGPT Codex 后端（与 codex 透传一致）：
/// 字符串 `input` → 消息数组、剔除账号绑定的 `reasoning` 项、补缺省 `instructions`、强制 `stream=true`。
fn normalize_codex_body(body: Bytes) -> Bytes {
    let Ok(mut root) = serde_json::from_slice::<Value>(&body) else {
        return body;
    };
    let Some(obj) = root.as_object_mut() else {
        return body;
    };
    if let Some(Value::String(text)) = obj.get("input").cloned() {
        obj.insert(
            "input".to_string(),
            json!([{
                "role": "user",
                "content": [{"type": "input_text", "text": text}]
            }]),
        );
    }
    // 剔除历史 reasoning 项：其 encrypted_content 与产出它的渠道/账号强绑定。
    // 同一会话跨渠道切换（如先走 openai 兼容渠道、后切 codex）或多账号 failover 时，
    // 把别处的 encrypted_content 透传给 codex 上游会解密校验失败（encrypted content could not be verified）
    if let Some(Value::Array(items)) = obj.get_mut("input") {
        items.retain(|item| item.get("type").and_then(|t| t.as_str()) != Some("reasoning"));
    }
    if !obj.contains_key("instructions") {
        obj.insert(
            "instructions".to_string(),
            json!("You are a helpful assistant."),
        );
    }
    obj.insert("stream".to_string(), json!(true));
    serde_json::to_vec(&root)
        .map(Bytes::from)
        .unwrap_or(body)
}
