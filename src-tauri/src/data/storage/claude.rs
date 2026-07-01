pub mod mapper;
pub mod traits;

pub use mapper::*;
pub use traits::*;

use crate::AppState;
use crate::data::storage::common::AccountStorage as CommonAccountStorage;
use crate::data::storage::common::{
    GenericDualStorage, GenericLocalStorage, GenericPostgreSQLStorage,
};
use crate::platforms::claude::Account;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State};

/// Claude 账户列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountListResponse {
    pub accounts: Vec<Account>,
}

async fn get_claude_storage_manager(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
) -> Result<Arc<ClaudeDualStorage>, String> {
    if let Some(manager) = state.claude_storage_manager.lock().unwrap().clone() {
        return Ok(manager);
    }

    initialize_claude_storage_manager(app, state)
        .await
        .map_err(|e| format!("Failed to initialize Claude storage manager: {}", e))?;

    state
        .claude_storage_manager
        .lock()
        .unwrap()
        .clone()
        .ok_or("Claude storage manager not initialized".to_string())
}

/// Claude 本地存储类型别名
pub type ClaudeLocalStorage = GenericLocalStorage<Account>;

/// Claude PostgreSQL 存储类型别名
pub type ClaudePostgreSQLStorage = GenericPostgreSQLStorage<Account, ClaudeAccountMapper>;

/// Claude 双层存储类型别名
pub type ClaudeDualStorage = GenericDualStorage<Account, ClaudeAccountMapper>;

/// 列出所有 Claude 账户
#[tauri::command]
pub async fn claude_list(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<AccountListResponse, String> {
    let storage_manager = get_claude_storage_manager(&app, &state).await?;

    let accounts = storage_manager
        .load_accounts()
        .await
        .map_err(|e| format!("Failed to load accounts: {}", e))?;

    // 过滤掉已删除的
    let active_accounts: Vec<Account> = accounts.into_iter().filter(|a| !a.deleted).collect();

    Ok(AccountListResponse {
        accounts: active_accounts,
    })
}

/// 直接从本地文件加载 Claude 账户（不依赖 storage manager 初始化）
#[tauri::command]
pub async fn claude_load_accounts_local(
    app: tauri::AppHandle,
) -> Result<AccountListResponse, String> {
    let local_storage = ClaudeLocalStorage::new(&app)
        .map_err(|e| format!("Failed to create Claude local storage: {}", e))?;

    let accounts = local_storage
        .load_accounts()
        .await
        .map_err(|e| format!("Failed to load Claude local accounts: {}", e))?;

    let active_accounts: Vec<Account> = accounts.into_iter().filter(|a| !a.deleted).collect();

    Ok(AccountListResponse {
        accounts: active_accounts,
    })
}

/// 添加 Claude 账户
#[tauri::command]
pub async fn claude_add(account: Account, state: State<'_, AppState>) -> Result<Account, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .save_account(&account)
        .await
        .map_err(|e| format!("Failed to save account: {}", e))?;

    Ok(account)
}

/// 更新 Claude 账户
#[tauri::command]
pub async fn claude_update(
    account: Account,
    state: State<'_, AppState>,
) -> Result<Account, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .update_account(&account)
        .await
        .map_err(|e| format!("Failed to update account: {}", e))?;

    Ok(account)
}

/// 删除 Claude 账户
#[tauri::command]
pub async fn claude_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .delete_account(&id)
        .await
        .map_err(|e| format!("Failed to delete account: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn claude_sync_accounts_to_database(
    state: State<'_, AppState>,
) -> Result<AccountSyncStatus, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .sync_local_to_remote()
        .await
        .map_err(|e| format!("Sync failed: {}", e))
}

#[tauri::command]
pub async fn claude_sync_accounts_from_database(
    state: State<'_, AppState>,
) -> Result<AccountSyncStatus, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .sync_remote_to_local()
        .await
        .map_err(|e| format!("Sync failed: {}", e))
}

#[tauri::command]
pub async fn claude_bidirectional_sync_accounts(
    state: State<'_, AppState>,
) -> Result<AccountSyncStatus, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .bidirectional_sync()
        .await
        .map_err(|e| format!("Sync failed: {}", e))
}

#[tauri::command]
pub async fn claude_sync_accounts(
    req_json: String,
    state: State<'_, AppState>,
) -> Result<ServerAccountSyncResponse<Account>, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    let req: ClientAccountSyncRequest<Account> = serde_json::from_str(&req_json)
        .map_err(|e| format!("Failed to parse sync request: {}", e))?;

    storage_manager
        .sync_accounts(req)
        .await
        .map_err(|e| format!("Sync failed: {}", e))
}

#[tauri::command]
pub async fn claude_get_sync_status(
    state: State<'_, AppState>,
) -> Result<Option<AccountSyncStatus>, String> {
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    storage_manager
        .get_sync_status()
        .await
        .map_err(|e| format!("Failed to get sync status: {}", e))
}

pub async fn initialize_claude_storage_manager(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let local_storage = Arc::new(ClaudeLocalStorage::new(app)?);

    let postgres_storage = {
        let db_manager_guard = state.database_manager.lock().unwrap();
        if let Some(db_manager) = db_manager_guard.as_ref() {
            Some(Arc::new(ClaudePostgreSQLStorage::new(db_manager.clone())))
        } else {
            None
        }
    };

    let dual_storage = Arc::new(ClaudeDualStorage::new(
        local_storage,
        postgres_storage,
        false,
    ));

    *state.claude_storage_manager.lock().unwrap() = Some(dual_storage);

    Ok(())
}

