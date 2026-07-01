//! 网关 HTTP 路由（复用 `8766`，`/gateway` 前缀）
//!
//! 挂载三种入站协议端点，做全局 Key 鉴权 + enabled 门控，按 model→渠道收集候选并
//! 依优先级 failover，串联 入站→canonical→渠道出站→响应/流式回转 的完整链路，并旁路
//! 采集用量（token / TTFT / 时延，不计价）。Responses↔Codex 同线型直转在此短路透传。

use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use hyper::{Body, Response};
use serde_json::{json, Value};
use warp::http::{HeaderMap, StatusCode};
use warp::{Filter, Rejection, Reply};

use super::affinity::extract_session_key;
use super::canonical::{StreamEvent, Usage};
use super::config::{ChannelKind, GatewayChannel};
use super::executor::GatewayExecutor;
use super::translate::stream_bridge::{inbound_for, outbound_for, SseDecoder, StreamBridge};
use super::translate::{ParseState, Wire};
use super::usage::UsageRecord;
use crate::platforms::openai::codex::upstream::should_retry_status;
use crate::AppState;

mod accumulate;
use accumulate::StreamAcc;

/// 入站请求体大小上限（32 MiB），防御异常大请求撑爆内存
const MAX_BODY_BYTES: u64 = 32 * 1024 * 1024;

/// 网关请求拒绝
#[derive(Debug)]
pub enum GatewayRejection {
    /// 网关未启用（enabled 门控）
    Disabled,
    /// 鉴权失败
    Unauthorized(String),
    /// 请求不合法（缺 model、JSON 解析失败等）
    BadRequest(String),
    /// 无可用渠道
    NoChannel(String),
    /// 全部候选均失败
    Upstream(String),
    /// 内部错误
    Internal(String),
}

impl warp::reject::Reject for GatewayRejection {}

/// 按线型注入 filter
fn with_wire(wire: Wire) -> impl Filter<Extract = (Wire,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || wire)
}

/// 网关路由（POST /gateway/v1/{chat/completions,responses,messages}）
pub fn gateway_routes_from_state(
    state: Arc<AppState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let state_filter = warp::any().map(move || state.clone());

    let common = warp::post()
        .and(state_filter.clone())
        .and(warp::header::headers_cloned())
        .and(warp::body::content_length_limit(MAX_BODY_BYTES))
        .and(warp::body::bytes());

    let chat = warp::path!("gateway" / "v1" / "chat" / "completions")
        .and(with_wire(Wire::Chat))
        .and(common.clone())
        .and_then(handle);

    let responses = warp::path!("gateway" / "v1" / "responses")
        .and(with_wire(Wire::Responses))
        .and(common.clone())
        .and_then(handle);

    let messages = warp::path!("gateway" / "v1" / "messages")
        .and(with_wire(Wire::Anthropic))
        .and(common)
        .and_then(handle);

    chat.or(responses).or(messages)
}

/// 入站协议名（落用量记录）
fn wire_name(wire: Wire) -> &'static str {
    match wire {
        Wire::Chat => "chat",
        Wire::Responses => "responses",
        Wire::Anthropic => "anthropic",
    }
}

/// 校验全局 API Key（Authorization: Bearer 或 x-api-key）
fn authorize(headers: &HeaderMap, expected: &str) -> Result<(), GatewayRejection> {
    if expected.trim().is_empty() {
        return Err(GatewayRejection::Unauthorized(
            "Gateway API key not configured".into(),
        ));
    }
    let mut provided: Vec<&str> = Vec::new();
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let t = auth.trim();
        provided.push(t.strip_prefix("Bearer ").or_else(|| t.strip_prefix("bearer ")).unwrap_or(t));
    }
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        provided.push(key.trim());
    }
    if provided.iter().any(|k| *k == expected) {
        Ok(())
    } else {
        Err(GatewayRejection::Unauthorized("Invalid API key".into()))
    }
}

/// 包装为 warp 拒绝
fn reject(r: GatewayRejection) -> Rejection {
    warp::reject::custom(r)
}

