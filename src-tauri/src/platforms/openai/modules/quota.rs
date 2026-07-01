use base64::{Engine as _, engine::general_purpose};
use serde_json::Value;

use crate::http_client::create_proxy_client;
use crate::platforms::openai::models::{
    CodexResetConsumeResponse, CodexResetCreditsResponse, QuotaData, WhamUsageResponse,
};

const CHATGPT_WHAM_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CHATGPT_RESET_CREDITS_URL: &str =
    "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits";
const CHATGPT_RESET_CREDITS_CONSUME_URL: &str =
    "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits/consume";

/// Fetch OpenAI quota from ChatGPT wham usage API.
pub async fn fetch_quota(
    access_token: &str,
    chatgpt_account_id: Option<&str>,
) -> Result<QuotaData, String> {
    println!("=== OpenAI fetch_quota ===");
    println!(
        "access_token: {}...",
        &access_token.chars().take(20).collect::<String>()
    );

    let client = create_proxy_client()?;
    let resolved_account_id = resolve_chatgpt_account_id(chatgpt_account_id, access_token);

    let mut request_builder = client
        .get(CHATGPT_WHAM_USAGE_URL)
        .header("authorization", format!("Bearer {}", access_token))
        .header("accept", "application/json");

    if let Some(account_id) = resolved_account_id.as_deref() {
        request_builder = request_builder.header("chatgpt-account-id", account_id);
    }

    let max_retries = 2;
    let mut last_error: Option<String> = None;

    for attempt in 1..=max_retries {
        let request = request_builder
            .try_clone()
            .ok_or_else(|| "failed to clone wham usage request".to_string())?;

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                println!("Response status (attempt {}): {}", attempt, status);

                if status == reqwest::StatusCode::UNAUTHORIZED {
                    println!("Token expired or invalid (401)");
                    return Err("HTTP 401: Token expired or invalid".to_string());
                }

                if status == reqwest::StatusCode::PAYMENT_REQUIRED
                    || status == reqwest::StatusCode::FORBIDDEN
                {
                    println!("Account is forbidden ({})", status);
                    let mut quota = QuotaData::new();
                    quota.is_forbidden = true;
                    return Ok(quota);
                }

                let body = response.text().await.unwrap_or_default();

                if status.is_success() {
                    match serde_json::from_str::<WhamUsageResponse>(&body) {
                        Ok(wham) => {
                            let mut quota = QuotaData::from_wham_usage(&wham);
                            println!("Successfully parsed quota from wham response");
                            println!("  5h used: {:?}%", quota.codex_5h_used_percent);
                            println!("  7d used: {:?}%", quota.codex_7d_used_percent);
                            // 旁路拉取限流重置券（尽力而为，失败不影响配额结果）
                            attach_reset_credits(
                                &mut quota,
                                access_token,
                                resolved_account_id.as_deref(),
                            )
                            .await;
                            return Ok(quota);
                        }
                        Err(e) => {
                            let msg = format!(
                                "Failed to parse wham usage response: {}; body: {}",
                                e,
                                truncate_for_error(&body)
                            );
                            println!("{}", msg);
                            last_error = Some(msg);
                        }
                    }
                } else {
                    let msg = format_http_error(status, &body);
                    println!("{}", msg);
                    last_error = Some(msg);
                }
            }
            Err(e) => {
                println!("Request failed (attempt {}): {}", attempt, e);
                last_error = Some(format!("Request failed: {}", e));
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    let final_error = last_error.unwrap_or_else(|| "Quota fetch failed after retries".to_string());
    println!("Quota fetch failed: {}", final_error);
    Err(final_error)
}

/// 拉取限流重置券列表（`GET /wham/rate-limit-reset-credits`）
pub async fn fetch_reset_credits(
    access_token: &str,
    chatgpt_account_id: Option<&str>,
) -> Result<CodexResetCreditsResponse, String> {
    let client = create_proxy_client()?;
    let mut builder = client
        .get(CHATGPT_RESET_CREDITS_URL)
        .header("authorization", format!("Bearer {}", access_token))
        .header("accept", "application/json");
    if let Some(account_id) = resolve_chatgpt_account_id(chatgpt_account_id, access_token).as_deref()
    {
        builder = builder.header("chatgpt-account-id", account_id);
    }

    let response = builder
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_http_error(status, &body));
    }
    serde_json::from_str::<CodexResetCreditsResponse>(&body).map_err(|e| {
        format!(
            "Failed to parse reset-credits response: {}; body: {}",
            e,
            truncate_for_error(&body)
        )
    })
}

