//! Cursor 认证模块

use crate::http_client::create_proxy_client;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

/// 将 null 反序列化为 0.0
fn null_as_zero<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    Option::<f64>::deserialize(deserializer).map(|v| v.unwrap_or(0.0))
}

/// Cursor 用户信息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorUserInfo {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Usage Summary 响应（cursor.com/api/usage-summary 返回结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    #[serde(default)]
    pub membership_type: Option<String>,
    #[serde(default)]
    pub individual_usage: Option<IndividualUsage>,
    #[serde(default)]
    pub billing_cycle_start: Option<String>,
    #[serde(default)]
    pub billing_cycle_end: Option<String>,
}

/// 个人用量
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndividualUsage {
    #[serde(default)]
    pub plan: Option<PlanUsage>,
    #[serde(rename = "onDemand", default)]
    pub on_demand: Option<OnDemandUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_cycle_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_cycle_end: Option<String>,
    /// Grok Bot 周额度（与 Auto / API 月度池分开）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grok_bot: Option<GrokBotUsage>,
}

/// Grok Bot 周额度
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GrokBotUsage {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_end: Option<String>,
}

impl GrokBotUsage {
    pub fn has_meter(&self) -> bool {
        self.percent_used.is_some()
            || (self.used.is_some() && self.limit.is_some())
            || (self.remaining.is_some() && self.limit.is_some())
    }
}

/// Plan 用量
///
/// 美分字段（used / limit / remaining）来自实时账单流水，是官方仪表盘文案
/// “You've used X% of your included usage” 的口径；
/// percent 字段是另一套缓存聚合指标，官方确认不等于 used / limit，且会滞后。
/// 两类字段都可能缺失（团队/企业账号），缺失必须保持 None，
/// 否则会被当成 0 而在界面上显示成“额度全满”。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanUsage {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub breakdown: Option<PlanBreakdown>,
    #[serde(
        rename = "autoPercentUsed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_percent_used: Option<f64>,
    #[serde(
        rename = "apiPercentUsed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub api_percent_used: Option<f64>,
    #[serde(
        rename = "totalPercentUsed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub total_percent_used: Option<f64>,
}

/// Plan 用量明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBreakdown {
    #[serde(default, deserialize_with = "null_as_zero")]
    pub included: f64,
    #[serde(default, deserialize_with = "null_as_zero")]
    pub bonus: f64,
    #[serde(default, deserialize_with = "null_as_zero")]
    pub total: f64,
}

/// On-Demand 用量（limit / remaining 为 null 表示未设上限）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDemandUsage {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, deserialize_with = "null_as_zero")]
    pub used: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
}

/// 聚合用量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedUsageData {
    pub aggregations: Vec<ModelUsage>,
    pub total_input_tokens: String,
    pub total_output_tokens: String,
    pub total_cache_write_tokens: String,
    pub total_cache_read_tokens: String,
    pub total_cost_cents: f64,
}

/// 模型用量
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model_intent: String,
    pub input_tokens: String,
    pub output_tokens: String,
    pub cache_write_tokens: String,
    pub cache_read_tokens: String,
    pub total_cents: f64,
}

/// 过滤的使用事件数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredUsageEventsData {
    #[serde(rename = "totalUsageEventsCount")]
    pub total_usage_events_count: i32,
    #[serde(rename = "usageEventsDisplay")]
    pub usage_events_display: Vec<UsageEventDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEventDisplay {
    pub timestamp: String,
    pub model: String,
    pub kind: String,
    #[serde(rename = "requestsCosts", default)]
    pub requests_costs: Option<f64>,
    #[serde(rename = "usageBasedCosts")]
    pub usage_based_costs: String,
    #[serde(rename = "isTokenBasedCall")]
    pub is_token_based_call: bool,
    #[serde(rename = "tokenUsage", default)]
    pub token_usage: Option<TokenUsageDetail>,
    #[serde(rename = "owningUser")]
    pub owning_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDetail {
    #[serde(rename = "inputTokens")]
    pub input_tokens: Option<i32>,
    #[serde(rename = "outputTokens")]
    pub output_tokens: Option<i32>,
    #[serde(rename = "cacheWriteTokens")]
    pub cache_write_tokens: Option<i32>,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: Option<i32>,
    #[serde(rename = "totalCents")]
    pub total_cents: Option<f64>,
}

/// /api/auth/me 接口响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthMeResponse {
    pub email: String,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub picture: Option<String>,
}

