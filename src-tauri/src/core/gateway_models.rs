//! 网关模型库：从公开供应商配置远程同步模型目录，落盘到 appdata。
//!
//! - `gateway_sync_models`：联网拉取 + 过滤/归并/排序 + 落盘
//! - `gateway_get_models`：读取本地缓存
//!
//! 复用 `core::http_client::create_http_client()` 自动套用代理配置。

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SOURCE_URL: &str =
    "https://raw.githubusercontent.com/ThinkInAIXYZ/PublicProviderConf/refs/heads/dev/dist/all.json";

const MODELS_FILENAME: &str = "gateway_models.json";

/// 白名单开发商（按 provider 的 key 或 id 不区分大小写匹配）。
const ALLOWED_DEVELOPERS: &[&str] = &[
    "openai", "anthropic", "google", "meta", "deepseek", "alibaba", "bytedance", "x-ai", "xai",
    "mistral", "mistralai", "moonshotai", "moonshot", "zhipu", "zhipuai", "qwen", "cohere",
    "perplexity", "groq",
];

/// llama 渠道 → meta，doubao 渠道 → bytedance 的通用归并映射。
const PROVIDER_REMAP: &[(&str, &str, &str)] = &[
    ("llama", "meta", "Meta"),
    ("doubao", "bytedance", "ByteDance"),
];

// --- 远程结构（宽松反序列化）---

#[derive(Deserialize, Default)]
struct RemoteConf {
    #[serde(default)]
    providers: HashMap<String, RemoteProvider>,
}

#[derive(Deserialize, Clone, Default)]
struct RemoteProvider {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    models: Vec<Model>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Model {
    #[serde(default)]
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release_date: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

// --- 输出结构（落盘 + 返回前端）---

#[derive(Serialize, Deserialize, Clone)]
pub struct OutProvider {
    id: String,
    name: String,
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModelsFile {
    #[serde(default)]
    synced_at: Option<String>,
    #[serde(default)]
    providers: Vec<OutProvider>,
}

#[derive(Serialize)]
pub struct SyncResult {
    synced_at: String,
    providers: usize,
    models: usize,
}

fn models_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    Ok(dir.join(MODELS_FILENAME))
}

fn is_allowed(key: &str, id: &str) -> bool {
    ALLOWED_DEVELOPERS
        .iter()
        .any(|d| d.eq_ignore_ascii_case(key) || d.eq_ignore_ascii_case(id))
}

fn release_key(m: &Model) -> NaiveDate {
    m.release_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
}

/// 过滤白名单 + 通用归并 + 按发布日期倒序排序。
fn build_providers(conf: RemoteConf) -> Vec<OutProvider> {
    let mut out: Vec<OutProvider> = Vec::new();

    for (key, provider) in conf.providers.iter() {
        if !is_allowed(key, &provider.id) {
            continue;
        }
        let name = if !provider.display_name.is_empty() {
            provider.display_name.clone()
        } else if !provider.name.is_empty() {
            provider.name.clone()
        } else {
            key.clone()
        };
        out.push(OutProvider {
            id: if provider.id.is_empty() { key.clone() } else { provider.id.clone() },
            name,
            models: provider.models.clone(),
        });
    }

    for (src_key, dst_id, dst_name) in PROVIDER_REMAP {
        if !is_allowed(dst_id, dst_id) {
            continue;
        }
        if out.iter().any(|p| p.id.eq_ignore_ascii_case(dst_id)) {
            continue;
        }
        if let Some(src) = conf.providers.get(*src_key) {
            let models: Vec<Model> = src
                .models
                .iter()
                .filter(|m| m.id.to_lowercase().starts_with(&src_key.to_lowercase()))
                .cloned()
                .collect();
            if !models.is_empty() {
                out.push(OutProvider {
                    id: dst_id.to_string(),
                    name: dst_name.to_string(),
                    models,
                });
            }
        }
    }

    for p in out.iter_mut() {
        p.models.sort_by(|a, b| release_key(b).cmp(&release_key(a)));
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

#[tauri::command]
pub async fn gateway_sync_models(app: AppHandle) -> Result<SyncResult, String> {
    let client = crate::http_client::create_http_client()?;
    let resp = client
        .get(SOURCE_URL)
        .header("User-Agent", "ATM-Gateway-Models")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("请求失败，HTTP 状态 {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let conf: RemoteConf =
        serde_json::from_str(&text).map_err(|e| format!("解析供应商配置失败: {}", e))?;

    let providers = build_providers(conf);
    let model_count: usize = providers.iter().map(|p| p.models.len()).sum();
    let synced_at = chrono::Utc::now().to_rfc3339();

    let file = ModelsFile {
        synced_at: Some(synced_at.clone()),
        providers,
    };

    let path = models_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("写入模型库失败: {}", e))?;

    Ok(SyncResult {
        synced_at,
        providers: file.providers.len(),
        models: model_count,
    })
}

#[tauri::command]
pub async fn gateway_get_models(app: AppHandle) -> Result<ModelsFile, String> {
    let path = models_path(&app)?;
    if !path.exists() {
        return Ok(ModelsFile::default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("读取模型库失败: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("解析模型库失败: {}", e))
}
