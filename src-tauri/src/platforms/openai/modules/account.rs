use crate::platforms::openai::models::{Account, AccountType, QuotaData, TokenData};
use crate::platforms::openai::modules::{oauth, quota};

/// 从错误消息中提取 rt_invalid 原因
fn extract_rt_invalid_reason(err: &str) -> Option<String> {
    if err.contains("refresh_token_reused") {
        Some("refresh_token_reused".to_string())
    } else if err.contains("invalid_grant") {
        Some("invalid_grant".to_string())
    } else {
        None
    }
}

/// 带有重试机制的配额查询
pub async fn fetch_quota_with_retry(account: &mut Account) -> Result<QuotaData, String> {
    println!("=== OpenAI fetch_quota_with_retry ===");
    println!("account.email: {}", account.email);

    // API 账号不支持配额查询
    if account.account_type == AccountType::API {
        return Err("API accounts do not support quota fetching".to_string());
    }

    // 1. 刷新配额时总是先尝试刷新 Token，失败则 fallback 用现有 token
    match refresh_token_if_needed(account, 0, true).await {
        Ok(refreshed) => {
            if refreshed {
                println!("Token refreshed, updating account");
            }
        }
        Err(e) => {
            println!("Token refresh failed (will try existing token): {}", e);
        }
    }

    let access_token = account
        .token
        .as_ref()
        .map(|t| t.access_token.clone())
        .ok_or_else(|| "OAuth account missing token".to_string())?;

    // 2. 查询配额
    let result =
        quota::fetch_quota(&access_token, account.chatgpt_account_id.as_deref()).await;

    // 3. 处理 401 错误 - 强制刷新 token 后重试
    if let Err(ref e) = result {
        if e.contains("401") || e.contains("unauthorized") {
            println!("Got 401 error, trying force token refresh...");

            match refresh_token_if_needed(account, 0, true).await {
                Ok(true) => {
                    let new_access_token = account
                        .token
                        .as_ref()
                        .map(|t| t.access_token.clone())
                        .ok_or_else(|| "OAuth account missing token".to_string())?;

                    println!("Retrying quota fetch with new token...");
                    let retry = quota::fetch_quota(
                        &new_access_token,
                        account.chatgpt_account_id.as_deref(),
                    )
                    .await;
                    if let Err(ref e) = retry {
                        if e.contains("401") || e.contains("unauthorized") {
                            account.rt_invalid = true;
                            account.rt_invalid_reason = Some("unauthorized".to_string());
                        }
                    }
                    return retry;
                }
                Ok(false) => {
                    account.rt_invalid = true;
                    account.rt_invalid_reason = Some("unauthorized".to_string());
                }
                Err(e) => {
                    return Err(format!("Token refresh failed: {}", e));
                }
            }
        }
    }

    result
}

/// 刷新配额（含 token 续期、拉取配额、更新 account 配额与 openai_auth_json）。
/// 供手动刷新、批量刷新、定时任务等统一调用；调用方负责保存账号。
pub async fn refresh_quota_and_backfill(account: &mut Account) -> Result<QuotaData, String> {
    let quota = fetch_quota_with_retry(account).await?;
    account.update_quota(quota.clone());
    backfill_openai_auth_json_if_missing(account);

    if missing_subscription_expiry(account) {
        if let Some(access_token) = account
            .token
            .as_ref()
            .map(|token| token.access_token.clone())
        {
            oauth::enrich_openai_auth_json_with_account_check(
                &access_token,
                account.organization_id.as_deref(),
                account.chatgpt_account_id.as_deref(),
                &mut account.openai_auth_json,
            )
            .await;
        }
    }

    Ok(quota)
}

pub(crate) fn missing_subscription_expiry(account: &Account) -> bool {
    let expiry_str = account
        .openai_auth_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|value| {
            value
                .get("chatgpt_subscription_active_until")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
        });

    match expiry_str {
        // 无订阅到期数据，需要补充
        None => true,
        Some(expiry) => {
            // JWT id_token 中的订阅到期时间是 token 签发时的快照。
            // 如果存储的到期日已过，可能已经自动续费，需要重新通过 API 获取。
            chrono::DateTime::parse_from_rfc3339(&expiry)
                .map(|dt| dt < chrono::Utc::now())
                .unwrap_or(false)
        }
    }
}