/// 使用 session_token 获取用户信息（调用 cursor.com/api/auth/me）
pub async fn get_user_info(session_token: &str) -> Result<CursorUserInfo, String> {
    let client = create_proxy_client()?;

    let response = client
        .get("https://cursor.com/api/auth/me")
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", session_token),
        )
        .header("Accept", "*/*")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("User info request failed: {}", e))?;

    let status_code = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status_code != 200 {
        return Err(format!(
            "Get user info failed (HTTP {}): {}",
            status_code, body
        ));
    }

    // 解析响应 JSON
    let auth_me: AuthMeResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse auth/me response: {}", e))?;

    // 提取 user_id：优先使用 sub 字段，否则使用 id
    let user_id = auth_me.sub.unwrap_or_else(|| {
        auth_me
            .id
            .map(|i| i.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    Ok(CursorUserInfo {
        id: user_id,
        email: auth_me.email,
        name: auth_me.name,
    })
}

/// 获取用量摘要（使用 session_token + Cookie 认证）
///
/// `access_token` 可选，保留兼容；Grok Bot 周额度走 session Cookie 的 Sand 接口。
pub async fn get_usage_summary(
    session_token: &str,
    _access_token: Option<&str>,
) -> Result<UsageSummary, String> {
    let client = create_proxy_client()?;

    let response = client
        .get("https://cursor.com/api/usage-summary")
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", session_token),
        )
        .header("Accept", "*/*")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Usage summary request failed: {}", e))?;

    let status = response.status();
    let usage_ok = status.is_success();

    let mut summary = if usage_ok {
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let raw: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse usage summary json: {}", e))?;
        let mut parsed = serde_json::from_value::<UsageSummary>(raw.clone())
            .map_err(|e| format!("Failed to parse usage summary: {}", e))?;

        if !super::grok_bot::has_grok_bot_meter(&parsed) {
            if let Some(bot) = super::grok_bot::extract_from_value(&raw) {
                super::grok_bot::attach_grok_bot(&mut parsed, bot);
            }
        }
        parsed
    } else {
        UsageSummary {
            membership_type: Some("free".to_string()),
            individual_usage: None,
            billing_cycle_start: None,
            billing_cycle_end: None,
        }
    };

    // usage-summary 不含 Grok Bot；仪表盘走 get-sand-usage-status。
    // 不按套餐门控：Pro 账号若链接了 SuperGrok 也会有周额度。
    // usage-summary 失败时 session 多半也过期，不必再打 Sand。
    if usage_ok && !super::grok_bot::has_grok_bot_meter(&summary) {
        if let Some(bot) = super::grok_bot::fetch_grok_bot_usage(session_token).await {
            super::grok_bot::attach_grok_bot(&mut summary, bot);
        }
    }

    Ok(summary)
}

/// Stripe Profile 响应结构（api2.cursor.sh/auth/full_stripe_profile）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StripeProfileResponse {
    #[serde(default)]
    pub membership_type: Option<String>,
    #[serde(default)]
    pub individual_membership_type: Option<String>,
    #[serde(default)]
    pub subscription_status: Option<String>,
    #[serde(default)]
    pub days_remaining_on_trial: Option<i32>,
    #[serde(default)]
    pub payment_id: Option<String>,
}

/// 使用 access_token 获取 Stripe 订阅信息（Bearer 认证，不需要 session）
pub async fn get_stripe_profile(access_token: &str) -> Result<StripeProfileResponse, String> {
    let client = create_proxy_client()?;

    let response = client
        .get("https://api2.cursor.sh/auth/full_stripe_profile")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "*/*")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Stripe profile request failed: {}", e))?;

    let status_code = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status_code != 200 {
        return Err(format!(
            "Get stripe profile failed (HTTP {}): {}",
            status_code, body
        ));
    }

    serde_json::from_str::<StripeProfileResponse>(&body)
        .map_err(|e| format!("Failed to parse stripe profile: {}", e))
}

