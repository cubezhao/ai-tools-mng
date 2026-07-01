pub mod platforms {
    pub mod antigravity;
    pub mod augment;
    pub mod claude;
    pub mod cursor;
    pub mod openai;
    pub mod windsurf;
}

pub mod features {
    pub mod mail;
    pub mod raindrop;
}

pub mod core {
    pub mod api_server;
    pub mod app_commands;
    pub mod gateway;
    pub mod gateway_channel;
    pub mod gateway_custom_models;
    pub mod gateway_models;
    pub mod http_client;
    pub mod path_manager;
    pub mod proxy_config;
    pub mod proxy_helper;
    pub mod spotlight;
    pub mod subscription_monitor;
    pub mod telegram;
    pub mod tray;
}

pub mod data {
    pub mod bookmark;
    pub mod database;
    pub mod storage;
    pub mod subscription;
}

// Backwards-compatible re-exports for existing module paths.
pub use core::{
    api_server, http_client, proxy_config, proxy_helper, subscription_monitor, telegram, tray,
};
pub use data::{database, storage};
pub use features::mail;
pub use platforms::{antigravity, augment, claude, cursor, openai, windsurf};

use crate::core::gateway::affinity::SessionAffinity;
use crate::core::gateway::config::GatewayConfig;
use crate::core::gateway::usage::GatewayUsageStore;
use crate::core::tray::TrayState;
use crate::data::bookmark::BookmarkDualStorage;
use crate::data::subscription::SubscriptionDualStorage;
use crate::features::mail::{
    gptmail, gptmail_storage::GptMailStorage, hme, hme_storage::HmeStorage, outlook,
    outlook_storage::OutlookStorage,
};
use crate::platforms::augment::models::AugmentOAuthState;
use crate::platforms::openai::codex::logger::RequestLogger;
use crate::platforms::openai::codex::pool::CodexServerConfig;
use crate::platforms::openai::codex::storage::CodexLogStorage;
use crate::platforms::openai::codex::{CodexExecutor, CodexPool, CodexServer};
use crate::platforms::openai::models::OpenAIOAuthSession;
use database::{DatabaseConfigManager, DatabaseManager};
use outlook::OutlookManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use storage::{
    AntigravityDualStorage, ClaudeDualStorage, CursorDualStorage, DualStorage, OpenAIDualStorage,
    WindsurfDualStorage,
};
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

// App Session 缓存结构 (公开以便其他模块使用)
#[derive(Clone)]
pub struct AppSessionCache {
    pub app_session: String,
    pub created_at: SystemTime,
}

// Global state to store OAuth state and storage managers
pub struct AppState {
    augment_oauth_state: Mutex<Option<AugmentOAuthState>>,
    pub openai_oauth_sessions: Arc<Mutex<HashMap<String, OpenAIOAuthSession>>>,
    api_server: Mutex<Option<api_server::ApiServer>>,
    outlook_manager: Mutex<OutlookManager>,
    pub outlook_storage: Arc<Mutex<Option<Arc<OutlookStorage>>>>,
    pub hme_cookie: Arc<Mutex<Option<String>>>,
    pub current_dsid: Arc<Mutex<Option<String>>>,
    pub hme_storage: Arc<Mutex<Option<Arc<HmeStorage>>>>,
    pub gptmail_storage: Arc<Mutex<Option<Arc<GptMailStorage>>>>,
    pub storage_manager: Arc<Mutex<Option<Arc<DualStorage>>>>,
    pub antigravity_storage_manager: Arc<Mutex<Option<Arc<AntigravityDualStorage>>>>,
    pub windsurf_storage_manager: Arc<Mutex<Option<Arc<WindsurfDualStorage>>>>,
    pub cursor_storage_manager: Arc<Mutex<Option<Arc<CursorDualStorage>>>>,
    pub openai_storage_manager: Arc<Mutex<Option<Arc<OpenAIDualStorage>>>>,
    pub subscription_storage_manager: Arc<Mutex<Option<Arc<SubscriptionDualStorage>>>>,
    pub bookmark_storage_manager: Arc<Mutex<Option<Arc<BookmarkDualStorage>>>>,
    pub claude_storage_manager: Arc<Mutex<Option<Arc<ClaudeDualStorage>>>>,
    pub database_manager: Arc<Mutex<Option<Arc<DatabaseManager>>>>,
    // App session 缓存: key 为 auth_session, value 为缓存的 app_session
    pub app_session_cache: Arc<Mutex<HashMap<String, AppSessionCache>>>,
    // App handle for emitting events
    pub app_handle: tauri::AppHandle,
    // Codex API 服务器相关
    pub codex_pool: Arc<Mutex<Option<Arc<CodexPool>>>>,
    pub codex_executor: Arc<Mutex<Option<Arc<CodexExecutor>>>>,
    pub codex_logger: Arc<Mutex<Option<Arc<tokio::sync::RwLock<RequestLogger>>>>>,
    codex_server: Arc<Mutex<Option<CodexServer>>>,
    pub codex_unsupported_params:
        Arc<crate::platforms::openai::codex::server::UnsupportedParamCache>,
    pub codex_server_config: Arc<Mutex<Option<CodexServerConfig>>>,
    pub codex_log_storage: Arc<Mutex<Option<Arc<CodexLogStorage>>>>,
    pub proxy_config: Arc<Mutex<Option<crate::core::proxy_config::ProxyConfig>>>,
    // API 网关（复用 8766，/gateway 前缀）
    pub gateway_config: Arc<Mutex<GatewayConfig>>,
    pub gateway_usage: Arc<GatewayUsageStore>,
    /// 网关会话粘性（sticky session）映射
    pub gateway_affinity: Arc<SessionAffinity>,
    /// 网关流式上游客户端缓存：构建一次后复用，避免每请求重建客户端与重复 TLS 握手
    pub gateway_stream_client: Arc<Mutex<Option<crate::core::proxy_helper::ProxyClient>>>,
    /// Outlook OAuth: CSRF state + PKCE code_verifier (see RFC 7636)
    pub outlook_oauth_pending: Mutex<Option<outlook::OutlookOAuthPending>>,
}