/// 渠道类型名（落用量记录）
fn kind_name(kind: ChannelKind) -> &'static str {
    match kind {
        ChannelKind::CodexOauth => "codex_oauth",
        ChannelKind::OpenaiCompat => "openai_compat",
        ChannelKind::Anthropic => "anthropic",
    }
}

/// 将请求体中的 `model` 字段改写为上游真实模型 id（解析失败则原样返回）
fn rewrite_body_model(body: &Bytes, upstream_model: &str) -> Bytes {
    let Ok(mut root) = serde_json::from_slice::<Value>(body) else {
        return body.clone();
    };
    let Some(obj) = root.as_object_mut() else {
        return body.clone();
    };
    obj.insert("model".to_string(), Value::String(upstream_model.to_string()));
    serde_json::to_vec(&root)
        .map(Bytes::from)
        .unwrap_or_else(|_| body.clone())
}

/// 网关请求总入口：门控 → 鉴权 → 解析 → 选候选 → failover 执行 → 响应/流式回转
async fn handle(
    wire: Wire,
    state: Arc<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Box<dyn Reply>, Rejection> {
    let started = Instant::now();

    // 解析请求体，取 model 与客户端 stream 意图
    let value: Value = serde_json::from_slice(&body).map_err(|e| {
        reject(GatewayRejection::BadRequest(format!(
            "请求体 JSON 解析失败: {}",
            e
        )))
    })?;
    let model = value
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err(reject(GatewayRejection::BadRequest(
            "请求缺少 model 字段".into(),
        )));
    }
    let client_stream = value.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    // 门控 + 鉴权 + 候选收集（快照后立即释放锁，避免跨 await 持锁）
    let mut candidates = {
        let cfg = state
            .gateway_config
            .lock()
            .map_err(|_| reject(GatewayRejection::Internal("网关配置锁中毒".into())))?;
        if !cfg.enabled {
            return Err(reject(GatewayRejection::Disabled));
        }
        authorize(&headers, &cfg.api_key).map_err(reject)?;
        cfg.candidates_for(&model)
    };
    if candidates.is_empty() {
        return Err(reject(GatewayRejection::NoChannel(format!(
            "无可用渠道支持模型 {}",
            model
        ))));
    }

    // 会话粘性：同优先级组内把「上次成功渠道」提到最前（候选已按优先级升序，
    // 稳定排序保持其余相对顺序，故粘性不会跨越优先级分组）
    let session_key = extract_session_key(&headers, &value);
    if let Some(sticky) = session_key.as_ref().and_then(|k| state.gateway_affinity.get(k)) {
        candidates.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| (a.id != sticky).cmp(&(b.id != sticky)))
        });
    }

    // 入站协议 → canonical 请求
    let canonical = inbound_for(wire)
        .parse_request(&value)
        .map_err(|e| reject(GatewayRejection::BadRequest(format!("入站请求解析失败: {}", e))))?;

    let client = state
        .gateway_http_client()
        .map_err(|e| reject(GatewayRejection::Internal(e)))?;
    let executor = GatewayExecutor::new(state.app_handle.clone(), client);

    // 依优先级 failover
    let mut last_err = String::new();
    let mut last_upstream: Option<(StatusCode, Bytes, UsageRecord)> = None;
    for channel in &candidates {
        let channel_wire = channel.wire();
        let same_wire = channel_wire == wire;

        // 别名改写：客户端别名 → 该渠道的上游真实模型 id（未设别名则原样）
        let upstream_model = channel.upstream_model(&model);
        let rewrite = upstream_model != model;

        // 同线型直转：透传原始 body（Codex 体在执行层规整）；跨协议走 canonical
        let send_result = if same_wire {
            let out_body = if rewrite {
                rewrite_body_model(&body, &upstream_model)
            } else {
                body.clone()
            };
            executor.send(channel, out_body).await
        } else if rewrite {
            let mut req = canonical.clone();
            req.model = upstream_model.clone();
            executor.send_canonical(channel, &req).await
        } else {
            executor.send_canonical(channel, &canonical).await
        };

        let resp = match send_result {
            Ok(r) => r,
            Err(e) => {
                last_err = e.message();
                continue;
            }
        };

        let status =
            StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

        // 非成功且可重试 → failover 到下一候选
        if !status.is_success() && should_retry_status(status) {
            last_err = format!("上游返回 {}", status.as_u16());
            let bytes = resp.bytes().await.unwrap_or_default();
            if let Some(msg) = extract_error_message(&bytes) {
                last_err = msg;
            }
            // 记住最后一次上游真实错误响应，候选耗尽时原样回传（保留状态码与错误体）
            let mut rec = new_record(&model, channel, wire_name(wire), client_stream);
            rec.status = "error".to_string();
            rec.status_code = status.as_u16();
            last_upstream = Some((status, bytes, rec));
            continue;
        }

        // 上游成功响应：写回会话→渠道，后续同会话请求优先粘到此渠道
        if status.is_success() {
            if let Some(key) = &session_key {
                state.gateway_affinity.set(key.clone(), channel.id.clone());
            }
        }

        let upstream_stream =
            is_event_stream(&resp) || channel.kind == ChannelKind::CodexOauth;
        let mut rec = new_record(&model, channel, wire_name(wire), client_stream);
        rec.status_code = status.as_u16();

        let reply = if !status.is_success() {
            error_reply(state.clone(), resp, status, rec, started).await
        } else if upstream_stream && client_stream {
            build_streaming_reply(
                state.clone(),
                resp,
                status,
                wire,
                channel_wire,
                !same_wire,
                model.clone(),
                rec,
                started,
            )
        } else if upstream_stream {
            destream_reply(
                state.clone(),
                resp,
                wire,
                channel_wire,
                same_wire,
                model.clone(),
                rec,
                started,
            )
            .await
        } else {
            buffered_reply(
                state.clone(),
                resp,
                status,
                wire,
                channel_wire,
                same_wire,
                rec,
                started,
            )
            .await
        };

        return reply.map_err(|e| reject(GatewayRejection::Internal(e)));
    }

    // 候选耗尽：若曾拿到上游真实错误响应，原样回传（保留状态码与错误体，便于客户端排障）
    if let Some((status, bytes, mut rec)) = last_upstream {
        rec.duration_ms = started.elapsed().as_millis();
        rec.error = extract_error_message(&bytes);
        state.gateway_usage.record(rec);
        return buffered_response(status, "application/json", bytes.to_vec())
            .map_err(|e| reject(GatewayRejection::Internal(e)));
    }

    Err(reject(GatewayRejection::Upstream(if last_err.is_empty() {
        "全部候选渠道均失败".into()
    } else {
        last_err
    })))
}

