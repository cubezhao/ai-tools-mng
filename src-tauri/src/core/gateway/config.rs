//! 网关渠道与配置模型
//!
//! 与前端 `stores/gateway.js` 配置契约对齐（camelCase）：渠道列表内嵌模型与
//! 优先级，账号仅存引用，不含独立监听端口（复用 `8766`）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::translate::Wire;

/// 网关配置落盘文件名
const CONFIG_FILENAME: &str = "gateway_config.json";

/// 网关配置文件路径
pub fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_FILENAME)
}

/// 渠道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    /// 复用现有 OpenAI OAuth 账号，走 Codex Responses 上游
    CodexOauth,
    /// OpenAI 兼容端点（Base URL + Key）
    OpenaiCompat,
    /// Anthropic Messages 端点
    Anthropic,
}

fn default_true() -> bool {
    true
}

/// 渠道模型条目：兼容纯字符串（别名==上游 id）与 `{ id, upstream }` 别名映射
///
/// - `id`：对客户端暴露的别名，作为匹配键与用量记录名；
/// - `upstream`：转发给上游时使用的真实模型 id，缺省/为空则等于 `id`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelEntry {
    /// 旧格式或无别名：别名与上游 id 一致
    Simple(String),
    /// 别名映射：客户端别名 `id` → 上游真实 id `upstream`
    Aliased {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upstream: Option<String>,
    },
}

impl ModelEntry {
    /// 对客户端暴露的别名（匹配键）
    pub fn id(&self) -> &str {
        match self {
            ModelEntry::Simple(s) => s,
            ModelEntry::Aliased { id, .. } => id,
        }
    }

    /// 转发上游时使用的真实模型 id（别名为空时回退到别名）
    pub fn upstream(&self) -> &str {
        match self {
            ModelEntry::Simple(s) => s,
            ModelEntry::Aliased { id, upstream } => upstream
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(id),
        }
    }
}

/// 出站渠道（一份上游凭证 + 其支持的模型集合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayChannel {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub kind: ChannelKind,
    /// CodexOauth：绑定的 OpenAI OAuth 账号 id
    #[serde(
        default,
        rename = "accountId",
        skip_serializing_if = "Option::is_none"
    )]
    pub account_id: Option<String>,
    /// OpenaiCompat / Anthropic：上游 Base URL
    #[serde(default, rename = "baseUrl", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// OpenaiCompat / Anthropic：上游 API Key
    #[serde(default, rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// OpenaiCompat 线型（chat | responses），缺省按 chat
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire: Option<String>,
    /// 支持的模型列表（别名或别名→上游映射）
    #[serde(default)]
    pub models: Vec<ModelEntry>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 优先级（数值越小越优先）
    #[serde(default = "default_priority")]
    pub priority: i64,
}

fn default_priority() -> i64 {
    100
}

impl GatewayChannel {
    /// 渠道出站线型
    pub fn wire(&self) -> Wire {
        match self.kind {
            ChannelKind::CodexOauth => Wire::Responses,
            ChannelKind::Anthropic => Wire::Anthropic,
            ChannelKind::OpenaiCompat => match self.wire.as_deref() {
                Some("responses") => Wire::Responses,
                _ => Wire::Chat,
            },
        }
    }

    /// 该渠道是否支持指定模型（按别名匹配）
    pub fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.id() == model)
    }

    /// 将客户端别名解析为转发上游的真实模型 id（未命中返回原值）
    pub fn upstream_model(&self, model: &str) -> String {
        self.models
            .iter()
            .find(|m| m.id() == model)
            .map(|m| m.upstream().to_string())
            .unwrap_or_else(|| model.to_string())
    }
}

/// 网关配置（共用 `8766`，无独立端口字段）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default)]
    pub enabled: bool,
    /// 全局 API Key（客户端鉴权）
    #[serde(default, rename = "apiKey")]
    pub api_key: String,
    #[serde(default)]
    pub channels: Vec<GatewayChannel>,
}

impl GatewayConfig {
    /// 从磁盘加载（不存在或解析失败则返回默认空配置）
    pub fn load(app_data_dir: &Path) -> Self {
        std::fs::read_to_string(config_path(app_data_dir))
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default()
    }

    /// 持久化到磁盘
    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化网关配置失败: {}", e))?;
        std::fs::write(config_path(app_data_dir), json)
            .map_err(|e| format!("写入网关配置失败: {}", e))
    }

    /// 收集支持指定模型的启用渠道，按优先级升序（同优先级保持原顺序）
    pub fn candidates_for(&self, model: &str) -> Vec<GatewayChannel> {
        let mut hits: Vec<(usize, &GatewayChannel)> = self
            .channels
            .iter()
            .enumerate()
            .filter(|(_, c)| c.enabled && c.supports_model(model))
            .collect();
        hits.sort_by(|a, b| a.1.priority.cmp(&b.1.priority).then(a.0.cmp(&b.0)));
        hits.into_iter().map(|(_, c)| c.clone()).collect()
    }
}