/// 消费一张限流重置券后刷新配额（token 续期、consume、重新拉取配额）。
/// 调用方负责保存账号。
pub async fn consume_reset_credit(account: &mut Account) -> Result<QuotaData, String> {
    if account.account_type == AccountType::API {
        return Err("API accounts do not support reset credits".to_string());
    }

    // 消费前尽量拿到有效 token（失败则用现有 token 继续尝试）
    let _ = refresh_token_if_needed(account, 0, true).await;

    let access_token = account
        .token
        .as_ref()
        .map(|t| t.access_token.clone())
        .ok_or_else(|| "OAuth account missing token".to_string())?;

    quota::consume_reset_credit(&access_token, account.chatgpt_account_id.as_deref()).await?;

    // 消费成功后重新拉取配额，反映重置后的窗口与剩余重置券
    refresh_quota_and_backfill(account).await
}

pub async fn refresh_token_if_needed(
    account: &mut Account,
    refresh_window_secs: i64,
    force: bool,
) -> Result<bool, String> {
    // API 账号不支持 token 刷新
    if account.account_type == AccountType::API {
        return Ok(false);
    }

    let refresh_window_secs = refresh_window_secs.max(0);

    let current_token = account
        .token
        .as_ref()
        .ok_or("OAuth account missing token".to_string())?;

    if !force && !oauth::token_needs_refresh(current_token, refresh_window_secs) {
        return Ok(false);
    }

    let account_token = current_token;

    let new_token = if force {
        let refresh_token_value = account_token
            .refresh_token
            .as_ref()
            .ok_or_else(|| "No refresh token available".to_string())?;
        match oauth::refresh_token(refresh_token_value).await {
            Ok(response) => {
                let now = chrono::Utc::now().timestamp();
                TokenData::new(
                    response.access_token,
                    response
                        .refresh_token
                        .or_else(|| account_token.refresh_token.clone()),
                    response.id_token.or_else(|| account_token.id_token.clone()),
                    response.expires_in,
                    now + response.expires_in,
                    response.token_type,
                )
            }
            Err(e) => {
                if e.contains("refresh_token_reused") || e.contains("invalid_grant") {
                    account.rt_invalid = true;
                    account.rt_invalid_reason = extract_rt_invalid_reason(&e);
                }
                return Err(e);
            }
        }
    } else {
        match oauth::ensure_fresh_token_with_window(account_token, refresh_window_secs).await {
            Ok(token) => token,
            Err(e) => {
                if e.contains("refresh_token_reused") || e.contains("invalid_grant") {
                    account.rt_invalid = true;
                    account.rt_invalid_reason = extract_rt_invalid_reason(&e);
                }
                return Err(e);
            }
        }
    };

    if let Some(ref account_token) = account.token {
        if new_token.access_token == account_token.access_token
            && new_token.expires_at == account_token.expires_at
        {
            return Ok(false);
        }
    }

    account.token = Some(new_token);
    account.updated_at = chrono::Utc::now().timestamp();
    account.rt_invalid = false;
    account.rt_invalid_reason = None;

    Ok(true)
}

fn has_empty_openai_auth_json(account: &Account) -> bool {
    account
        .openai_auth_json
        .as_ref()
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}

/// 用当前 token 中的 id_token 解析并更新 account.openai_auth_json（订阅到期、套餐等字段）。
/// 解析逻辑在 oauth::extract_openai_auth_json：解码 JWT payload，取出 "https://api.openai.com/auth" 并序列化为 JSON。
/// 可在刷新配额/刷新 token 后调用，保证订阅信息与 id_token 一致。
pub fn backfill_openai_auth_json_if_missing(account: &mut Account) -> bool {
    let id_token = account.token.as_ref().and_then(|t| t.id_token.as_deref());
    let Some(id_token) = id_token else {
        return false;
    };
    let Some(auth_json) = oauth::extract_openai_auth_json(id_token) else {
        return false;
    };
    let was_empty = has_empty_openai_auth_json(account);
    account.openai_auth_json = Some(auth_json);
    was_empty
}
