use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    System,
    Http,
    Https,
    Socks5,
    #[serde(rename = "custom_url")]
    CustomUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub custom_url: Option<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            enabled: false,
            proxy_type: ProxyType::System,
            host: String::new(),
            port: 7890,
            username: None,
            password: None,
            custom_url: None,
        }
    }
}

impl ProxyConfig {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// 构建代理URL
    pub fn build_proxy_url(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }

        match self.proxy_type {
            ProxyType::System => None, // 系统代理由 reqwest 自动处理
            ProxyType::CustomUrl => {
                // 直接返回自定义 URL
                self.custom_url.clone()
            }
            ProxyType::Http | ProxyType::Https | ProxyType::Socks5 => {
                let protocol = match self.proxy_type {
                    ProxyType::Http => "http",
                    ProxyType::Https => "https",
                    ProxyType::Socks5 => "socks5",
                    _ => return None,
                };

                let auth =
                    if let (Some(username), Some(password)) = (&self.username, &self.password) {
                        format!("{}:{}@", username, password)
                    } else {
                        String::new()
                    };

                Some(format!(
                    "{}://{}{}:{}",
                    protocol, auth, self.host, self.port
                ))
            }
        }
    }

    /// 创建配置了代理的 reqwest 客户端
    pub fn create_client(&self) -> Result<reqwest::Client, String> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(15));

        if self.enabled {
            match self.proxy_type {
                ProxyType::CustomUrl => {
                    // CustomUrl (如 Supabase Edge Functions) 不是传统代理
                    // 显式禁用所有代理,确保直接连接到 Edge Function
                    // 这样可以避免系统代理干扰
                    builder = builder.no_proxy();
                }
                _ => {
                    // 构建传统代理 URL
                    if let Some(proxy_url) = self.build_proxy_url() {
                        let proxy = reqwest::Proxy::all(&proxy_url)
                            .map_err(|e| format!("Failed to create proxy: {}", e))?;
                        builder = builder.proxy(proxy);
                    }
                    // 如果是 System 类型,build_proxy_url() 返回 None
                    // 不配置代理,让 reqwest 使用默认行为 (尝试系统代理)
                }
            }
        }
        // 如果 enabled = false,也不调用 no_proxy()
        // 让 reqwest 使用默认行为 (尝试系统代理)

        builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))
    }

    /// 创建带 Cookie 支持的客户端
    pub fn create_client_with_cookies(
        &self,
        jar: std::sync::Arc<reqwest::cookie::Jar>,
    ) -> Result<reqwest::Client, String> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .cookie_provider(jar)
            .redirect(reqwest::redirect::Policy::limited(10));

        if self.enabled {
            match self.proxy_type {
                ProxyType::CustomUrl => {
                    // CustomUrl (如 Supabase Edge Functions) 不是传统代理
                    // 显式禁用所有代理,确保直接连接到 Edge Function
                    // 这样可以避免系统代理干扰
                    builder = builder.no_proxy();
                }
                _ => {
                    // 构建传统代理 URL
                    if let Some(proxy_url) = self.build_proxy_url() {
                        let proxy = reqwest::Proxy::all(&proxy_url)
                            .map_err(|e| format!("Failed to create proxy: {}", e))?;
                        builder = builder.proxy(proxy);
                    }
                    // 如果是 System 类型,build_proxy_url() 返回 None
                    // 不配置代理,让 reqwest 使用默认行为 (尝试系统代理)
                }
            }
        }
        // 如果 enabled = false,也不调用 no_proxy()
        // 让 reqwest 使用默认行为 (尝试系统代理)

        builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))
    }
}

#[allow(dead_code)]
pub struct ProxyConfigManager {
    app: tauri::AppHandle,
}

#[allow(clippy::module_name_repetitions)]
impl ProxyConfigManager {
    #[allow(dead_code)]
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        Ok(Self { app: app.clone() })
    }

    #[allow(dead_code)]
    pub fn save_config(&self, config: &ProxyConfig) -> Result<(), String> {
        let path = self.get_config_path()?;
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize proxy config: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write proxy config: {}", e))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_config(&self) -> Result<ProxyConfig, String> {
        let path = self.get_config_path()?;
        if path.exists() {
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read proxy config: {}", e))?;

            let config: ProxyConfig = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse proxy config: {}", e))?;

            Ok(config)
        } else {
            Ok(ProxyConfig::default())
        }
    }

    #[allow(dead_code)]
    pub fn delete_config(&self) -> Result<(), String> {
        let path = self.get_config_path()?;
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete proxy config: {}", e))?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn config_exists(&self) -> bool {
        let path = match self.get_config_path() {
            Ok(p) => p,
            Err(_) => return false,
        };

        path.exists()
    }

    fn get_config_path(&self) -> Result<PathBuf, String> {
        let app_data_dir = self
            .app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;

        // 确保目录存在
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;

        Ok(app_data_dir.join("proxy_config.json"))
    }
}

/// 获取代理配置文件路径
fn get_proxy_config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // 确保目录存在
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir.join("proxy_config.json"))
}

