use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Quota fields consumed by the current frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaData {
    #[serde(
        rename = "codex_5h_used_percent",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_5h_used_percent: Option<f64>,

    #[serde(
        rename = "codex_5h_reset_after_seconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_5h_reset_after_seconds: Option<i64>,

    #[serde(
        rename = "codex_5h_window_minutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_5h_window_minutes: Option<i64>,

    #[serde(
        rename = "codex_7d_used_percent",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_7d_used_percent: Option<f64>,

    #[serde(
        rename = "codex_7d_reset_after_seconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_7d_reset_after_seconds: Option<i64>,

    #[serde(
        rename = "codex_7d_window_minutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_7d_window_minutes: Option<i64>,

    #[serde(
        rename = "codex_primary_over_secondary_percent",
        skip_serializing_if = "Option::is_none"
    )]
    pub codex_primary_over_secondary_percent: Option<f64>,

    #[serde(rename = "codex_usage_updated_at")]
    pub codex_usage_updated_at: i64,

    #[serde(default)]
    pub is_forbidden: bool,

    /// 可用重置额度（rate-limit-reset-credits 中 status=available 的数量）
    #[serde(
        rename = "codex_reset_credits_available",
        skip_serializing_if = "Option::is_none"
    )]
    pub reset_credits_available: Option<i64>,

    /// 重置额度总数（已授予的重置券总量，用于展示已消费 = 总数 - 可用）
    #[serde(
        rename = "codex_reset_credits_total",
        skip_serializing_if = "Option::is_none"
    )]
    pub reset_credits_total: Option<i64>,
}

/// `GET /wham/rate-limit-reset-credits` 响应：限流重置券列表
#[derive(Debug, Clone, Deserialize)]
pub struct CodexResetCreditsResponse {
    #[serde(default)]
    pub credits: Vec<CodexResetCredit>,
    #[serde(default)]
    pub available_count: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexResetCredit {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `POST /wham/rate-limit-reset-credits/consume` 响应
#[derive(Debug, Clone, Deserialize)]
pub struct CodexResetConsumeResponse {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub windows_reset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhamUsageResponse {
    #[serde(default)]
    pub rate_limit: Option<WhamRateLimit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhamRateLimit {
    #[serde(default)]
    pub primary_window: Option<WhamRateLimitWindow>,
    #[serde(default)]
    pub secondary_window: Option<WhamRateLimitWindow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhamRateLimitWindow {
    #[serde(default)]
    pub used_percent: Option<f64>,
    #[serde(default)]
    pub limit_window_seconds: Option<i64>,
    #[serde(default)]
    pub reset_after_seconds: Option<i64>,
    #[serde(default)]
    pub reset_at: Option<i64>,
}

impl QuotaData {
    pub fn new() -> Self {
        Self {
            codex_5h_used_percent: None,
            codex_5h_reset_after_seconds: None,
            codex_5h_window_minutes: None,
            codex_7d_used_percent: None,
            codex_7d_reset_after_seconds: None,
            codex_7d_window_minutes: None,
            codex_primary_over_secondary_percent: None,
            codex_usage_updated_at: Utc::now().timestamp(),
            is_forbidden: false,
            reset_credits_available: None,
            reset_credits_total: None,
        }
    }

    pub fn from_wham_usage(usage: &WhamUsageResponse) -> Self {
        let mut quota = Self::new();
        let now = Utc::now().timestamp();

        if let Some(rate_limit) = &usage.rate_limit {
            if let Some(primary) = &rate_limit.primary_window {
                quota.codex_5h_used_percent = primary.used_percent.map(clamp_percent);
                quota.codex_5h_window_minutes =
                    window_minutes_from_seconds(primary.limit_window_seconds);
                quota.codex_5h_reset_after_seconds =
                    reset_after_seconds(now, primary.reset_after_seconds, primary.reset_at);
            }

            if let Some(secondary) = &rate_limit.secondary_window {
                quota.codex_7d_used_percent = secondary.used_percent.map(clamp_percent);
                quota.codex_7d_window_minutes =
                    window_minutes_from_seconds(secondary.limit_window_seconds);
                quota.codex_7d_reset_after_seconds =
                    reset_after_seconds(now, secondary.reset_after_seconds, secondary.reset_at);
            }
        }

        quota
    }

    pub fn is_valid(&self) -> bool {
        self.codex_5h_used_percent.is_some() || self.codex_7d_used_percent.is_some()
    }
}

impl Default for QuotaData {
    fn default() -> Self {
        Self::new()
    }
}

fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn window_minutes_from_seconds(seconds: Option<i64>) -> Option<i64> {
    let seconds = seconds?;
    if seconds <= 0 {
        return None;
    }
    Some((seconds + 59) / 60)
}

fn reset_after_seconds(
    now: i64,
    reset_after_seconds: Option<i64>,
    reset_at: Option<i64>,
) -> Option<i64> {
    if let Some(seconds) = reset_after_seconds {
        return Some(seconds.max(0));
    }

    reset_at.map(|timestamp| (timestamp - now).max(0))
}
