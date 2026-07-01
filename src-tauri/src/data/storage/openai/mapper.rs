use crate::data::storage::common::{AccountDbMapper, StorageError};
use crate::platforms::openai::models::{Account, AccountType, ApiConfig, QuotaData, TokenData};
use tokio_postgres::Row;

/// OpenAI 账号数据库映射器
pub struct OpenAIAccountMapper;

impl AccountDbMapper<Account> for OpenAIAccountMapper {
    fn from_row(row: &Row) -> Result<Account, StorageError> {
        // 读取账号类型
        let account_type: String = row.try_get(26).unwrap_or_else(|_| "oauth".to_string());
        let account_type = match account_type.as_str() {
            "api" => AccountType::API,
            _ => AccountType::OAuth,
        };

        // 读取 token 数据（仅 OAuth 账号）
        let token = if account_type == AccountType::OAuth {
            Some(TokenData {
                access_token: row.get(2),
                refresh_token: row.get(3),
                id_token: row.get(4),
                expires_in: row.get(5),
                expires_at: row.get(6),
                token_type: row.get(7),
            })
        } else {
            None
        };

        // 读取 API 配置（仅 API 账号）
        let api_config = if account_type == AccountType::API {
            let model_provider: Option<String> = row.try_get(27).ok().flatten();
            let model: Option<String> = row.try_get(28).ok().flatten();
            let model_reasoning_effort: Option<String> = row.try_get(29).ok().flatten();
            let wire_api: Option<String> = row.try_get(30).ok().flatten();
            let base_url: Option<String> = row.try_get(31).ok().flatten();
            let api_key: Option<String> = row.try_get(32).ok().flatten();

            if model_provider.is_some()
                || model.is_some()
                || base_url.is_some()
                || api_key.is_some()
            {
                Some(ApiConfig {
                    model_provider,
                    model,
                    model_reasoning_effort,
                    wire_api,
                    base_url,
                    key: api_key,
                })
            } else {
                None
            }
        } else {
            None
        };

        // 检查是否有配额字段（通过检查 codex_usage_updated_at 字段是否存在）
        // is_forbidden 在索引 34
        let is_forbidden: bool = row.try_get(34).unwrap_or(false);
        let quota = if row.len() > 18 {
            // 有配额字段
            let quota_data = QuotaData {
                codex_5h_used_percent: row.try_get(18).ok().flatten(),
                codex_5h_reset_after_seconds: row.try_get(19).ok().flatten(),
                codex_5h_window_minutes: row.try_get(20).ok().flatten(),
                codex_7d_used_percent: row.try_get(21).ok().flatten(),
                codex_7d_reset_after_seconds: row.try_get(22).ok().flatten(),
                codex_7d_window_minutes: row.try_get(23).ok().flatten(),
                codex_primary_over_secondary_percent: row.try_get(24).ok().flatten(),
                codex_usage_updated_at: row.try_get(25).unwrap_or(chrono::Utc::now().timestamp()),
                is_forbidden,
                reset_credits_available: None,
                reset_credits_total: None,
            };
            if quota_data.is_valid() || is_forbidden {
                Some(quota_data)
            } else {
                None
            }
        } else if is_forbidden {
            // 即使没有配额数据，如果 is_forbidden 为 true，也需要创建 quota
            Some(QuotaData {
                is_forbidden: true,
                ..QuotaData::new()
            })
        } else {
            None
        };

        let rt_invalid: bool = row.try_get(35).unwrap_or(false);
        let rt_invalid_reason: Option<String> = row.try_get(36).ok().flatten();

        Ok(Account {
            id: row.get(0),
            email: row.get(1),
            reverse_proxy_enabled: row.try_get(37).unwrap_or(true),
            account_type,
            token,
            api_config,
            chatgpt_account_id: row.get(8),
            chatgpt_user_id: row.get(9),
            organization_id: row.get(10),
            openai_auth_json: row.try_get(33).ok().flatten(),
            quota,
            tag: row.try_get(16).ok().flatten(),
            tag_color: row.try_get(17).ok().flatten(),
            created_at: row.get(11),
            last_used: row.get(12),
            updated_at: row.get(13),
            deleted: row.get(14),
            version: row.get(15),
            rt_invalid,
            rt_invalid_reason,
        })
    }

