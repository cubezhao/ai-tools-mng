//! 网关渠道连通性：拉取远端模型列表 + 测试渠道可用性。
//!
//! - `gateway_fetch_channel_models`：调用上游 /models 接口，返回模型 ID 列表
//! - `gateway_test_channel`：发送一次最小聊天请求，返回成功状态、HTTP 码与耗时
//!
//! 复用 `core::http_client::create_http_client()` 自动套用代理配置。

use serde::Serialize;
use serde_json::json;
use std::time::Instant;

fn normalize_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut out: String = t.chars().take(max).collect();
    out.push('…');
    out
}

/// 从上游错误响应体提取可读消息，兼容 OpenAI 的 `error.message` 与扁平 `message`。
fn extract_error(text: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(m) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| v.get("message").and_then(|m| m.as_str()))
        {
            return m.to_string();
        }
    }
    truncate(text, 200)
}

#[derive(Serialize)]
pub struct TestResult {
    success: bool,
    status: u16,
    latency_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tauri::command]
pub async fn gateway_fetch_channel_models(
    kind: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    let base = normalize_base(&base_url);
    if base.is_empty() {
        return Err("Base URL 不能为空".into());
    }
    let client = crate::http_client::create_http_client()?;

    let req = if kind == "anthropic" {
        client
            .get(format!("{}/v1/models", base))
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
    } else {
        client.get(format!("{}/models", base)).bearer_auth(&api_key)
    };

    let resp = req.send().await.map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), extract_error(&text)));
    }

    let val: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;
    // openai / anthropic 均为 { data: [{ id }] }；部分兼容端点直接返回数组
    let arr = val
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| val.as_array())
        .cloned()
        .unwrap_or_default();

    let mut ids: Vec<String> = arr
        .iter()
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[tauri::command]
pub async fn gateway_test_channel(
    kind: String,
    base_url: String,
    api_key: String,
    wire: Option<String>,
    model: String,
) -> Result<TestResult, String> {
    let base = normalize_base(&base_url);
    if base.is_empty() {
        return Err("Base URL 不能为空".into());
    }
    if model.trim().is_empty() {
        return Err("请先指定测试模型".into());
    }
    let client = crate::http_client::create_http_client()?;
    let started = Instant::now();

    let req = if kind == "anthropic" {
        let body = json!({
            "model": model,
            "max_tokens": 16,
            "messages": [{ "role": "user", "content": "Hi" }]
        });
        client
            .post(format!("{}/v1/messages", base))
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
    } else if wire.as_deref() == Some("responses") {
        let body = json!({
            "model": model,
            "input": [{ "role": "user", "content": "Hi" }]
        });
        client
            .post(format!("{}/responses", base))
            .bearer_auth(&api_key)
            .json(&body)
    } else {
        let body = json!({
            "model": model,
            "max_tokens": 16,
            "messages": [{ "role": "user", "content": "Hi" }]
        });
        client
            .post(format!("{}/chat/completions", base))
            .bearer_auth(&api_key)
            .json(&body)
    };

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Ok(TestResult {
                success: false,
                status: 0,
                latency_ms: started.elapsed().as_millis(),
                error: Some(format!("请求失败: {}", e)),
            });
        }
    };
    let status = resp.status();
    let latency_ms = started.elapsed().as_millis();
    let text = resp.text().await.unwrap_or_default();

    Ok(TestResult {
        success: status.is_success(),
        status: status.as_u16(),
        latency_ms,
        error: if status.is_success() {
            None
        } else {
            Some(extract_error(&text))
        },
    })
}
