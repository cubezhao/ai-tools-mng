use crate::platforms::openai::models::{Account, TokenData};
use crate::platforms::openai::modules::oauth;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{Emitter, Url};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// 全局取消 Token 存储
static CANCELLATION_TOKEN: OnceLock<Mutex<Option<oneshot::Sender<()>>>> = OnceLock::new();

// 全局 OAuth 数据存储
struct OAuthData {
    code_verifier: String,
    state: String,
}

static OAUTH_DATA: OnceLock<Mutex<Option<OAuthData>>> = OnceLock::new();
const OAUTH_CALLBACK_TIMEOUT_SECS: u64 = 300;

/// 获取取消 Token 的 Mutex
fn get_cancellation_token() -> &'static Mutex<Option<oneshot::Sender<()>>> {
    CANCELLATION_TOKEN.get_or_init(|| Mutex::new(None))
}

/// 获取 OAuth 数据的 Mutex
fn get_oauth_data() -> &'static Mutex<Option<OAuthData>> {
    OAUTH_DATA.get_or_init(|| Mutex::new(None))
}

/// 设置 OAuth 数据
fn set_oauth_data(code_verifier: String, state: String) {
    let mutex = get_oauth_data();
    if let Ok(mut lock) = mutex.lock() {
        *lock = Some(OAuthData {
            code_verifier,
            state,
        });
    }
}

/// 获取并清除 OAuth 数据
fn take_oauth_data() -> Option<(String, String)> {
    let mutex = get_oauth_data();
    if let Ok(mut lock) = mutex.lock() {
        lock.take().map(|data| (data.code_verifier, data.state))
    } else {
        None
    }
}

fn clear_cancellation_token() {
    let mutex = get_cancellation_token();
    if let Ok(mut lock) = mutex.lock() {
        *lock = None;
    }
}

fn clear_oauth_data() {
    let _ = take_oauth_data();
}

/// 取消当前的 OAuth 流程
pub fn cancel_oauth_flow() {
    let mutex = get_cancellation_token();
    if let Ok(mut lock) = mutex.lock() {
        if let Some(tx) = lock.take() {
            let _ = tx.send(());
            println!("已发送 OpenAI OAuth 取消信号");
        }
    }
    clear_oauth_data();
}

/// 绑定 OAuth 回调监听端口，带重试以等待上一次流程释放端口 1455
async fn bind_callback_listener() -> Result<TcpListener, String> {
    const MAX_ATTEMPTS: u32 = 20;
    let mut last_err = None;
    for attempt in 0..MAX_ATTEMPTS {
        match TcpListener::bind("127.0.0.1:1455").await {
            Ok(listener) => return Ok(listener),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }
        }
    }
    Err(format!(
        "无法绑定本地端口 1455: {}。请确认没有其他程序占用该端口后重试",
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "未知错误".to_string())
    ))
}