    fn select_columns() -> &'static str {
        "id, email, access_token, refresh_token, id_token, expires_in, expires_at, token_type, \
         chatgpt_account_id, chatgpt_user_id, organization_id, created_at, last_used, updated_at, deleted, version, \
         tag, tag_color, \
         codex_5h_used_percent, codex_5h_reset_after_seconds, codex_5h_window_minutes, \
         codex_7d_used_percent, codex_7d_reset_after_seconds, codex_7d_window_minutes, \
         codex_primary_over_secondary_percent, codex_usage_updated_at, \
         account_type, model_provider, model, model_reasoning_effort, wire_api, base_url, api_key, \
         openai_auth_json, is_forbidden, rt_invalid, rt_invalid_reason, reverse_proxy_enabled"
    }

    fn insert_sql() -> &'static str {
        r#"
        INSERT INTO openai_accounts
            (id, email, access_token, refresh_token, id_token, expires_in, expires_at, token_type,
             chatgpt_account_id, chatgpt_user_id, organization_id, created_at, last_used, updated_at,
             version, deleted, tag, tag_color, codex_5h_used_percent, codex_5h_reset_after_seconds, codex_5h_window_minutes,
             codex_7d_used_percent, codex_7d_reset_after_seconds, codex_7d_window_minutes,
             codex_primary_over_secondary_percent, codex_usage_updated_at, account_type,
             model_provider, model, model_reasoning_effort, wire_api, base_url, api_key, openai_auth_json, is_forbidden, rt_invalid, rt_invalid_reason, reverse_proxy_enabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38)
        ON CONFLICT (id) DO UPDATE SET
            email = EXCLUDED.email,
            access_token = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token,
            id_token = EXCLUDED.id_token,
            expires_in = EXCLUDED.expires_in,
            expires_at = EXCLUDED.expires_at,
            token_type = EXCLUDED.token_type,
            chatgpt_account_id = EXCLUDED.chatgpt_account_id,
            chatgpt_user_id = EXCLUDED.chatgpt_user_id,
            organization_id = EXCLUDED.organization_id,
            last_used = EXCLUDED.last_used,
            updated_at = EXCLUDED.updated_at,
            version = EXCLUDED.version,
            deleted = EXCLUDED.deleted,
            tag = EXCLUDED.tag,
            tag_color = EXCLUDED.tag_color,
            codex_5h_used_percent = EXCLUDED.codex_5h_used_percent,
            codex_5h_reset_after_seconds = EXCLUDED.codex_5h_reset_after_seconds,
            codex_5h_window_minutes = EXCLUDED.codex_5h_window_minutes,
            codex_7d_used_percent = EXCLUDED.codex_7d_used_percent,
            codex_7d_reset_after_seconds = EXCLUDED.codex_7d_reset_after_seconds,
            codex_7d_window_minutes = EXCLUDED.codex_7d_window_minutes,
            codex_primary_over_secondary_percent = EXCLUDED.codex_primary_over_secondary_percent,
            codex_usage_updated_at = EXCLUDED.codex_usage_updated_at,
            account_type = EXCLUDED.account_type,
            model_provider = EXCLUDED.model_provider,
            model = EXCLUDED.model,
            model_reasoning_effort = EXCLUDED.model_reasoning_effort,
            wire_api = EXCLUDED.wire_api,
            base_url = EXCLUDED.base_url,
            api_key = EXCLUDED.api_key,
            openai_auth_json = EXCLUDED.openai_auth_json,
            is_forbidden = EXCLUDED.is_forbidden,
            rt_invalid = EXCLUDED.rt_invalid,
            rt_invalid_reason = EXCLUDED.rt_invalid_reason,
            reverse_proxy_enabled = EXCLUDED.reverse_proxy_enabled
        "#
    }

    fn to_params(
        account: &Account,
        version: i64,
    ) -> Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> {
        let (
            codex_5h_used_percent,
            codex_5h_reset_after_seconds,
            codex_5h_window_minutes,
            codex_7d_used_percent,
            codex_7d_reset_after_seconds,
            codex_7d_window_minutes,
            codex_primary_over_secondary_percent,
            codex_usage_updated_at,
        ) = if let Some(ref quota) = account.quota {
            (
                quota.codex_5h_used_percent,
                quota.codex_5h_reset_after_seconds,
                quota.codex_5h_window_minutes,
                quota.codex_7d_used_percent,
                quota.codex_7d_reset_after_seconds,
                quota.codex_7d_window_minutes,
                quota.codex_primary_over_secondary_percent,
                quota.codex_usage_updated_at,
            )
        } else {
            (
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                chrono::Utc::now().timestamp(),
            )
        };

        // 获取 account_type 字符串
        let account_type_str = match account.account_type {
            AccountType::OAuth => "oauth",
            AccountType::API => "api",
        };

        // 获取 token 相关字段（OAuth 账号）
        let access_token = account
            .token
            .as_ref()
            .map(|t| t.access_token.clone())
            .unwrap_or_default();
        let refresh_token = account.token.as_ref().and_then(|t| t.refresh_token.clone());
        let id_token = account.token.as_ref().and_then(|t| t.id_token.clone());
        let expires_in = account.token.as_ref().map(|t| t.expires_in).unwrap_or(0);
        let expires_at = account.token.as_ref().map(|t| t.expires_at).unwrap_or(0);
        let token_type = account.token.as_ref().and_then(|t| t.token_type.clone());

        // 获取 API 配置字段
        let (model_provider, model, model_reasoning_effort, wire_api, base_url, api_key) =
            if let Some(ref api_config) = account.api_config {
                (
                    api_config.model_provider.clone(),
                    api_config.model.clone(),
                    api_config.model_reasoning_effort.clone(),
                    api_config.wire_api.clone(),
                    api_config.base_url.clone(),
                    api_config.key.clone(),
                )
            } else {
                (None, None, None, None, None, None)
            };

        // 获取 is_forbidden
        let is_forbidden = account
            .quota
            .as_ref()
            .map(|q| q.is_forbidden)
            .unwrap_or(false);

        vec![
            Box::new(account.id.clone()),
            Box::new(account.email.clone()),
            Box::new(access_token),
            Box::new(refresh_token),
            Box::new(id_token),
            Box::new(expires_in),
            Box::new(expires_at),
            Box::new(token_type),
            Box::new(account.chatgpt_account_id.clone()),
            Box::new(account.chatgpt_user_id.clone()),
            Box::new(account.organization_id.clone()),
            Box::new(account.created_at),
            Box::new(account.last_used),
            Box::new(account.updated_at),
            Box::new(version),
            Box::new(account.deleted),
            Box::new(account.tag.clone()),
            Box::new(account.tag_color.clone()),
            Box::new(codex_5h_used_percent),
            Box::new(codex_5h_reset_after_seconds),
            Box::new(codex_5h_window_minutes),
            Box::new(codex_7d_used_percent),
            Box::new(codex_7d_reset_after_seconds),
            Box::new(codex_7d_window_minutes),
            Box::new(codex_primary_over_secondary_percent),
            Box::new(codex_usage_updated_at),
            Box::new(account_type_str.to_string()),
            Box::new(model_provider),
            Box::new(model),
            Box::new(model_reasoning_effort),
            Box::new(wire_api),
            Box::new(base_url),
            Box::new(api_key),
            Box::new(account.openai_auth_json.clone()),
            Box::new(is_forbidden),
            Box::new(account.rt_invalid),
            Box::new(account.rt_invalid_reason.clone()),
            Box::new(account.reverse_proxy_enabled),
        ]
    }
}
