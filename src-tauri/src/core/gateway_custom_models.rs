//! 网关自定义模型：用户在「模型」Tab 手动新建的模型（含价格），独立持久化到 appdata。
//!
//! - `gateway_get_custom_models`：读取本地 `gateway_custom_models.json`
//! - `gateway_set_custom_models`：覆盖写入
//!
//! 独立于会被同步覆盖的 `gateway_models.json`；前端按 `developer` 归并进展示分组、同 `id` 自定义优先。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const CUSTOM_MODELS_FILENAME: &str = "gateway_custom_models.json";

/// 自定义模型：固定 `id`，其余字段（`developer` / `cost` 等）原样保留。
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CustomModel {
    #[serde(default)]
    id: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Default)]
struct CustomModelsFile {
    #[serde(default)]
    models: Vec<CustomModel>,
}

fn custom_models_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    Ok(dir.join(CUSTOM_MODELS_FILENAME))
}

#[tauri::command]
pub async fn gateway_get_custom_models(app: AppHandle) -> Result<Vec<CustomModel>, String> {
    let path = custom_models_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("读取自定义模型失败: {}", e))?;
    let file: CustomModelsFile =
        serde_json::from_str(&content).map_err(|e| format!("解析自定义模型失败: {}", e))?;
    Ok(file.models)
}

#[tauri::command]
pub async fn gateway_set_custom_models(
    app: AppHandle,
    models: Vec<CustomModel>,
) -> Result<(), String> {
    let path = custom_models_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    let file = CustomModelsFile { models };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("写入自定义模型失败: {}", e))?;
    Ok(())
}