/// 上游响应是否为 SSE 事件流
fn is_event_stream(resp: &reqwest::Response) -> bool {
    resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

/// 新建一条用量记录模板（token 字段后续补齐）
fn new_record(model: &str, channel: &GatewayChannel, inbound: &str, stream: bool) -> UsageRecord {
    UsageRecord {
        request_id: format!("gw-{}", uuid::Uuid::new_v4()),
        created_at: chrono::Utc::now().timestamp_millis(),
        model: model.to_string(),
        channel_id: channel.id.clone(),
        channel_name: channel.name.clone(),
        kind: kind_name(channel.kind).to_string(),
        inbound: inbound.to_string(),
        status: "success".to_string(),
        status_code: 200,
        stream,
        duration_ms: 0,
        ttft_ms: None,
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        cached_tokens: 0,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        error: None,
    }
}

/// 从响应体提取错误消息（兼容 `error.message` 与扁平 `message`）
fn extract_error_message(bytes: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(bytes).ok()?;
    v.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .or_else(|| v.get("message").and_then(|m| m.as_str()))
        .map(|s| s.to_string())
}

/// 构造缓冲（非流式）响应
fn buffered_response(
    status: StatusCode,
    content_type: &str,
    body: Vec<u8>,
) -> Result<Box<dyn Reply>, String> {
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(Body::from(body))
        .map(|r| Box::new(r) as Box<dyn Reply>)
        .map_err(|e| format!("构造响应失败: {}", e))
}

/// 上游错误：原样回传错误体并落 error 用量记录
async fn error_reply(
    state: Arc<AppState>,
    resp: reqwest::Response,
    status: StatusCode,
    mut rec: UsageRecord,
    started: Instant,
) -> Result<Box<dyn Reply>, String> {
    let bytes = resp.bytes().await.unwrap_or_default();
    rec.status = "error".to_string();
    rec.status_code = status.as_u16();
    rec.duration_ms = started.elapsed().as_millis();
    rec.error = extract_error_message(&bytes);
    state.gateway_usage.record(rec);
    buffered_response(status, "application/json", bytes.to_vec())
}

/// 非流式上游：同线型原样回传，跨协议经 canonical 回渲；旁路解析 usage
async fn buffered_reply(
    state: Arc<AppState>,
    resp: reqwest::Response,
    status: StatusCode,
    wire: Wire,
    channel_wire: Wire,
    same_wire: bool,
    mut rec: UsageRecord,
    started: Instant,
) -> Result<Box<dyn Reply>, String> {
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取上游响应失败: {}", e))?;
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let canonical = outbound_for(channel_wire).parse_response(&value).ok();
    let usage = canonical
        .as_ref()
        .and_then(|c| c.usage.clone())
        .unwrap_or_default();

    let body_bytes = if same_wire {
        bytes.to_vec()
    } else if let Some(c) = &canonical {
        let v = inbound_for(wire).render_response(c);
        serde_json::to_vec(&v).unwrap_or_else(|_| bytes.to_vec())
    } else {
        bytes.to_vec()
    };

    rec.stream = false;
    rec.duration_ms = started.elapsed().as_millis();
    let rec = rec.with_usage(&usage);
    state.gateway_usage.record(rec);
    buffered_response(status, "application/json", body_bytes)
}

/// 上游流式但客户端要非流式：聚合为单条响应回传，旁路解析 usage
async fn destream_reply(
    state: Arc<AppState>,
    resp: reqwest::Response,
    wire: Wire,
    channel_wire: Wire,
    same_wire: bool,
    model: String,
    mut rec: UsageRecord,
    started: Instant,
) -> Result<Box<dyn Reply>, String> {
    let mut stream = resp.bytes_stream();
    let mut decoder = SseDecoder::default();
    let out_tr = outbound_for(channel_wire);
    let mut parse = ParseState {
        model: model.clone(),
        ..Default::default()
    };
    let mut acc = StreamAcc::default();
    let mut raw = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                raw.push_str(&String::from_utf8_lossy(&bytes));
                for (event, data) in decoder.push(&bytes) {
                    for ev in out_tr.parse_stream(event.as_deref(), &data, &mut parse) {
                        acc.apply(&ev);
                    }
                }
            }
            Err(e) => {
                rec.error = Some(format!("上游流读取失败: {}", e));
                break;
            }
        }
    }

    let usage = {
        let u = acc.usage();
        if u != Usage::default() {
            u
        } else {
            parse.usage.clone()
        }
    };
    let error = acc.error();
    let canonical = acc.into_response(&model);

    // Responses↔Codex 直转：尽量保全专有字段（提取 response.completed 原始对象）
    let body_value = if let Some(message) = &error {
        json!({"error": {"message": message, "type": "upstream_error"}})
    } else if same_wire && wire == Wire::Responses {
        extract_completed_response(&raw)
            .unwrap_or_else(|| inbound_for(wire).render_response(&canonical))
    } else {
        inbound_for(wire).render_response(&canonical)
    };

    rec.stream = false;
    rec.duration_ms = started.elapsed().as_millis();
    if error.is_some() {
        rec.error = error;
    }
    if rec.error.is_some() {
        rec.status = "error".to_string();
    }
    let response_status = if rec.error.is_some() {
        StatusCode::BAD_GATEWAY
    } else {
        StatusCode::OK
    };
    let rec = rec.with_usage(&usage);
    state.gateway_usage.record(rec);

    let body_bytes = serde_json::to_vec(&body_value).unwrap_or_default();
    buffered_response(response_status, "application/json", body_bytes)
}