/// 消费一张可用的限流重置券（`POST /wham/rate-limit-reset-credits/consume`）
pub async fn consume_reset_credit(
    access_token: &str,
    chatgpt_account_id: Option<&str>,
) -> Result<CodexResetConsumeResponse, String> {
    let list = fetch_reset_credits(access_token, chatgpt_account_id).await?;
    let credit_id = list
        .credits
        .iter()
        .find(|c| c.status.as_deref() == Some("available"))
        .and_then(|c| c.id.clone())
        .ok_or_else(|| "no available reset credit".to_string())?;

    let client = create_proxy_client()?;
    let mut builder = client
        .post(CHATGPT_RESET_CREDITS_CONSUME_URL)
        .header("authorization", format!("Bearer {}", access_token))
        .header("accept", "application/json")
        .header("content-type", "application/json");
    if let Some(account_id) = resolve_chatgpt_account_id(chatgpt_account_id, access_token).as_deref()
    {
        builder = builder.header("chatgpt-account-id", account_id);
    }

    let payload = serde_json::json!({
        "credit_id": credit_id,
        "redeem_request_id": uuid::Uuid::new_v4().to_string(),
    });

    let response = builder
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_http_error(status, &body));
    }
    serde_json::from_str::<CodexResetConsumeResponse>(&body).map_err(|e| {
        format!(
            "Failed to parse reset-credits consume response: {}; body: {}",
            e,
            truncate_for_error(&body)
        )
    })
}

/// 尽力而为地把重置券可用/总数写入 quota（失败静默）
async fn attach_reset_credits(
    quota: &mut QuotaData,
    access_token: &str,
    chatgpt_account_id: Option<&str>,
) {
    match fetch_reset_credits(access_token, chatgpt_account_id).await {
        Ok(resp) => {
            let total = resp.credits.len() as i64;
            let available = resp.available_count.unwrap_or_else(|| {
                resp.credits
                    .iter()
                    .filter(|c| c.status.as_deref() == Some("available"))
                    .count() as i64
            });
            quota.reset_credits_total = Some(total);
            quota.reset_credits_available = Some(available);
        }
        Err(e) => {
            println!("Fetch reset credits failed (ignored): {}", e);
        }
    }
}

fn resolve_chatgpt_account_id(
    explicit_account_id: Option<&str>,
    access_token: &str,
) -> Option<String> {
    let explicit = explicit_account_id
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    if explicit.is_some() {
        return explicit;
    }

    extract_chatgpt_account_id_from_access_token(access_token)
}

fn extract_chatgpt_account_id_from_access_token(access_token: &str) -> Option<String> {
    let payload = access_token.split('.').nth(1)?;
    let decoded_payload = decode_base64_url(payload)?;
    let claims: Value = serde_json::from_slice(&decoded_payload).ok()?;

    claims
        .pointer("/https://api.openai.com/auth/chatgpt_account_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn decode_base64_url(value: &str) -> Option<Vec<u8>> {
    if let Ok(decoded) = general_purpose::URL_SAFE_NO_PAD.decode(value.as_bytes()) {
        return Some(decoded);
    }

    let mut padded = value.to_string();
    let remainder = padded.len() % 4;
    if remainder != 0 {
        padded.push_str(&"=".repeat(4 - remainder));
    }

    general_purpose::URL_SAFE.decode(padded.as_bytes()).ok()
}

fn format_http_error(status: reqwest::StatusCode, body: &str) -> String {
    let (code, message) = parse_error_code_and_message(body);

    match (code, message) {
        (Some(code), Some(message)) => format!("HTTP {} [{}]: {}", status, code, message),
        (Some(code), None) => format!("HTTP {} [{}]: {}", status, code, truncate_for_error(body)),
        (None, Some(message)) => format!("HTTP {}: {}", status, message),
        (None, None) => format!("HTTP {}: {}", status, truncate_for_error(body)),
    }
}

fn parse_error_code_and_message(body: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (None, None);
    };

    let code = value
        .pointer("/detail/code")
        .and_then(json_value_to_string)
        .or_else(|| value.get("code").and_then(json_value_to_string));

    let message = value
        .pointer("/detail/message")
        .and_then(json_value_to_string)
        .or_else(|| value.get("message").and_then(json_value_to_string))
        .or_else(|| value.get("detail").and_then(json_value_to_string));

    (code, message)
}

fn json_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(v) => Some(v.clone()),
        Value::Number(v) => Some(v.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}

fn truncate_for_error(text: &str) -> String {
    const MAX_LEN: usize = 400;
    let trimmed = text.trim();
    if trimmed.chars().count() <= MAX_LEN {
        return trimmed.to_string();
    }

    let truncated: String = trimmed.chars().take(MAX_LEN).collect();
    format!("{}...", truncated)
}