/// 启动 OAuth 流程
/// 1. 启动本地服务器监听回调
/// 2. 打开浏览器访问授权页面
/// 3. 等待并捕获 code
/// 4. 交换 token
pub async fn start_oauth_flow(app_handle: tauri::AppHandle) -> Result<Account, String> {
    // 先取消可能仍在运行的上一次授权流程，避免端口 1455 被占用
    cancel_oauth_flow();

    // 1. 生成 PKCE 参数
    let code_verifier = oauth::generate_code_verifier();
    let code_challenge = oauth::generate_code_challenge(&code_verifier)?;
    let state = oauth::generate_state();

    // 2. 启动本地监听器 (固定端口 1455，带重试以等待上一次流程释放端口)
    let listener = bind_callback_listener().await?;
    let redirect_uri = "http://localhost:1455/auth/callback".to_string();

    // 成功绑定端口后再写入全局状态，避免与被取消流程的清理产生竞态
    let (tx, rx) = oneshot::channel::<()>();
    {
        let mutex = get_cancellation_token();
        if let Ok(mut lock) = mutex.lock() {
            *lock = Some(tx);
        }
    }

    // 存储 OAuth 数据
    set_oauth_data(code_verifier, state.clone());

    // 3. 构建授权 URL
    let auth_url = oauth::build_authorization_url(&state, &code_challenge, &redirect_uri).map_err(
        |e| {
            clear_oauth_data();
            clear_cancellation_token();
            e
        },
    )?;

    // 发送事件给前端
    let _ = app_handle.emit("oauth-url-generated", &auth_url);

    // 4. 打开浏览器
    use tauri_plugin_opener::OpenerExt;
    app_handle
        .opener()
        .open_url(&auth_url, None::<String>)
        .map_err(|e| {
            clear_oauth_data();
            clear_cancellation_token();
            format!("无法打开浏览器: {}", e)
        })?;

    println!("已打开浏览器进行 OpenAI OAuth 授权");
    println!("回调地址: {}", redirect_uri);

    // 5. 等待回调
    let timeout = tokio::time::sleep(Duration::from_secs(OAUTH_CALLBACK_TIMEOUT_SECS));
    tokio::pin!(timeout);
    let (mut stream, _) = tokio::select! {
        res = listener.accept() => {
             res.map_err(|e| {
                 clear_oauth_data();
                 clear_cancellation_token();
                 format!("接受连接失败: {}", e)
             })?
        }
        _ = rx => {
            // 清除 OAuth 数据
            clear_oauth_data();
            clear_cancellation_token();
            return Err("用户取消了授权".to_string());
        }
        _ = &mut timeout => {
            clear_oauth_data();
            clear_cancellation_token();
            return Err("授权等待超时，请重新发起登录".to_string());
        }
    };

    // 清除取消 token
    clear_cancellation_token();

    let mut buffer = [0; 2048];
    let n = stream
        .read(&mut buffer)
        .await
        .map_err(|e| {
            clear_oauth_data();
            format!("读取请求失败: {}", e)
        })?;

    let request = String::from_utf8_lossy(&buffer[..n]);
    println!("收到回调请求: {}", request.lines().next().unwrap_or(""));

    // 获取并清除 OAuth 数据
    let (code_verifier, expected_state) =
        take_oauth_data().ok_or("OAuth 数据已过期，请重新开始".to_string())?;

    // 解析请求行获取 code 和 state
    let (code, _returned_state) = if let Some(line) = request.lines().next() {
        if let Some(path) = line.split_whitespace().nth(1) {
            let url = Url::parse(&format!("http://localhost:1455{}", path))
                .map_err(|e| format!("URL 解析失败: {}", e))?;

            let pairs = url.query_pairs();
            let mut code = None;
            let mut state_val = None;
            let mut error = None;
            for (key, value) in pairs {
                let value_str = value.as_ref();
                if key == "code" {
                    code = Some(value_str.to_string());
                }
                if key == "state" {
                    state_val = Some(value_str.to_string());
                }
                if key == "error" {
                    error = Some(value_str.to_string());
                }
            }

            if let Some(err) = error {
                return Err(format!("OAuth 返回错误: {}", err));
            }

            // 验证 state
            if let Some(ref returned) = state_val {
                if returned != &expected_state {
                    return Err("State 验证失败，可能存在安全风险".to_string());
                }
            }

            (code, state_val)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let response_html = if code.is_some() {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <html>\
        <head><meta charset='utf-8'></head>\
        <body style='font-family: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif; text-align: center; padding: 50px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); min-height: 100vh; margin: 0;'>\
            <div style='background: white; border-radius: 12px; padding: 40px; box-shadow: 0 10px 40px rgba(0,0,0,0.1); max-width: 400px; margin: 40px auto;'>\
                <div style='font-size: 48px; margin-bottom: 20px;'>✅</div>\
                <h1 style='color: #10a37f; margin: 0 0 16px 0; font-size: 24px;'>授权成功!</h1>\
                <p style='color: #6b7280; margin: 0 0 24px 0;'>您可以关闭此窗口返回应用。</p>\
                <script>\
                    setTimeout(function() {\
                        window.close();\
                        if (!window.closed) {\
                            document.body.innerHTML = '<p style=\"color: #10a37f;\">请手动关闭此窗口返回应用。</p>';\
                        }\
                    }, 2000);\
                </script>\
            </div>\
        </body>\
        </html>"
    } else {
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
        <html>\
        <head><meta charset='utf-8'></head>\
        <body style='font-family: sans-serif; text-align: center; padding: 50px;'>\
            <h1 style='color: #ef4444;'>❌ 授权失败</h1>\
            <p>未能获取授权码，请重试。</p>\
        </body>\
        </html>"
    };

    stream
        .write_all(response_html.as_bytes())
        .await
        .unwrap_or(());
    stream.flush().await.unwrap_or(());

    let code = code.ok_or("未能在回调中获取 Authorization Code".to_string())?;
    println!(
        "获取到授权码: {}...",
        &code.chars().take(20).collect::<String>()
    );

    // 6. 交换 Token
    let token = oauth::exchange_code(&code, &code_verifier, &redirect_uri).await?;

    // 7. 解析用户信息
    let user_info = token.id_token.as_deref().and_then(oauth::parse_id_token);
    let openai_auth_json = token
        .id_token
        .as_deref()
        .and_then(oauth::extract_openai_auth_json);

    // 获取邮箱和 chatgpt_account_id
    let email = user_info
        .as_ref()
        .and_then(|u| u.email.as_ref())
        .ok_or_else(|| "Failed to get email from token".to_string())?;

    let chatgpt_account_id = user_info
        .as_ref()
        .and_then(|u| u.chatgpt_account_id.clone());

    println!("=== OpenAI OAuth Flow ===");
    println!("Email: {}", email);
    println!("ChatGPT Account ID: {:?}", chatgpt_account_id);

    // 构造 TokenData
    let now = chrono::Utc::now().timestamp();
    let token_data = TokenData::new(
        token.access_token.clone(),
        token.refresh_token.clone(),
        token.id_token.clone(),
        token.expires_in,
        now + token.expires_in,
        token.token_type.clone(),
    );

    // 创建账号
    let mut account = Account::new_oauth(
        email.clone(),
        token_data,
        chatgpt_account_id.clone(),
        user_info.as_ref().and_then(|u| u.chatgpt_user_id.clone()),
        user_info.as_ref().and_then(|u| u.organization_id.clone()),
    );
    account.openai_auth_json = openai_auth_json;

    println!("OpenAI OAuth 授权成功: {}", account.email);

    Ok(account)
}
