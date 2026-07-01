//! Codex 透传执行器
//!
//! 负责将本地请求透传到 ChatGPT Codex 上游，并使用账号池进行鉴权与失败切换。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use reqwest::Method;
use warp::http::HeaderMap;

use super::pool::CodexPool;
use super::upstream::{
    apply_forward_headers, build_upstream_url, format_transport_error, is_retryable_transport_error,
    map_upstream_path, should_retry_status, CODEX_UPSTREAM_ORIGIN,
};
use crate::http_client::create_proxy_client_for_streaming;
use crate::platforms::openai::codex::models::{CodexError, CodexPoolAccount};
use crate::proxy_helper::ProxyClient;

/// 透传请求上下文
#[derive(Debug, Clone)]
pub struct ForwardRequest {
    pub method: Method,
    pub path: String,
    pub query: Option<String>,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub format: String,
    pub model: String,
}

/// 透传执行元数据（供上层记录日志）
#[derive(Debug, Clone)]
pub struct ForwardMeta {
    pub account_id: String,
    pub account_email: String,
    pub format: String,
    pub model: String,
    pub started_at: Instant,
}

/// Codex API 执行器
pub struct CodexExecutor {
    pool: Arc<CodexPool>,
    client: ProxyClient,
    upstream_origin: String,
}

impl CodexExecutor {
    pub fn new(pool: Arc<CodexPool>) -> Result<Self, String> {
        let client = create_proxy_client_for_streaming()?;

        Ok(Self {
            pool,
            client,
            upstream_origin: CODEX_UPSTREAM_ORIGIN.to_string(),
        })
    }

    /// 透传执行：返回上游响应（包含原始状态码与头）
    pub async fn forward(
        &self,
        request: ForwardRequest,
    ) -> Result<(reqwest::Response, ForwardMeta), CodexError> {
        let active_count = self.pool.active_count().await;
        if active_count == 0 {
            return Err(CodexError::NoAvailableAccount);
        }

        let mapped_path = map_upstream_path(&request.path)?;
        let upstream_url = build_upstream_url(
            &self.upstream_origin,
            &mapped_path,
            request.query.as_deref(),
        );

        let mut attempted_ids = HashSet::new();
        let mut selection_budget = active_count.saturating_mul(3).max(1);
        let mut last_transport_error: Option<reqwest::Error> = None;

        while attempted_ids.len() < active_count && selection_budget > 0 {
            selection_budget -= 1;

            let Some(account) = self.pool.next_account().await else {
                break;
            };
            if !attempted_ids.insert(account.id.clone()) {
                continue;
            }

            let meta = ForwardMeta {
                account_id: account.id.clone(),
                account_email: account.email.clone(),
                format: request.format.clone(),
                model: request.model.clone(),
                started_at: Instant::now(),
            };

            let response = match self.send_once(&upstream_url, &request, &account).await {
                Ok(resp) => resp,
                Err(err) => {
                    self.pool.record_failure(&account.id, None).await;
                    if is_retryable_transport_error(&err) && attempted_ids.len() < active_count {
                        last_transport_error = Some(err);
                        continue;
                    }
                    return Err(CodexError::ExecutionError(format_transport_error(&err)));
                }
            };

            let status = response.status();
            if status.is_success() {
                self.pool.record_success(&account.id).await;
                return Ok((response, meta));
            }

            self.pool
                .record_failure(&account.id, Some(status.as_u16()))
                .await;

            if should_retry_status(status) && attempted_ids.len() < active_count {
                continue;
            }

            return Ok((response, meta));
        }

        if let Some(err) = last_transport_error {
            return Err(CodexError::ExecutionError(format_transport_error(&err)));
        }

        Err(CodexError::NoAvailableAccount)
    }

    async fn send_once(
        &self,
        url: &str,
        request: &ForwardRequest,
        account: &CodexPoolAccount,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let builder = self.client.request(request.method.clone(), url);
        let builder = apply_forward_headers(
            builder,
            &request.headers,
            &account.access_token,
            &account.chatgpt_account_id,
        );

        builder.body(request.body.clone()).send().await
    }
}