/// 保存代理配置到文件
pub fn save_proxy_config_internal(
    app_handle: &tauri::AppHandle,
    config: &ProxyConfig,
) -> Result<(), String> {
    let config_path = get_proxy_config_path(app_handle)?;

    // 如果配置文件已存在且新密码为空，则保留原有密码
    let mut final_config = config.clone();
    if config_path.exists() && config.password.as_ref().map_or(true, |p| p.is_empty()) {
        // 尝试加载现有配置以获取原密码
        if let Ok(existing_json) = fs::read_to_string(&config_path) {
            if let Ok(existing_config) = serde_json::from_str::<ProxyConfig>(&existing_json) {
                // 如果新密码为空但旧密码不为空，使用旧密码
                if config.password.as_ref().map_or(true, |p| p.is_empty())
                    && existing_config.password.is_some()
                {
                    final_config.password = existing_config.password;
                }
            }
        }
    }

    // 序列化配置
    let json = serde_json::to_string_pretty(&final_config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // 写入文件
    fs::write(&config_path, json).map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// 从文件加载代理配置
pub fn load_proxy_config_internal(app_handle: &tauri::AppHandle) -> Result<ProxyConfig, String> {
    let config_path = get_proxy_config_path(app_handle)?;

    // 如果文件不存在，返回默认配置
    if !config_path.exists() {
        return Ok(ProxyConfig::default());
    }

    // 读取文件
    let json = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // 反序列化配置
    let config: ProxyConfig =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// 删除代理配置文件
pub fn delete_proxy_config_internal(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let config_path = get_proxy_config_path(app_handle)?;

    // 如果文件存在，删除它
    if config_path.exists() {
        fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to delete config file: {}", e))?;
    }

    Ok(())
}

/// 检查代理配置文件是否存在
pub fn proxy_config_exists_internal(app_handle: &tauri::AppHandle) -> Result<bool, String> {
    let config_path = get_proxy_config_path(app_handle)?;
    Ok(config_path.exists())
}

pub async fn test_proxy_connection(config: &ProxyConfig) -> Result<(), String> {
    // 对于 CustomUrl 类型（如 Supabase Edge Functions），需要特殊处理
    if config.enabled && config.proxy_type == ProxyType::CustomUrl {
        if let Some(custom_url) = &config.custom_url {
            // 创建普通客户端（显式禁用代理,避免系统代理干扰）
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .no_proxy() // 显式禁用所有代理
                .build()
                .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

            // 向 Edge Function 发送测试请求
            // 这个 Edge Function 使用路径参数而不是查询参数
            let test_url = "d15.api.augmentcode.com/get-models"; // 不需要 https://

            // 构建完整的代理 URL（路径方式）
            let proxy_url = if custom_url.ends_with("/") {
                format!("{}{}", custom_url, test_url)
            } else {
                format!("{}/{}", custom_url, test_url)
            };

            // 构建请求到 Edge Function
            let response = client
                .get(&proxy_url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| format!("Edge Function test failed: {}", e))?;

            // 对于 Edge Function，我们接受任何响应状态
            // 只要 Edge Function 本身能响应就算成功
            // 因为目标服务可能返回各种状态码
            if response.status() == 400 {
                // 如果是 400，可能是 Edge Function 自己的错误（如缺少参数）
                let body = response.text().await.unwrap_or_default();
                Err(format!("Edge Function error: {}", body))
            } else {
                // 其他任何状态码都算成功（包括目标服务返回的错误）
                Ok(())
            }
        } else {
            Err("Custom URL is not configured".to_string())
        }
    } else {
        // 传统代理的测试方式
        let client = config.create_client()?;

        // 简单的健康检查请求
        let test_url = "https://app.augmentcode.com";

        let _response = client
            .get(test_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Proxy test failed: {}", e))?;

        // 只要能连接成功就算通过，不管返回什么状态码
        // 因为有些代理可能会返回自己的错误页面
        Ok(())
    }
}

// 代理配置相关命令
#[tauri::command]
pub async fn save_proxy_config(
    app: tauri::AppHandle,
    proxy_type: String,
    enabled: bool,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    custom_url: Option<String>,
) -> Result<(), String> {
    let proxy_type = match proxy_type.as_str() {
        "system" => ProxyType::System,
        "http" => ProxyType::Http,
        "https" => ProxyType::Https,
        "socks5" => ProxyType::Socks5,
        "custom_url" => ProxyType::CustomUrl,
        _ => return Err(format!("Unknown proxy type: {}", proxy_type)),
    };

    let config = ProxyConfig {
        enabled,
        proxy_type,
        host: host.unwrap_or_default(),
        port: port.unwrap_or(7890),
        username,
        password,
        custom_url,
    };

    save_proxy_config_internal(&app, &config)
        .map_err(|e| format!("Failed to save proxy config: {}", e))?;
    app.state::<crate::AppState>().invalidate_gateway_http_client();
    Ok(())
}

#[tauri::command]
pub async fn load_proxy_config(app: tauri::AppHandle) -> Result<ProxyConfig, String> {
    load_proxy_config_internal(&app).map_err(|e| format!("Failed to load proxy config: {}", e))
}

#[tauri::command]
pub async fn test_proxy_config(
    proxy_type: String,
    enabled: bool,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    custom_url: Option<String>,
) -> Result<(), String> {
    let proxy_type = match proxy_type.as_str() {
        "system" => ProxyType::System,
        "http" => ProxyType::Http,
        "https" => ProxyType::Https,
        "socks5" => ProxyType::Socks5,
        "custom_url" => ProxyType::CustomUrl,
        _ => return Err(format!("Unknown proxy type: {}", proxy_type)),
    };

    let config = ProxyConfig {
        enabled,
        proxy_type,
        host: host.unwrap_or_default(),
        port: port.unwrap_or(7890),
        username,
        password,
        custom_url,
    };

    test_proxy_connection(&config).await
}

#[tauri::command]
pub async fn delete_proxy_config(app: tauri::AppHandle) -> Result<(), String> {
    delete_proxy_config_internal(&app)
        .map_err(|e| format!("Failed to delete proxy config: {}", e))?;
    app.state::<crate::AppState>().invalidate_gateway_http_client();
    Ok(())
}

#[tauri::command]
pub async fn proxy_config_exists(app: tauri::AppHandle) -> Result<bool, String> {
    proxy_config_exists_internal(&app).map_err(|e| format!("Failed to check proxy config: {}", e))
}
