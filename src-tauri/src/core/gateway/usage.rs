//! 网关用量记录存储
//!
//! 轻量旁路统计：每次请求收尾落一条记录（仅 token / TTFT / 时延，无任何价格字段）。
//! 使用本地 SQLite（`gateway_usage.db`）持久化，供前端用量页读取。

use std::path::PathBuf;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::canonical::Usage;

/// 单条用量记录（字段名对齐前端 `GatewayUsage.vue` 的 camelCase 契约）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    #[serde(rename = "requestId")]
    pub request_id: String,
    /// 毫秒时间戳
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    pub model: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "channelName", default)]
    pub channel_name: String,
    /// 渠道类型：codex_oauth / openai_compat / anthropic
    #[serde(default)]
    pub kind: String,
    /// 入站协议：chat / responses / anthropic
    #[serde(default)]
    pub inbound: String,
    /// success / error
    pub status: String,
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub stream: bool,
    #[serde(rename = "durationMs")]
    pub duration_ms: u128,
    #[serde(rename = "ttftMs", skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u128>,
    #[serde(rename = "promptTokens")]
    pub prompt_tokens: u64,
    #[serde(rename = "completionTokens")]
    pub completion_tokens: u64,
    #[serde(rename = "totalTokens")]
    pub total_tokens: u64,
    #[serde(rename = "cachedTokens")]
    pub cached_tokens: u64,
    #[serde(rename = "cacheWriteTokens", default)]
    pub cache_write_tokens: u64,
    #[serde(rename = "reasoningTokens")]
    pub reasoning_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl UsageRecord {
    /// 套用 canonical 用量明细
    pub fn with_usage(mut self, usage: &Usage) -> Self {
        self.prompt_tokens = usage.prompt_tokens;
        self.completion_tokens = usage.completion_tokens;
        self.total_tokens = if usage.total_tokens > 0 {
            usage.total_tokens
        } else {
            usage.prompt_tokens + usage.completion_tokens
        };
        self.cached_tokens = usage.cached_tokens;
        self.cache_write_tokens = usage.cache_write_tokens;
        self.reasoning_tokens = usage.reasoning_tokens;
        self
    }
}

/// 用量记录存储（本地 SQLite，每次操作开新连接）
pub struct GatewayUsageStore {
    db_path: PathBuf,
}

impl GatewayUsageStore {
    /// 打开/初始化 SQLite 存储
    pub fn load(db_path: PathBuf) -> Self {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let store = Self { db_path };
        if let Err(e) = store.init_schema() {
            eprintln!("[GatewayUsage] 初始化 SQLite 失败: {}", e);
        }
        store
    }

    /// 获取连接并设置忙等待，规避并发写入的 SQLITE_BUSY
    fn get_connection(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("打开用量数据库失败: {}", e))?;
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        Ok(conn)
    }

    /// 建表与索引
    fn init_schema(&self) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS gateway_usage (
                request_id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                model TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                channel_name TEXT NOT NULL DEFAULT '',
                kind TEXT NOT NULL DEFAULT '',
                inbound TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                status_code INTEGER NOT NULL DEFAULT 0,
                stream INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                ttft_ms INTEGER,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                cached_tokens INTEGER NOT NULL DEFAULT 0,
                cache_write_tokens INTEGER NOT NULL DEFAULT 0,
                reasoning_tokens INTEGER NOT NULL DEFAULT 0,
                error TEXT
            )",
            [],
        )
        .map_err(|e| format!("创建用量表失败: {}", e))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_gateway_usage_created ON gateway_usage(created_at DESC)",
            [],
        )
        .map_err(|e| format!("创建时间索引失败: {}", e))?;
        Ok(())
    }

    /// 追加一条记录
    pub fn record(&self, rec: UsageRecord) {
        if let Err(e) = self.insert(&rec) {
            eprintln!("[GatewayUsage] 写入记录失败: {}", e);
        }
    }

    fn insert(&self, rec: &UsageRecord) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO gateway_usage
             (request_id, created_at, model, channel_id, channel_name, kind, inbound,
              status, status_code, stream, duration_ms, ttft_ms,
              prompt_tokens, completion_tokens, total_tokens,
              cached_tokens, cache_write_tokens, reasoning_tokens, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                rec.request_id,
                rec.created_at,
                rec.model,
                rec.channel_id,
                rec.channel_name,
                rec.kind,
                rec.inbound,
                rec.status,
                rec.status_code as i64,
                rec.stream,
                rec.duration_ms as i64,
                rec.ttft_ms.map(|v| v as i64),
                rec.prompt_tokens as i64,
                rec.completion_tokens as i64,
                rec.total_tokens as i64,
                rec.cached_tokens as i64,
                rec.cache_write_tokens as i64,
                rec.reasoning_tokens as i64,
                rec.error,
            ],
        )
        .map_err(|e| format!("插入记录失败: {}", e))?;
        Ok(())
    }

    /// 读取全部记录（最新在前）
    pub fn list(&self) -> Vec<UsageRecord> {
        match self.query_all() {
            Ok(records) => records,
            Err(e) => {
                eprintln!("[GatewayUsage] 读取记录失败: {}", e);
                Vec::new()
            }
        }
    }

    fn query_all(&self) -> Result<Vec<UsageRecord>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT request_id, created_at, model, channel_id, channel_name, kind, inbound,
                        status, status_code, stream, duration_ms, ttft_ms,
                        prompt_tokens, completion_tokens, total_tokens,
                        cached_tokens, cache_write_tokens, reasoning_tokens, error
                 FROM gateway_usage
                 ORDER BY created_at DESC, rowid DESC",
            )
            .map_err(|e| format!("准备查询失败: {}", e))?;
        let rows = stmt
            .query_map([], row_to_record)
            .map_err(|e| format!("执行查询失败: {}", e))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| format!("读取记录行失败: {}", e))?);
        }
        Ok(records)
    }

    /// 清空全部记录
    pub fn clear(&self) {
        let result = self
            .get_connection()
            .and_then(|conn| {
                conn.execute("DELETE FROM gateway_usage", [])
                    .map_err(|e| format!("清空记录失败: {}", e))
            });
        if let Err(e) = result {
            eprintln!("[GatewayUsage] 清空记录失败: {}", e);
        }
    }
}

/// SQLite 行映射为 `UsageRecord`
fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<UsageRecord> {
    Ok(UsageRecord {
        request_id: row.get(0)?,
        created_at: row.get(1)?,
        model: row.get(2)?,
        channel_id: row.get(3)?,
        channel_name: row.get(4)?,
        kind: row.get(5)?,
        inbound: row.get(6)?,
        status: row.get(7)?,
        status_code: row.get::<_, i64>(8)? as u16,
        stream: row.get(9)?,
        duration_ms: row.get::<_, i64>(10)? as u128,
        ttft_ms: row.get::<_, Option<i64>>(11)?.map(|v| v as u128),
        prompt_tokens: row.get::<_, i64>(12)? as u64,
        completion_tokens: row.get::<_, i64>(13)? as u64,
        total_tokens: row.get::<_, i64>(14)? as u64,
        cached_tokens: row.get::<_, i64>(15)? as u64,
        cache_write_tokens: row.get::<_, i64>(16)? as u64,
        reasoning_tokens: row.get::<_, i64>(17)? as u64,
        error: row.get(18)?,
    })
}
