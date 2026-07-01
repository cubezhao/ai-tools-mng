//! 流式事件累积器：将 canonical `StreamEvent` 聚合为一个 `CanonicalResponse`。
//!
//! 用于「上游流式 → 客户端非流式」的回聚合，以及流式透传时的旁路用量采集。

use std::collections::HashMap;

use crate::core::gateway::canonical::{CanonicalResponse, StreamEvent, ToolCall, Usage};

/// canonical 流事件累积器
#[derive(Default)]
pub struct StreamAcc {
    content: String,
    reasoning: String,
    tool_order: Vec<usize>,
    tools: HashMap<usize, ToolCall>,
    finish: Option<String>,
    usage: Usage,
    got_usage: bool,
    error: Option<String>,
}

impl StreamAcc {
    /// 套用一个 canonical 流事件
    pub fn apply(&mut self, ev: &StreamEvent) {
        match ev {
            StreamEvent::ContentDelta(t) => self.content.push_str(t),
            StreamEvent::ReasoningDelta(t) => self.reasoning.push_str(t),
            StreamEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments,
            } => {
                if !self.tools.contains_key(index) {
                    self.tool_order.push(*index);
                    self.tools.insert(*index, ToolCall::default());
                }
                let tc = self.tools.get_mut(index).expect("tool just inserted");
                if let Some(i) = id {
                    if !i.is_empty() {
                        tc.id = i.clone();
                    }
                }
                if let Some(n) = name {
                    if !n.is_empty() {
                        tc.name = n.clone();
                    }
                }
                tc.arguments.push_str(arguments);
            }
            StreamEvent::Finish { reason } => {
                if reason.is_some() {
                    self.finish = reason.clone();
                }
            }
            StreamEvent::Usage(u) => {
                self.usage = u.clone();
                self.got_usage = true;
            }
            StreamEvent::Error { message } => {
                self.error = Some(message.clone());
            }
            StreamEvent::Done => {}
        }
    }

    /// 已累积的用量（缺失则为零值）
    pub fn usage(&self) -> Usage {
        self.usage.clone()
    }

    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// 收敛为非流式 canonical 响应
    pub fn into_response(self, model: &str) -> CanonicalResponse {
        let mut tool_calls = Vec::new();
        for idx in &self.tool_order {
            if let Some(t) = self.tools.get(idx) {
                tool_calls.push(t.clone());
            }
        }
        CanonicalResponse {
            id: format!("gw-{}", chrono::Utc::now().timestamp_millis()),
            model: model.to_string(),
            content: self.content,
            reasoning: if self.reasoning.is_empty() {
                None
            } else {
                Some(self.reasoning)
            },
            tool_calls,
            finish_reason: self.finish,
            usage: if self.got_usage {
                Some(self.usage)
            } else {
                None
            },
        }
    }
}
