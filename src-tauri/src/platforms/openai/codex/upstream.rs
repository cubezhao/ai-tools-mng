//! Codex 上游请求构造工具
//!
//! 从 `CodexExecutor` 抽出的与号池无关的纯工具函数：路径映射、URL 构造、
//! 转发头处理与重试判定。供 Codex 透传执行器与网关 OAuth 渠道共用。

use reqwest::{RequestBuilder, StatusCode};
use warp::http::HeaderMap;

use crate::platforms::openai::codex::models::CodexError;

/// Codex 上游源站
pub const CODEX_UPSTREAM_ORIGIN: &str = "https://chatgpt.com";

/// 将本地 `/v1/*` 路径映射到 Codex 后端 `/backend-api/codex/*`
pub fn map_upstream_path(path: &str) -> Result<String, CodexError> {
    if path == "/v1" {
        return Ok("/backend-api/codex".to_string());
    }

    if let Some(tail) = path.strip_prefix("/v1/") {
        return Ok(format!("/backend-api/codex/{}", tail));
    }

    if path == "/backend-api/codex" || path.starts_with("/backend-api/codex/") {
        return Ok(path.to_string());
    }

    Err(CodexError::InvalidRequest(format!(
        "Unsupported Codex path: {}",
        path
    )))
}

/// 拼接上游 URL（保留原始 query）
pub fn build_upstream_url(origin: &str, path: &str, raw_query: Option<&str>) -> String {
    let mut url = format!("{}{}", origin, path);
    if let Some(query) = raw_query.map(str::trim).filter(|q| !q.is_empty()) {
        url.push('?');
        url.push_str(query);
    }
    url
}

/// 应用转发头：剔除逐跳/鉴权头，注入 Codex 鉴权与默认头
///
/// 与号池解耦——直接接收 `access_token` 与 `chatgpt_account_id`。
pub fn apply_forward_headers(
    mut builder: RequestBuilder,
    headers: &HeaderMap,
    access_token: &str,
    chatgpt_account_id: &str,
) -> RequestBuilder {
    let mut has_user_agent = false;
    let mut has_openai_beta = false;
    let mut has_originator = false;

    for (name, value) in headers.iter() {
        if should_strip_request_header(name.as_str()) {
            continue;
        }

        if name.as_str().eq_ignore_ascii_case("user-agent") {
            has_user_agent = true;
        }
        if name.as_str().eq_ignore_ascii_case("openai-beta") {
            has_openai_beta = true;
        }
        if name.as_str().eq_ignore_ascii_case("originator") {
            has_originator = true;
        }

        builder = builder.header(name, value.clone());
    }

    builder = builder
        .header("Authorization", format!("Bearer {}", access_token))
        .header("chatgpt-account-id", chatgpt_account_id.to_string());

    if !has_user_agent {
        builder = builder.header("User-Agent", "codex_cli_rs/0.98.0");
    }
    if !has_openai_beta {
        builder = builder.header("OpenAI-Beta", "responses=experimental");
    }
    if !has_originator {
        builder = builder.header("originator", "codex_cli_rs");
    }

    builder
}

/// 是否为需要从转发请求中剔除的头
pub fn should_strip_request_header(header_name: &str) -> bool {
    matches!(
        header_name.to_ascii_lowercase().as_str(),
        "host"
            | "content-length"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "authorization"
            | "x-api-key"
            | "chatgpt-account-id"
    )
}

/// 上游状态码是否应触发切号重试
pub fn should_retry_status(status: StatusCode) -> bool {
    matches!(
        status.as_u16(),
        401 | 403 | 408 | 429 | 500 | 502 | 503 | 504
    )
}

/// 传输层错误是否可重试（超时/连接失败）
pub fn is_retryable_transport_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect()
}

/// 格式化传输层错误信息
pub fn format_transport_error(err: &reqwest::Error) -> String {
    if err.is_timeout() || err.is_connect() {
        return format!(
            "Request failed: {}. Upstream connection timed out; check proxy settings and network reachability.",
            err
        );
    }

    format!("Request failed: {}", err)
}
