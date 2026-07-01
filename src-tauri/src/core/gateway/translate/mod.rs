//! 协议转换 trait 与共享类型
//!
//! 纯数据映射（无 IO）：入站协议 ↔ canonical ↔ 出站协议。新增渠道协议只需新增
//! 其 `OutboundTranslator`，新增入站协议只需新增其 `InboundTranslator`。

pub mod anthropic;
pub mod common;
pub mod openai_chat;
pub mod openai_responses;
pub mod stream_bridge;

use std::collections::HashMap;

use serde_json::Value;

use super::canonical::{CanonicalRequest, CanonicalResponse, StreamEvent, Usage};

/// 出站协议线型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Chat,
    Responses,
    Anthropic,
}

/// 出站 SSE 分片
#[derive(Debug, Clone)]
pub struct SseChunk {
    /// 具名事件（Anthropic / Responses 使用），Chat 为 None
    pub event: Option<String>,
    /// data 负载（序列化 JSON 或 `[DONE]`）
    pub data: String,
}

impl SseChunk {
    pub fn data(data: impl Into<String>) -> Self {
        Self {
            event: None,
            data: data.into(),
        }
    }

    pub fn named(event: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event: Some(event.into()),
            data: data.into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut s = String::new();
        if let Some(ev) = &self.event {
            s.push_str("event: ");
            s.push_str(ev);
            s.push('\n');
        }
        s.push_str("data: ");
        s.push_str(&self.data);
        s.push_str("\n\n");
        s.into_bytes()
    }
}

/// 出站流式解析中的 Anthropic content block 元信息
#[derive(Debug, Clone, Default)]
pub struct AnthBlock {
    pub kind: String,
    /// tool_use 块对应的 canonical tool 序号
    pub tool_index: usize,
}

/// 出站协议流式解析的累积状态（拼接增量）
#[derive(Debug, Default)]
pub struct ParseState {
    pub model: String,
    /// 累积的用量（input/cache 早到，output 末包补齐）
    pub usage: Usage,
    /// 已分配的 tool 调用序号计数
    pub tool_seq: usize,
    /// Anthropic：content block index → 块信息
    pub anth_blocks: HashMap<i64, AnthBlock>,
    /// Responses：output item id → 已分配的 tool 序号
    pub resp_tool_index: HashMap<String, usize>,
}

/// 入站协议流式渲染的累积状态
#[derive(Debug, Default)]
pub struct RenderState {
    pub id: String,
    pub model: String,
    pub created: i64,
    pub started: bool,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
    /// Anthropic 渲染：message_start 是否已发
    pub anth_message_started: bool,
    /// Anthropic 渲染：当前打开块的类型（""=无 / text / thinking）
    pub anth_open_kind: String,
    /// Anthropic 渲染：当前打开块的序号
    pub anth_open_index: i64,
    /// Anthropic 渲染：下一个 content block 序号
    pub anth_next_block: i64,
    /// Anthropic 渲染：canonical tool index → anthropic block index
    pub anth_tool_blocks: HashMap<usize, i64>,
    /// Responses 渲染状态
    pub resp: ResponsesRender,
}

/// Responses 入站渲染的累积状态
#[derive(Debug, Default)]
pub struct ResponsesRender {
    /// 自增事件序号 sequence_number
    pub seq: i64,
    /// 下一个 output_index
    pub next_output: i64,
    /// 文本 message item 的 output_index（未开则 None）
    pub text_item: Option<i64>,
    /// reasoning item 的 output_index（未开则 None）
    pub reasoning_item: Option<i64>,
    /// 累积的文本/推理（用于 done 事件与最终 completed）
    pub text_buf: String,
    pub reasoning_buf: String,
    /// canonical tool index 的到达顺序
    pub tool_order: Vec<usize>,
    /// canonical tool index → 累积的调用信息
    pub tools: HashMap<usize, RespTool>,
}

/// Responses 渲染中累积的单个工具调用
#[derive(Debug, Default, Clone)]
pub struct RespTool {
    pub output_index: i64,
    pub id: String,
    pub name: String,
    pub args: String,
}

/// 入站协议转换器：客户端协议 ↔ canonical
pub trait InboundTranslator: Send + Sync {
    /// 客户端请求体 → canonical 请求
    fn parse_request(&self, body: &Value) -> Result<CanonicalRequest, String>;
    /// canonical 响应 → 客户端非流式响应体
    fn render_response(&self, resp: &CanonicalResponse) -> Value;
    /// canonical 流事件 → 客户端 SSE 分片（可能展开为多片）
    fn render_stream(&self, ev: &StreamEvent, st: &mut RenderState) -> Vec<SseChunk>;
}

/// 出站协议转换器：canonical ↔ 渠道协议
pub trait OutboundTranslator: Send + Sync {
    /// canonical 请求 → 渠道请求体
    fn build_request(&self, req: &CanonicalRequest) -> Value;
    /// 渠道非流式响应 → canonical 响应
    fn parse_response(&self, body: &Value) -> Result<CanonicalResponse, String>;
    /// 渠道流式事件（event 名 + data 负载）→ canonical 流事件
    fn parse_stream(&self, event: Option<&str>, data: &str, st: &mut ParseState) -> Vec<StreamEvent>;
}
