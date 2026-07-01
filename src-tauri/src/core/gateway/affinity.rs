//! 会话粘性（轻量 sticky session）
//!
//! 维护「会话标识 → 上次成功渠道」的内存映射（带 TTL），让同一会话的多轮请求
//! 优先粘到上次成功的渠道，以命中上游 prompt/KV 缓存、降低 TTFT。仅在同优先级组
//! 内生效，不跨越优先级分组；上次渠道失效时 failover 后会自然漂移。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;
use warp::http::HeaderMap;

/// 映射存活时长（超过则视为失效）
const TTL: Duration = Duration::from_secs(300);

/// 会话粘性映射表
#[derive(Default)]
pub struct SessionAffinity {
    map: Mutex<HashMap<String, (String, Instant)>>,
}

impl SessionAffinity {
    pub fn new() -> Self {
        Self::default()
    }

    /// 读取会话上次成功渠道（过期视为无）
    pub fn get(&self, key: &str) -> Option<String> {
        let mut map = self.map.lock().ok()?;
        match map.get(key) {
            Some((ch, at)) if at.elapsed() < TTL => Some(ch.clone()),
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    }

    /// 写回会话→成功渠道，并顺带清理过期项
    pub fn set(&self, key: String, channel_id: String) {
        if let Ok(mut map) = self.map.lock() {
            map.retain(|_, (_, at)| at.elapsed() < TTL);
            map.insert(key, (channel_id, Instant::now()));
        }
    }
}

/// 从入站请求提取会话标识：Codex session 头 → Claude Code `metadata.user_id`
/// → OpenAI `prompt_cache_key`/`user`，都取不到则返回 None（粘性不生效）。
pub fn extract_session_key(headers: &HeaderMap, value: &Value) -> Option<String> {
    for h in ["session_id", "x-session-id", "conversation_id"] {
        if let Some(v) = headers.get(h).and_then(|v| v.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(format!("h:{}", v));
            }
        }
    }

    if let Some(uid) = value
        .pointer("/metadata/user_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let sess = uid.split("_session_").nth(1).unwrap_or(uid);
        return Some(format!("cc:{}", sess));
    }

    for k in ["prompt_cache_key", "user"] {
        if let Some(v) = value
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(format!("b:{}", v));
        }
    }

    None
}