/// 流式回转：跨协议经 `StreamBridge` 转换，同线型透传；旁路采集 TTFT/usage
fn build_streaming_reply(
    state: Arc<AppState>,
    resp: reqwest::Response,
    status: StatusCode,
    wire: Wire,
    channel_wire: Wire,
    transform: bool,
    model: String,
    mut rec: UsageRecord,
    started: Instant,
) -> Result<Box<dyn Reply>, String> {
    let builder = Response::builder()
        .status(status)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache");

    let mut upstream = resp.bytes_stream();
    let (tx, rx) = futures::channel::mpsc::channel::<Result<Bytes, std::io::Error>>(16);

    tokio::spawn(async move {
        let mut maybe_tx = Some(tx);
        let mut ttft: Option<u128> = None;
        let mut usage = Usage::default();

        // 跨协议：桥接转换；同线型：透传 + 旁路解析 usage
        let mut bridge = if transform {
            Some(StreamBridge::new(channel_wire, wire, &model))
        } else {
            None
        };
        let mut decoder = SseDecoder::default();
        let out_tr = outbound_for(channel_wire);
        let mut parse = ParseState {
            model: model.clone(),
            ..Default::default()
        };

        while let Some(chunk) = upstream.next().await {
            match chunk {
                Ok(bytes) => {
                    let forward: Vec<u8> = if let Some(b) = bridge.as_mut() {
                        let out = b.push(&bytes);
                        usage = b.usage();
                        if rec.error.is_none() {
                            rec.error = b.error();
                        }
                        out
                    } else {
                        for (event, data) in decoder.push(&bytes) {
                            for ev in out_tr.parse_stream(event.as_deref(), &data, &mut parse) {
                                match ev {
                                    StreamEvent::Usage(u) => usage = u,
                                    StreamEvent::Error { message } => {
                                        if rec.error.is_none() {
                                            rec.error = Some(message);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        bytes.to_vec()
                    };
                    if !forward.is_empty() {
                        if ttft.is_none() {
                            ttft = Some(started.elapsed().as_millis());
                        }
                        if let Some(s) = maybe_tx.as_mut() {
                            if s.send(Ok(Bytes::from(forward))).await.is_err() {
                                maybe_tx = None;
                            }
                        }
                    }
                }
                Err(e) => {
                    rec.error = Some(format!("上游流读取失败: {}", e));
                    if let Some(s) = maybe_tx.as_mut() {
                        let _ = s
                            .send(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("{}", e),
                            )))
                            .await;
                    }
                    break;
                }
            }
        }

        // 桥接收尾：补发终止分片
        if let Some(b) = bridge.as_mut() {
            let tail = b.finish();
            usage = b.usage();
            if rec.error.is_none() {
                rec.error = b.error();
            }
            if !tail.is_empty() {
                if let Some(s) = maybe_tx.as_mut() {
                    let _ = s.send(Ok(Bytes::from(tail))).await;
                }
            }
        }
        drop(maybe_tx);

        rec.stream = true;
        rec.ttft_ms = ttft;
        rec.duration_ms = started.elapsed().as_millis();
        if rec.error.is_some() {
            rec.status = "error".to_string();
        }
        let rec = rec.with_usage(&usage);
        state.gateway_usage.record(rec);
    });

    builder
        .body(Body::wrap_stream(rx))
        .map(|r| Box::new(r) as Box<dyn Reply>)
        .map_err(|e| format!("构造流式响应失败: {}", e))
}

/// 从 SSE 文本提取 `response.completed` 事件的 response 对象（Responses↔Codex 直转用）
fn extract_completed_response(sse_text: &str) -> Option<Value> {
    for block in sse_text.split("\n\n") {
        let mut event_type = None;
        let mut data_lines = Vec::new();
        for line in block.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(rest) = line.strip_prefix("event:") {
                event_type = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
            }
        }
        if event_type.as_deref() == Some("response.completed") {
            let data = data_lines.join("\n");
            if let Ok(v) = serde_json::from_str::<Value>(&data) {
                if let Some(resp) = v.get("response") {
                    return Some(resp.clone());
                }
            }
        }
    }
    None
}