/// 获取聚合用量数据
pub async fn get_aggregated_usage_data(
    workos_session_token: &str,
    start_date: u64,
    end_date: u64,
    team_id: i32,
) -> Result<Option<AggregatedUsageData>, String> {
    let client = create_proxy_client()?;

    let request_body = serde_json::json!({
        "startDate": start_date,
        "endDate": end_date,
        "teamId": team_id
    });

    let response = client
        .post("https://cursor.com/api/dashboard/get-aggregated-usage-events")
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", workos_session_token),
        )
        .header("Accept", "*/*")
        .header("Content-Type", "application/json")
        .header("Origin", "https://cursor.com")
        .header("Referer", "https://cursor.com/cn/dashboard")
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(40))
        .send()
        .await
        .map_err(|e| format!("Aggregated usage request failed: {}", e))?;

    if response.status().is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&body) {
            let mut aggregations = Vec::new();

            if let Some(agg_array) = json_data.get("aggregations").and_then(|v| v.as_array()) {
                for agg in agg_array {
                    if let Some(model_intent) = agg.get("modelIntent").and_then(|v| v.as_str()) {
                        let model_usage = ModelUsage {
                            model_intent: model_intent.to_string(),
                            input_tokens: agg
                                .get("inputTokens")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            output_tokens: agg
                                .get("outputTokens")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            cache_write_tokens: agg
                                .get("cacheWriteTokens")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            cache_read_tokens: agg
                                .get("cacheReadTokens")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            total_cents: agg
                                .get("totalCents")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0),
                        };
                        aggregations.push(model_usage);
                    }
                }
            }

            let aggregated_usage = AggregatedUsageData {
                aggregations,
                total_input_tokens: json_data
                    .get("totalInputTokens")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                total_output_tokens: json_data
                    .get("totalOutputTokens")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                total_cache_write_tokens: json_data
                    .get("totalCacheWriteTokens")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                total_cache_read_tokens: json_data
                    .get("totalCacheReadTokens")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                total_cost_cents: json_data
                    .get("totalCostCents")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
            };

            return Ok(Some(aggregated_usage));
        }
    }

    Ok(None)
}

/// 获取过滤的使用事件
pub async fn get_filtered_usage_events(
    workos_session_token: &str,
    team_id: i32,
    start_date: Option<&str>,
    end_date: Option<&str>,
    page: Option<i32>,
    page_size: Option<i32>,
) -> Result<Option<FilteredUsageEventsData>, String> {
    let client = create_proxy_client()?;

    let mut request_body = serde_json::json!({
        "teamId": team_id
    });
    if let Some(sd) = start_date {
        request_body["startDate"] = serde_json::Value::String(sd.to_string());
    }
    if let Some(ed) = end_date {
        request_body["endDate"] = serde_json::Value::String(ed.to_string());
    }
    if let Some(p) = page {
        request_body["page"] = serde_json::Value::Number(p.into());
    }
    if let Some(ps) = page_size {
        request_body["pageSize"] = serde_json::Value::Number(ps.into());
    }

    let response = client
        .post("https://cursor.com/api/dashboard/get-filtered-usage-events")
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", workos_session_token),
        )
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json")
        .header("Origin", "https://cursor.com")
        .header("Referer", "https://cursor.com/dashboard")
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(40))
        .send()
        .await
        .map_err(|e| format!("Filtered usage events request failed: {}", e))?;

    if response.status().is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if let Ok(events_data) = serde_json::from_str::<FilteredUsageEventsData>(&body) {
            return Ok(Some(events_data));
        }
    }

    Ok(None)
}

// ============================================================================
// PKCE 授权流程：从 WorkosCursorSessionToken 获取 AccessToken
// ============================================================================

/// AccessToken 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken", default)]
    pub refresh_token: Option<String>,
    #[serde(rename = "authId", default)]
    pub auth_id: Option<String>,
}

/// 生成 PKCE verifier (32字节随机数，Base64URL编码)
fn generate_pkce_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 生成 PKCE challenge (SHA256(verifier) 的 Base64URL编码)
fn generate_pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

/// 触发授权登录 - 用 session token 自动确认授权
async fn trigger_authorization_login(
    uuid: &str,
    challenge: &str,
    session_token: &str,
) -> Result<(), String> {
    let client = create_proxy_client()?;

    let payload = serde_json::json!({
        "challenge": challenge,
        "uuid": uuid,
    });

    let response = client
        .post("https://cursor.com/api/auth/loginDeepCallbackControl")
        .header(
            "Cookie",
            format!("WorkosCursorSessionToken={}", session_token),
        )
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Authorization request failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Authorization failed: {}", error_text))
    }
}