impl AppState {
    /// 取网关流式上游客户端：首次按当前代理配置构建并缓存，后续请求直接复用以保持连接池
    pub fn gateway_http_client(&self) -> Result<crate::core::proxy_helper::ProxyClient, String> {
        let mut guard = self
            .gateway_stream_client
            .lock()
            .map_err(|_| "网关客户端缓存锁中毒".to_string())?;
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }
        let client = crate::core::http_client::create_proxy_client_for_streaming()?;
        *guard = Some(client.clone());
        Ok(client)
    }

    /// 失效网关客户端缓存（代理配置变更后调用），下次请求按新配置重建
    pub fn invalidate_gateway_http_client(&self) {
        if let Ok(mut guard) = self.gateway_stream_client.lock() {
            *guard = None;
        }
    }
}

pub fn run() {
    let mut builder = tauri::Builder::default();

    // 在桌面平台上，single-instance 插件必须是第一个注册的插件
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            eprintln!("A new app instance was opened with argv: {:?}", argv);
            eprintln!("Deep link event was already triggered by the plugin");

            // 聚焦主窗口
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.set_focus();
                let _ = main_window.unminimize();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.handle().path().app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let app_state = AppState {
                augment_oauth_state: Mutex::new(None),
                openai_oauth_sessions: Arc::new(Mutex::new(HashMap::new())),
                api_server: Mutex::new(None),
                outlook_manager: Mutex::new(OutlookManager::new()),
                outlook_storage: Arc::new(Mutex::new(
                    OutlookStorage::new(app_data_dir.clone())
                        .map(Arc::new)
                        .ok(),
                )),
                hme_cookie: Arc::new(Mutex::new(None)),
                current_dsid: Arc::new(Mutex::new(None)),
                hme_storage: Arc::new(Mutex::new(
                    HmeStorage::new(app_data_dir.clone())
                        .map(Arc::new)
                        .ok(),
                )),
                gptmail_storage: Arc::new(Mutex::new(
                    GptMailStorage::new(app_data_dir.clone())
                        .map(Arc::new)
                        .ok(),
                )),
                storage_manager: Arc::new(Mutex::new(None)),
                antigravity_storage_manager: Arc::new(Mutex::new(None)),
                windsurf_storage_manager: Arc::new(Mutex::new(None)),
                cursor_storage_manager: Arc::new(Mutex::new(None)),
                openai_storage_manager: Arc::new(Mutex::new(None)),
                subscription_storage_manager: Arc::new(Mutex::new(None)),
                bookmark_storage_manager: Arc::new(Mutex::new(None)),
                claude_storage_manager: Arc::new(Mutex::new(None)),
                database_manager: Arc::new(Mutex::new(None)),
                app_session_cache: Arc::new(Mutex::new(HashMap::new())),
                app_handle: app.handle().clone(),
                codex_pool: Arc::new(Mutex::new(None)),
                codex_executor: Arc::new(Mutex::new(None)),
                codex_logger: Arc::new(Mutex::new(None)),
                codex_server: Arc::new(Mutex::new(None)),
                codex_unsupported_params: Arc::new(
                    crate::platforms::openai::codex::server::UnsupportedParamCache::load(&app_data_dir),
                ),
                codex_server_config: Arc::new(Mutex::new(None)),
                codex_log_storage: Arc::new(Mutex::new(None)),
                proxy_config: Arc::new(Mutex::new(None)),
                gateway_config: Arc::new(Mutex::new(GatewayConfig::load(&app_data_dir))),
                gateway_usage: Arc::new(GatewayUsageStore::load(
                    app_data_dir.join("gateway_usage.db"),
                )),
                gateway_affinity: Arc::new(SessionAffinity::new()),
                gateway_stream_client: Arc::new(Mutex::new(None)),
                outlook_oauth_pending: Mutex::new(None),
            };

            app.manage(app_state);

            // 初始化托盘状态管理器
            app.manage(TrayState::new());

            // Outlook Token 定时刷新（每 24 小时）
            {
                let app_handle_refresh = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 首次延迟 60 秒，等待应用完成初始化
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    loop {
                        let state = app_handle_refresh.state::<AppState>();
                        outlook::scheduled_refresh_tokens(state.inner()).await;
                        // 每 24 小时刷新一次
                        tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
                    }
                });
            }

            // 异步初始化存储管理器
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();

                // 尝试加载数据库配置
                match DatabaseConfigManager::new(&app_handle) {
                    Ok(config_manager) => {
                        match config_manager.load_config() {
                            Ok(config) => {
                                if config.enabled {
                                    let mut db_manager = DatabaseManager::new(config);
                                    if db_manager.initialize().await.is_ok() {
                                        // 检查表是否存在
                                        if let Some(pool) = db_manager.get_pool() {
                                            match pool.get().await {
                                                Ok(client) => {
                                                        match database::augment::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    // 创建表
                                                                    if let Err(e) = database::augment::create_tables(&client).await {
                                                                        eprintln!("Failed to create tables on startup: {}", e);
                                                                    }
                                                                } else {
                                                                    // 表已存在，检查并添加新字段
                                                                    if let Err(e) = database::augment::add_new_fields_if_not_exist(&client).await {
                                                                        eprintln!("Failed to add new fields on startup: {}", e);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check tables on startup: {}", e);
                                                            }
                                                        }
                                                        match database::antigravity::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = database::antigravity::create_tables(&client).await {
                                                                        eprintln!("Failed to create Antigravity tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = database::antigravity::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to add new fields to Antigravity tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Antigravity tables on startup: {}", e);
                                                            }
                                                        }
                                                        match database::windsurf::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = database::windsurf::create_tables(&client).await {
                                                                        eprintln!("Failed to create Windsurf tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = database::windsurf::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update Windsurf tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Windsurf tables on startup: {}", e);
                                                            }
                                                        }
                                                        match database::cursor::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = database::cursor::create_tables(&client).await {
                                                                        eprintln!("Failed to create Cursor tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = database::cursor::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update Cursor tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Cursor tables on startup: {}", e);
                                                            }
                                                        }
                                                        match database::openai::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = database::openai::create_tables(&client).await {
                                                                        eprintln!("Failed to create OpenAI tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = database::openai::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update OpenAI tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check OpenAI tables on startup: {}", e);
                                                            }
                                                        }
                                                        match database::claude::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = database::claude::create_tables(&client).await {
                                                                        eprintln!("Failed to create Claude tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = database::claude::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update Claude tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Claude tables on startup: {}", e);
                                                            }
                                                        }
                                                        match crate::data::subscription::migrations::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = crate::data::subscription::migrations::create_tables(&client).await {
                                                                        eprintln!("Failed to create Subscription tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = crate::data::subscription::migrations::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update Subscription tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Subscription tables on startup: {}", e);
                                                            }
                                                        }
                                                        match crate::data::bookmark::migrations::check_tables_exist(&client).await {
                                                            Ok(exists) => {
                                                                if !exists {
                                                                    if let Err(e) = crate::data::bookmark::migrations::create_tables(&client).await {
                                                                        eprintln!("Failed to create Bookmark tables on startup: {}", e);
                                                                    }
                                                                } else if let Err(e) = crate::data::bookmark::migrations::add_new_fields_if_not_exist(&client).await {
                                                                    eprintln!("Failed to update Bookmark tables on startup: {}", e);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Failed to check Bookmark tables on startup: {}", e);
                                                            }
                                                        }
                                                }
                                                Err(e) => {
                                                    eprintln!("Failed to get database client on startup: {}", e);
                                                }
                                            }
                                        }

                                        *state.database_manager.lock().unwrap() = Some(Arc::new(db_manager));
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to load database config: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to create config manager: {}", e),
                }

                // 初始化存储管理器
                if let Err(e) = storage::initialize_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize storage manager: {}", e);
                }
                if let Err(e) = storage::initialize_antigravity_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Antigravity storage manager: {}", e);
                }
                if let Err(e) = storage::initialize_windsurf_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Windsurf storage manager: {}", e);
                }
                if let Err(e) = storage::initialize_cursor_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Cursor storage manager: {}", e);
                }
                if let Err(e) = storage::initialize_openai_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize OpenAI storage manager: {}", e);
                }
                if let Err(e) = storage::initialize_claude_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Claude storage manager: {}", e);
                }
                if let Err(e) = crate::data::subscription::initialize_subscription_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Subscription storage manager: {}", e);
                }
                if let Err(e) = crate::data::bookmark::initialize_bookmark_storage_manager(&app_handle, &state).await {
                    eprintln!("Failed to initialize Bookmark storage manager: {}", e);
                }

                // 初始化 Codex 日志存储
                if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                    match CodexLogStorage::new(app_data_dir.to_path_buf()) {
                        Ok(storage) => {
                            *state.codex_log_storage.lock().unwrap() = Some(Arc::new(storage));
                            println!("✅ Codex log storage initialized successfully");
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to initialize Codex log storage: {}", e);
                        }
                    }
                }

                // 启动 Codex 日志定期刷新任务
            });

            // 启动 API 服务器
            let app_handle_for_api = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle_for_api.state::<AppState>();

                // 等待存储管理器初始化完成
                let mut attempts = 0;
                while attempts < 50 {
                    let storage_ready = {
                        let storage_guard = state.storage_manager.lock().unwrap();
                        storage_guard.is_some()
                    };

                    if storage_ready {
                        break;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    attempts += 1;
                }

                // 启动 API 服务器
                match api_server::start_api_server(
                    Arc::new(AppState {
                        augment_oauth_state: Mutex::new(None),
                        openai_oauth_sessions: state.openai_oauth_sessions.clone(),
                        api_server: Mutex::new(None),
                        outlook_manager: Mutex::new(OutlookManager::new()),
                        outlook_storage: state.outlook_storage.clone(),
                        hme_cookie: state.hme_cookie.clone(),
                        current_dsid: state.current_dsid.clone(),
                        hme_storage: state.hme_storage.clone(),
                        gptmail_storage: state.gptmail_storage.clone(),
                        storage_manager: state.storage_manager.clone(),
                        antigravity_storage_manager: state.antigravity_storage_manager.clone(),
                        windsurf_storage_manager: state.windsurf_storage_manager.clone(),
                        cursor_storage_manager: state.cursor_storage_manager.clone(),
                        openai_storage_manager: state.openai_storage_manager.clone(),
                        claude_storage_manager: state.claude_storage_manager.clone(),
                        subscription_storage_manager: state.subscription_storage_manager.clone(),
                        bookmark_storage_manager: state.bookmark_storage_manager.clone(),
                        database_manager: state.database_manager.clone(),
                        app_session_cache: state.app_session_cache.clone(),
                        app_handle: app_handle_for_api.clone(),
                        codex_pool: state.codex_pool.clone(),
                        codex_executor: state.codex_executor.clone(),
                        codex_logger: state.codex_logger.clone(),
                        codex_server: state.codex_server.clone(),
                        codex_unsupported_params: state.codex_unsupported_params.clone(),
                        codex_server_config: state.codex_server_config.clone(),
                        codex_log_storage: state.codex_log_storage.clone(),
                        proxy_config: state.proxy_config.clone(),
                        gateway_config: state.gateway_config.clone(),
                        gateway_usage: state.gateway_usage.clone(),
                        gateway_affinity: state.gateway_affinity.clone(),
                        gateway_stream_client: state.gateway_stream_client.clone(),
                        outlook_oauth_pending: Mutex::new(None),
                    }),
                    8766,
                ).await {
                    Ok(server) => {
                        println!("✅ API Server initialized successfully");
                        *state.api_server.lock().unwrap() = Some(server);
                        if let Err(err) =
                            crate::platforms::openai::codex::commands::init_codex_enabled_state_on_startup(
                                &app_handle_for_api,
                                state.inner(),
                            )
                            .await
                        {
                            eprintln!("❌ Failed to initialize Codex enabled state: {}", err);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to start API server: {}", e);
                    }
                }
            });

            // 启动订阅到期监控定时任务
            let app_handle_for_monitor = app.handle().clone();
            subscription_monitor::start_subscription_monitor(app_handle_for_monitor);

            // 设置 deep-link 处理器
            let app_handle_for_deep_link = app.app_handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                eprintln!("Deep link received: {:?}", urls);

                for parsed_url in urls {
                    let url_str = parsed_url.as_str();

                    // 检查是否是 atm://import 协议
                    if url_str.starts_with("atm://import") {
                        eprintln!("Processing ATM import deep link: {}", url_str);

                        // 查找 session 参数
                        if let Some(session) = parsed_url
                            .query_pairs()
                            .find(|(key, _)| key == "session")
                            .map(|(_, value)| value.to_string())
                        {
                            eprintln!("Found session parameter in deep link");

                            // 克隆需要的变量
                            let app_handle = app_handle_for_deep_link.clone();

                            // 在异步任务中等待窗口就绪并发送 session 到前端
                            tauri::async_runtime::spawn(async move {
                                // 等待主窗口加载完成（最多等待 10 秒）
                                let mut attempts = 0;
                                let max_attempts = 100; // 100 * 100ms = 10 秒
                                while attempts < max_attempts {
                                    if let Some(main_window) = app_handle.get_webview_window("main") {
                                        // 窗口存在，再等待一小段时间确保前端事件监听器已注册
                                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

                                        // 聚焦主窗口
                                        let _ = main_window.set_focus();
                                        let _ = main_window.unminimize();

                                        // 发送 session 到前端，由前端调用导入方法
                                        let _ = app_handle.emit(
                                            "deep-link-session-received",
                                            serde_json::json!({
                                                "session": session
                                            })
                                        );

                                        break;
                                    }
                                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                    attempts += 1;
                                }
                            });
                        } else {
                            eprintln!("No session parameter found in deep link URL");
                        }
                    }
                }
            });

            // 在 Linux 和 Windows 上注册协议
            // Linux: 总是注册（AppImage 需要运行时注册）
            // Windows: 总是注册（确保协议在所有模式下都可用）
            // macOS: 不支持运行时注册，必须通过 bundle 配置
            #[cfg(any(target_os = "linux", windows))]
            {
                if let Err(e) = app.deep_link().register_all() {
                    eprintln!("Failed to register deep link protocols: {}", e);
                } else {
                    eprintln!("Successfully registered deep link protocols");
                }
            }

            // 监听主窗口关闭事件，实现最小化到托盘功能
            if let Some(main_window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 检查托盘是否启用
                        let tray_state = app_handle.state::<TrayState>();
                        let tray_enabled = tray_state.tray_icon.lock()
                            .map(|guard| guard.is_some())
                            .unwrap_or(false);

                        if tray_enabled {
                            // 托盘已启用，阻止关闭并隐藏窗口
                            api.prevent_close();
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        // 托盘未启用时，允许正常关闭（退出应用）
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Augment 命令
            augment::generate_augment_auth_url,
            augment::get_augment_token,
            augment::batch_check_tokens_status,
            augment::batch_check_tokens_status_simple,
            augment::fetch_batch_credit_consumption,
            augment::fetch_payment_method_link_command,
            augment::add_token_from_session,
            augment::batch_refresh_sessions,
            // Augment 团队管理命令
            augment::fetch_team_info,
            augment::update_team_seats,
            augment::invite_team_members,
            augment::delete_team_member,
            augment::delete_team_invitation,

            // OpenAI OAuth 命令
            openai::openai_generate_auth_url,
            openai::openai_exchange_code,
            openai::openai_refresh_token,
            openai::openai_list_accounts,
            openai::openai_load_account,
            openai::openai_save_account,
            openai::openai_delete_account,
            openai::openai_get_current_account_id,
            openai::openai_set_current_account_id,
            openai::openai_fetch_quota,
            openai::openai_consume_reset_credit,
            openai::openai_refresh_all_quotas,
            openai::openai_refresh_account_token,
            openai::openai_refresh_all_tokens,
            openai::openai_load_accounts_json,
            openai::openai_add_account,
            openai::openai_add_account_with_access_token,
            openai::openai_import_account_direct,
            openai::openai_add_api_account,
            openai::openai_update_api_account,
            openai::openai_save_accounts,
            openai::openai_update_account,
            openai::openai_start_oauth_login,
            openai::openai_cancel_oauth_login,
            openai::openai_switch_account,
            openai::codex_switch_account,
            openai::droid_switch_account,

            // Antigravity 管理命令
            antigravity::antigravity_list_accounts,
            antigravity::antigravity_load_accounts_json,
            antigravity::antigravity_add_account,
            antigravity::antigravity_update_account,
            antigravity::antigravity_delete_account,
            antigravity::antigravity_get_current_account,
            antigravity::antigravity_fetch_quota,
            antigravity::antigravity_refresh_all_quotas,
            antigravity::antigravity_switch_account,
            antigravity::antigravity_check_installation,
            antigravity::antigravity_is_running,
            antigravity::antigravity_launch,
            antigravity::antigravity_get_auth_url,
            antigravity::antigravity_exchange_code,
            antigravity::antigravity_start_oauth_login,
            antigravity::antigravity_cancel_oauth_login,
            antigravity::antigravity_get_custom_path,
            antigravity::antigravity_set_custom_path,
            antigravity::antigravity_validate_path,
            antigravity::antigravity_get_default_path,
            antigravity::antigravity_select_executable_path,

            // Windsurf 管理命令
            windsurf::windsurf_login,
            windsurf::windsurf_add_account,
            windsurf::windsurf_add_account_by_devin_auth1,
            windsurf::windsurf_add_account_by_devin_session,
            windsurf::windsurf_list_accounts,
            windsurf::windsurf_update_account,
            windsurf::windsurf_delete_account,
            windsurf::windsurf_switch_account,
            windsurf::windsurf_switch_account_seamless,
            windsurf::windsurf_fetch_quota,
            windsurf::windsurf_fetch_all_quotas,
            windsurf::windsurf_get_seamless_patch_status,
            windsurf::windsurf_apply_seamless_patch,
            windsurf::windsurf_restore_seamless_patch,
            windsurf::windsurf_get_custom_path,
            windsurf::windsurf_set_custom_path,
            windsurf::windsurf_validate_path,
            windsurf::windsurf_get_default_path,
            windsurf::windsurf_select_executable_path,
            // Windsurf 同步命令
            data::storage::windsurf::windsurf_sync_accounts_to_database,
            data::storage::windsurf::windsurf_sync_accounts_from_database,
            data::storage::windsurf::windsurf_bidirectional_sync_accounts,
            data::storage::windsurf::windsurf_sync_accounts,

            // Cursor 管理命令
            cursor::cursor_get_user_info_from_session,
            cursor::cursor_refresh_account_tokens,
            cursor::cursor_add_account_with_session,
            cursor::cursor_add_account_with_access_token,
            cursor::cursor_import_accounts,
            cursor::cursor_update_account,
            cursor::cursor_list_accounts,
            cursor::cursor_delete_account,
            cursor::cursor_switch_account,
            cursor::cursor_generate_and_bind_machine_id,
            cursor::cursor_export_accounts,
            cursor::cursor_get_custom_path,
            cursor::cursor_set_custom_path,
            cursor::cursor_validate_path,
            cursor::cursor_get_default_path,
            cursor::cursor_select_executable_path,
            cursor::cursor_validate_account,
            cursor::cursor_get_usage_summary,
            cursor::cursor_get_aggregated_usage,
            cursor::cursor_get_filtered_usage_events,
            cursor::cursor_check_auto_update_disabled,
            cursor::cursor_disable_auto_update,
            cursor::cursor_enable_auto_update,
            cursor::cursor_check_main_js_permission,

            // 书签同步命令
            data::bookmark::bookmark_sync_accounts,
            data::bookmark::bookmark_sync_to_database,
            data::bookmark::bookmark_sync_from_database,
            data::bookmark::bookmark_bidirectional_sync,
            // 书签 CRUD 命令
            data::bookmark::bookmark_load_local,
            data::bookmark::bookmark_list,
            data::bookmark::bookmark_add,
            data::bookmark::bookmark_update,
            data::bookmark::bookmark_delete,
            data::bookmark::bookmark_import_from_browser,

            // 核心应用命令
            core::app_commands::open_url,
            core::app_commands::open_data_folder,
            core::app_commands::open_editor_with_protocol,
            core::app_commands::create_jetbrains_token_file,
            core::app_commands::configure_vim_augment,
            core::app_commands::configure_auggie,
            core::app_commands::open_internal_browser,
            core::app_commands::check_for_updates,
            core::app_commands::copy_to_clipboard,

            // 存储命令
            storage::save_tokens_json,
            storage::load_tokens_json,
            storage::bidirectional_sync_tokens,
            storage::sync_tokens,
            storage::get_storage_status,
            storage::get_sync_status,
            storage::antigravity::antigravity_sync_accounts_to_database,
            storage::antigravity::antigravity_sync_accounts_from_database,
            storage::antigravity::antigravity_bidirectional_sync_accounts,
            storage::antigravity::antigravity_sync_accounts,
            storage::antigravity::antigravity_get_sync_status,
            // OpenAI 同步命令
            storage::openai::openai_sync_accounts_to_database,
            storage::openai::openai_sync_accounts_from_database,
            storage::openai::openai_bidirectional_sync_accounts,
            storage::openai::openai_sync_accounts,
            storage::openai::openai_get_sync_status,
            // Cursor 同步命令
            storage::cursor::cursor_sync_accounts_to_database,
            storage::cursor::cursor_sync_accounts_from_database,
            storage::cursor::cursor_bidirectional_sync_accounts,
            storage::cursor::cursor_sync_accounts,
            storage::cursor::cursor_get_sync_status,

            // 订阅同步命令
            data::subscription::subscription_sync_accounts,
            data::subscription::subscription_sync_to_database,
            data::subscription::subscription_sync_from_database,
            data::subscription::subscription_bidirectional_sync,
            // 订阅 CRUD 命令
            data::subscription::subscription_load_local,
            data::subscription::subscription_list,
            data::subscription::subscription_add,
            data::subscription::subscription_update,
            data::subscription::subscription_delete,

            // Claude 账户管理命令
            storage::claude::claude_load_accounts_local,
            storage::claude::claude_list,
            storage::claude::claude_add,
            storage::claude::claude_update,
            storage::claude::claude_delete,
            storage::claude::claude_sync_accounts,
            storage::claude::claude_sync_accounts_to_database,
            storage::claude::claude_sync_accounts_from_database,
            storage::claude::claude_bidirectional_sync_accounts,
            storage::claude::claude_switch_account,
            storage::claude::claude_switch_gateway,
            storage::claude::claude_get_current_account_id,

            // API 服务器管理命令
            api_server::get_api_server_status,
            api_server::start_api_server_cmd,
            api_server::stop_api_server,

            // Outlook 邮箱管理命令
            outlook::outlook_save_credentials,
            outlook::outlook_get_all_accounts_info,
            outlook::outlook_get_account_statuses,
            outlook::outlook_update_tag,
            outlook::outlook_delete_account,
            outlook::outlook_check_account_status,
            outlook::outlook_get_emails,
            outlook::outlook_get_email_details,
            outlook::outlook_refresh_all_tokens,
            outlook::outlook_oauth_available,
            outlook::outlook_get_oauth_auth_url,
            outlook::outlook_exchange_oauth_token,
            outlook::outlook_delete_emails,

            // GPTMail 管理命令
            gptmail::generate_random_email,
            gptmail::get_emails,
            gptmail::gptmail_list_history,
            gptmail::gptmail_save_email,
            gptmail::gptmail_update_email,
            gptmail::gptmail_delete_emails,
            gptmail::gptmail_update_tag,

            // iCloud HME 管理命令
            hme::hme_set_cookie,
            hme::hme_get_cookie,
            hme::hme_clear_cookie,
            hme::hme_has_cookie,
            hme::hme_validate_cookie,
            hme::hme_get_current_dsid,
            hme::hme_generate,
            hme::hme_list,
            hme::hme_list_local,
            hme::hme_sync,
            hme::hme_deactivate,
            hme::hme_delete,
            hme::hme_cleanup,
            hme::hme_update_tag,

            // 数据库配置命令
            database::save_database_config,
            database::load_database_config,
            database::test_database_connection_cmd,
            database::delete_database_config,

            // 代理配置命令
            proxy_config::save_proxy_config,
            proxy_config::load_proxy_config,
            proxy_config::test_proxy_config,
            proxy_config::delete_proxy_config,
            proxy_config::proxy_config_exists,

            // Raindrop 书签同步命令
            features::raindrop::raindrop_save_config,
            features::raindrop::raindrop_load_config,
            features::raindrop::raindrop_delete_config,
            features::raindrop::raindrop_sync,
            features::raindrop::raindrop_full_sync,
            features::raindrop::raindrop_validate_token,

            // Spotlight 快捷搜索命令
            crate::core::spotlight::toggle_spotlight,
            crate::core::spotlight::register_spotlight_shortcut,
            crate::core::spotlight::unregister_spotlight_shortcut,
            crate::core::spotlight::is_spotlight_shortcut_registered,

            // 系统托盘命令
            tray::create_tray,
            tray::destroy_tray,
            tray::toggle_tray,
            tray::get_tray_status,

            // Telegram 通知命令
            telegram::save_telegram_config,
            telegram::load_telegram_config,
            telegram::delete_telegram_config,
            telegram::test_telegram_connection_cmd,
            telegram::send_telegram_message_cmd,

            // Codex API 管理命令
            crate::platforms::openai::codex::commands::get_codex_server_status,
            crate::platforms::openai::codex::commands::start_codex_server,
            crate::platforms::openai::codex::commands::stop_codex_server,
            crate::platforms::openai::codex::commands::get_codex_pool_status,
            crate::platforms::openai::codex::commands::refresh_codex_pool,
            crate::platforms::openai::codex::commands::get_codex_logs,
            crate::platforms::openai::codex::commands::query_codex_logs,
            crate::platforms::openai::codex::commands::get_codex_stats,
            crate::platforms::openai::codex::commands::get_codex_period_stats,
            crate::platforms::openai::codex::commands::get_codex_model_stats,
            crate::platforms::openai::codex::commands::clear_codex_logs,
            crate::platforms::openai::codex::commands::set_codex_pool_strategy,
            crate::platforms::openai::codex::commands::set_codex_selected_account,
            crate::platforms::openai::codex::commands::get_codex_access_config,
            crate::platforms::openai::codex::commands::set_codex_access_config,
            crate::platforms::openai::codex::commands::get_codex_runtime_settings,
            crate::platforms::openai::codex::commands::set_codex_runtime_settings,
            // Codex 日志存储命令
            crate::platforms::openai::codex::commands::query_codex_logs_from_storage,
            crate::platforms::openai::codex::commands::get_codex_model_stats_from_storage,
            crate::platforms::openai::codex::commands::get_codex_period_stats_from_storage,
            crate::platforms::openai::codex::commands::get_codex_daily_stats_from_storage,
            crate::platforms::openai::codex::commands::clear_codex_logs_in_storage,
            crate::platforms::openai::codex::commands::delete_codex_logs_before,
            crate::platforms::openai::codex::commands::get_codex_log_storage_status,
            crate::platforms::openai::codex::commands::flush_codex_logs,
            crate::platforms::openai::codex::commands::get_codex_all_time_stats,

            // 订阅监控命令
            subscription_monitor::check_subscriptions_expiry,
            subscription_monitor::get_expiring_subscriptions,

            // 网关模型库命令
            crate::core::gateway_models::gateway_sync_models,
            crate::core::gateway_models::gateway_get_models,
            crate::core::gateway_custom_models::gateway_get_custom_models,
            crate::core::gateway_custom_models::gateway_set_custom_models,
            crate::core::gateway_channel::gateway_fetch_channel_models,
            crate::core::gateway_channel::gateway_test_channel,
            // 网关核心命令
            crate::core::gateway::commands::gateway_get_config,
            crate::core::gateway::commands::gateway_set_config,
            crate::core::gateway::commands::gateway_get_status,
            crate::core::gateway::commands::gateway_start,
            crate::core::gateway::commands::gateway_stop,
            crate::core::gateway::commands::gateway_list_usage,
            crate::core::gateway::commands::gateway_clear_usage,
            crate::core::gateway::commands::gateway_list_bindable_accounts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
