//! 网关协议转换的 canonical 枢轴模型（OpenAI Chat Completions）
//!
//! 仅保留本期三入站链所需子集。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// canonical 角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "system" | "developer" => Role::System,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User,
        }
    }
}

/// 消息内容分片（文本 / 图像）
#[derive(Debug, Clone)]
pub enum ContentPart {
    Text(String),
    ImageUrl { url: String, detail: Option<String> },
}

/// 工具调用（assistant 发起）
#[derive(Debug, Clone, Default)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// 函数参数（JSON 字符串）
    pub arguments: String,
}

/// 工具定义（function）
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Value,
}

/// canonical 消息
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentPart>,
    /// 推理内容（reasoning / thinking）
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    /// role=tool 时指向的工具调用 id
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

impl Message {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            content: Vec::new(),
            reasoning: None,
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        }
    }

    /// 将内容分片合并为纯文本（多模态时仅取文本片段）
    pub fn text(&self) -> String {
        let mut out = String::new();
        for part in &self.content {
            if let ContentPart::Text(t) = part {
                out.push_str(t);
            }
        }
        out
    }
}

/// canonical 请求
#[derive(Debug, Clone, Default)]
pub struct CanonicalRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub tool_choice: Option<Value>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u64>,
    pub stream: bool,
    pub stop: Option<Value>,
    pub reasoning_effort: Option<String>,
    /// 未显式建模字段的兜底透传
    pub extra: Map<String, Value>,
}

/// 用量统计
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub cached_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
}

/// canonical 非流式响应
#[derive(Debug, Clone, Default)]
pub struct CanonicalResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// canonical 流事件
#[derive(Debug, Clone)]
pub enum StreamEvent {
    ContentDelta(String),
    ReasoningDelta(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: String,
    },
    Finish {
        reason: Option<String>,
    },
    Usage(Usage),
    Error {
        message: String,
    },
    Done,
}