/// 轮询获取 AccessToken
async fn poll_for_access_token(
    uuid: &str,
    verifier: &str,
) -> Result<Option<AccessTokenResponse>, String> {
    let client = create_proxy_client()?;

    let url = format!(
        "https://api2.cursor.sh/auth/poll?uuid={}&verifier={}",
        uuid, verifier
    );

    let response = client
        .get(&url)
        .header("Accept", "*/*")
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("Poll request failed: {}", e))?;

    if response.status().is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read poll response: {}", e))?;

        // 尝试解析为 AccessTokenResponse
        if let Ok(token_response) = serde_json::from_str::<AccessTokenResponse>(&body) {
            return Ok(Some(token_response));
        }
    }

    Ok(None)
}

/// 从 WorkosCursorSessionToken 获取 AccessToken
///
/// 1. 生成 PKCE verifier 和 challenge
/// 2. 触发授权登录（用 session token 自动确认）
/// 3. 轮询获取 accessToken
///
/// # 参数
/// - `session_token`: WorkosCursorSessionToken (网页登录后的 cookie)
///
/// # 返回
/// - 成功: AccessTokenResponse 包含 accessToken 和 refreshToken
/// - 失败: 错误信息
pub async fn get_access_token_from_session(
    session_token: &str,
) -> Result<AccessTokenResponse, String> {
    // 1. 生成 PKCE 参数
    let uuid = uuid::Uuid::new_v4().to_string();
    let verifier = generate_pkce_verifier();
    let challenge = generate_pkce_challenge(&verifier);

    // 2. 触发授权登录
    trigger_authorization_login(&uuid, &challenge, session_token).await?;

    // 3. 轮询获取 token (最多尝试 20 次，每次间隔 1 秒)
    for _ in 0..20 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if let Some(token_response) = poll_for_access_token(&uuid, &verifier).await? {
            return Ok(token_response);
        }
    }

    Err("Timeout: Failed to get access token after 20 seconds".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plan_cents_and_percent_fields() {
        let raw = serde_json::json!({
            "membershipType": "pro",
            "billingCycleStart": "2026-08-02T14:11:55.000Z",
            "billingCycleEnd": "2026-09-02T14:11:55.000Z",
            "individualUsage": {
                "plan": {
                    "enabled": true,
                    "used": 1288,
                    "limit": 2000,
                    "remaining": 712,
                    "breakdown": { "included": 1288, "bonus": 0, "total": 1288 },
                    "autoPercentUsed": 3.8966666666666665,
                    "apiPercentUsed": 2.6444444444444444,
                    "totalPercentUsed": 3.733333333333334
                },
                "onDemand": { "enabled": true, "used": 2309, "limit": null, "remaining": null }
            }
        });

        let summary: UsageSummary = serde_json::from_value(raw).unwrap();
        let usage = summary.individual_usage.unwrap();
        let plan = usage.plan.unwrap();

        assert_eq!(plan.used, Some(1288.0));
        assert_eq!(plan.limit, Some(2000.0));
        assert_eq!(plan.remaining, Some(712.0));
        assert!((plan.auto_percent_used.unwrap() - 3.8966666666666665).abs() < 1e-9);

        // On-Demand 无上限时保持 None，不能退化成 $0 上限
        let on_demand = usage.on_demand.unwrap();
        assert_eq!(on_demand.used, 2309.0);
        assert_eq!(on_demand.limit, None);
    }

    #[test]
    fn missing_percent_fields_stay_none() {
        let raw = serde_json::json!({
            "membershipType": "enterprise",
            "individualUsage": {
                "plan": { "enabled": true, "used": 1500, "limit": 2000, "remaining": 500 }
            }
        });

        let summary: UsageSummary = serde_json::from_value(raw).unwrap();
        let plan = summary
            .individual_usage
            .and_then(|u| u.plan)
            .expect("plan present");

        assert_eq!(plan.auto_percent_used, None);
        assert_eq!(plan.api_percent_used, None);
        assert_eq!(plan.total_percent_used, None);
    }

    #[test]
    fn absent_percent_fields_are_not_serialized_as_zero() {
        let raw = serde_json::json!({
            "individualUsage": { "plan": { "enabled": true, "used": null, "limit": null } }
        });

        let summary: UsageSummary = serde_json::from_value(raw).unwrap();
        let json = serde_json::to_value(&summary).unwrap();
        let plan = &json["individualUsage"]["plan"];

        assert!(plan.get("autoPercentUsed").is_none());
        assert!(plan.get("used").is_none());
        assert!(plan.get("limit").is_none());
    }
}