/// 切换 Claude 账户
/// 将选定的账户配置写入 ~/.claude/settings.json
#[tauri::command]
pub async fn claude_switch_account(
    app: tauri::AppHandle,
    account_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 获取存储管理器
    let storage_manager = {
        let guard = state.claude_storage_manager.lock().unwrap();
        guard
            .clone()
            .ok_or("Claude storage manager not initialized")?
    };

    // 获取所有账户
    let accounts = storage_manager
        .load_accounts()
        .await
        .map_err(|e| format!("Failed to load accounts: {}", e))?;

    // 查找目标账户
    let account = accounts
        .iter()
        .find(|a| a.id == account_id && !a.deleted)
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    // 构造 settings.json 内容
    let mut settings_json = serde_json::json!({
        "env": {
            "ANTHROPIC_AUTH_TOKEN": account.auth_token,
            "ANTHROPIC_BASE_URL": account.base_url,
            "ANTHROPIC_DEFAULT_OPUS_MODEL": account.default_opus_model,
            "ANTHROPIC_DEFAULT_SONNET_MODEL": account.default_sonnet_model,
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": account.default_haiku_model,
            "API_TIMEOUT_MS": "3000000",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1"
        }
    });

    // 如果 use_model 不是 default，则添加 model 项
    if account.use_model != "default" {
        settings_json["model"] = serde_json::json!(account.use_model);
    }

    let content = serde_json::to_string_pretty(&settings_json)
        .map_err(|e| format!("Failed to serialize settings.json: {}", e))?;

    // 获取用户主目录并构造 .claude 路径
    let home_dir = app
        .path()
        .home_dir()
        .map_err(|e| format!("Failed to get home directory: {}", e))?;

    let claude_dir = home_dir.join(".claude");
    let settings_file = claude_dir.join("settings.json");

    // 确保目录存在
    std::fs::create_dir_all(&claude_dir)
        .map_err(|e| format!("Failed to create .claude directory: {}", e))?;

    // 删除旧文件（如果存在）
    if settings_file.exists() {
        std::fs::remove_file(&settings_file)
            .map_err(|e| format!("Failed to remove old settings.json: {}", e))?;
    }

    // 写入新文件
    std::fs::write(&settings_file, content)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    println!(
        "Claude account switched: {} -> {}",
        account.service_name,
        settings_file.display()
    );

    Ok(())
}

/// 获取当前激活的 Claude 账户 ID（从 settings.json 读取）
#[tauri::command]
pub async fn claude_get_current_account_id(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let home_dir = app
        .path()
        .home_dir()
        .map_err(|e| format!("Failed to get home directory: {}", e))?;

    let settings_file = home_dir.join(".claude").join("settings.json");

    if !settings_file.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&settings_file)
        .map_err(|e| format!("Failed to read settings.json: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings.json: {}", e))?;

    // 获取当前配置中的 auth_token
    let current_token = json["env"]["ANTHROPIC_AUTH_TOKEN"]
        .as_str()
        .ok_or_else(|| "Missing ANTHROPIC_AUTH_TOKEN in settings.json".to_string())?;

    // 从存储管理器获取所有账户并查找匹配的
    let storage_manager = get_claude_storage_manager(&app, &state).await?;

    let accounts = storage_manager
        .load_accounts()
        .await
        .map_err(|e| format!("Failed to load accounts: {}", e))?;

    // 查找匹配的账户
    let current_account = accounts
        .iter()
        .find(|a| !a.deleted && a.auth_token == current_token);

    Ok(current_account.map(|a| a.id.clone()))
}

/// 将 Claude Code 直接切换到网关端点
/// 把网关 base_url / api_key 及 Opus/Sonnet/Haiku 三档模型写入 ~/.claude/settings.json（不依赖已存账号）
#[tauri::command]
pub async fn claude_switch_gateway(
    app: tauri::AppHandle,
    base_url: String,
    api_key: String,
    opus_model: String,
    sonnet_model: String,
    haiku_model: String,
) -> Result<(), String> {
    let opus = opus_model.trim();
    let sonnet = sonnet_model.trim();
    let haiku = haiku_model.trim();

    let mut env = serde_json::json!({
        "ANTHROPIC_AUTH_TOKEN": api_key,
        "ANTHROPIC_BASE_URL": base_url,
        "API_TIMEOUT_MS": "3000000",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1"
    });
    // 分档指定模型时写入对应默认模型，使各档请求经网关路由
    if !opus.is_empty() {
        env["ANTHROPIC_DEFAULT_OPUS_MODEL"] = serde_json::json!(opus);
    }
    if !sonnet.is_empty() {
        env["ANTHROPIC_DEFAULT_SONNET_MODEL"] = serde_json::json!(sonnet);
    }
    if !haiku.is_empty() {
        env["ANTHROPIC_DEFAULT_HAIKU_MODEL"] = serde_json::json!(haiku);
    }

    let settings_json = serde_json::json!({ "env": env });

    let content = serde_json::to_string_pretty(&settings_json)
        .map_err(|e| format!("Failed to serialize settings.json: {}", e))?;

    let home_dir = app
        .path()
        .home_dir()
        .map_err(|e| format!("Failed to get home directory: {}", e))?;

    let claude_dir = home_dir.join(".claude");
    let settings_file = claude_dir.join("settings.json");

    std::fs::create_dir_all(&claude_dir)
        .map_err(|e| format!("Failed to create .claude directory: {}", e))?;

    if settings_file.exists() {
        std::fs::remove_file(&settings_file)
            .map_err(|e| format!("Failed to remove old settings.json: {}", e))?;
    }

    std::fs::write(&settings_file, content)
        .map_err(|e| format!("Failed to write settings.json: {}", e))?;

    println!(
        "Claude switched to gateway: {} -> {}",
        base_url,
        settings_file.display()
    );

    Ok(())
}
