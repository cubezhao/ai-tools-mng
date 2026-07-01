//! 网关 Tauri 命令
//!
//! 配置读写、enabled 门控（start/stop）、状态查询、用量记录读取/清空，以及可绑定账号
//! （仅 OpenAI OAuth）列举。网关复用 `8766`，无独立端口；start/stop 仅切换 enabled。

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::config::GatewayConfig;
use super::usage::UsageRecord;
use crate::platforms::openai::models::AccountType;
use crate::platforms::openai::modules::storage as account_storage;
use crate::AppState;

/// 网关复用的固定端口与端点基址
const GATEWAY_PORT: u16 = 8766;
const GATEWAY_ADDRESS: &str = "http://127.0.0.1:8766/gateway";

/// 网关运行状态（端口固定 8766）
#[derive(Serialize)]
pub struct GatewayStatus {
    pub running: bool,
    pub address: String,
    pub port: u16,
}

/// 可绑定账号（仅 OpenAI OAuth）
#[derive(Serialize)]
pub struct BindableAccount {
    pub id: String,
    pub label: String,
    pub platform: String,
}

/// 应用数据目录
fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))
}

/// 读取网关配置
#[tauri::command]
pub async fn gateway_get_config(state: State<'_, AppState>) -> Result<GatewayConfig, String> {
    let cfg = state
        .gateway_config
        .lock()
        .map_err(|_| "网关配置锁中毒".to_string())?;
    Ok(cfg.clone())
}

/// 覆盖写入网关配置并持久化
#[tauri::command]
pub async fn gateway_set_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: GatewayConfig,
) -> Result<(), String> {
    let dir = app_data_dir(&app)?;
    {
        let mut guard = state
            .gateway_config
            .lock()
            .map_err(|_| "网关配置锁中毒".to_string())?;
        *guard = config;
    }
    let snapshot = state
        .gateway_config
        .lock()
        .map_err(|_| "网关配置锁中毒".to_string())?
        .clone();
    snapshot.save(&dir)
}

/// 查询网关状态（enabled 门控；端口固定 8766）
#[tauri::command]
pub async fn gateway_get_status(state: State<'_, AppState>) -> Result<GatewayStatus, String> {
    let enabled = state
        .gateway_config
        .lock()
        .map_err(|_| "网关配置锁中毒".to_string())?
        .enabled;
    Ok(GatewayStatus {
        running: enabled,
        address: GATEWAY_ADDRESS.to_string(),
        port: GATEWAY_PORT,
    })
}

/// 启用网关（enabled = true）
#[tauri::command]
pub async fn gateway_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    set_enabled(&app, &state, true)
}

/// 停用网关（enabled = false）
#[tauri::command]
pub async fn gateway_stop(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    set_enabled(&app, &state, false)
}

/// 切换 enabled 并持久化
fn set_enabled(app: &AppHandle, state: &State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let dir = app_data_dir(app)?;
    let snapshot = {
        let mut guard = state
            .gateway_config
            .lock()
            .map_err(|_| "网关配置锁中毒".to_string())?;
        guard.enabled = enabled;
        guard.clone()
    };
    snapshot.save(&dir)
}

/// 读取用量记录（最新在前）
#[tauri::command]
pub async fn gateway_list_usage(state: State<'_, AppState>) -> Result<Vec<UsageRecord>, String> {
    Ok(state.gateway_usage.list())
}

/// 清空用量记录
#[tauri::command]
pub async fn gateway_clear_usage(state: State<'_, AppState>) -> Result<(), String> {
    state.gateway_usage.clear();
    Ok(())
}

/// 列举可绑定账号（仅 OpenAI OAuth 账号）
#[tauri::command]
pub async fn gateway_list_bindable_accounts(
    app: AppHandle,
) -> Result<Vec<BindableAccount>, String> {
    let accounts = account_storage::list_accounts(&app).await?;
    Ok(accounts
        .into_iter()
        .filter(|a| a.account_type == AccountType::OAuth)
        .map(|a| BindableAccount {
            id: a.id,
            label: a.email,
            platform: "openai".to_string(),
        })
        .collect())
}
