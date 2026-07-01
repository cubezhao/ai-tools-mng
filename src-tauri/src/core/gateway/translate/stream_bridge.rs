//! 流式桥接：渠道 SSE 字节 → canonical 事件 → 客户端 SSE 字节。
//!
//! 增量解析渠道 SSE 帧，经 `OutboundTranslator` 解析为 canonical 事件，再由
//! `InboundTranslator` 重渲染为客户端协议分片。Responses↔Codex 同线型直转在路由层
//! 短路，不进入本桥接。

use super::anthropic::Anthropic;
use super::openai_chat::OpenAiChat;
use super::openai_responses::OpenAiResponses;
use super::{InboundTranslator, OutboundTranslator, ParseState, RenderState, Wire};
use crate::core::gateway::canonical::{StreamEvent, Usage};

/// 按线型取入站转换器
pub fn inbound_for(wire: Wire) -> Box<dyn InboundTranslator> {
    match wire {
        Wire::Chat => Box::new(OpenAiChat),
        Wire::Responses => Box::new(OpenAiResponses),
        Wire::Anthropic => Box::new(Anthropic),
    }
}

/// 按线型取出站转换器
pub fn outbound_for(wire: Wire) -> Box<dyn OutboundTranslator> {
    match wire {
        Wire::Chat => Box::new(OpenAiChat),
        Wire::Responses => Box::new(OpenAiResponses),
        Wire::Anthropic => Box::new(Anthropic),
    }
}

/// 增量 SSE 帧解析（以空行分帧，聚合 event/data 行）
#[derive(Default)]
pub struct SseDecoder {
    buf: String,
}

impl SseDecoder {
    /// 喂入新字节，返回已完成的 (event, data) 帧
    pub fn push(&mut self, bytes: &[u8]) -> Vec<(Option<String>, String)> {
        self.buf.push_str(&String::from_utf8_lossy(bytes));
        let mut frames = Vec::new();
        while let Some(pos) = self.buf.find("\n\n") {
            let raw: String = self.buf.drain(..pos + 2).collect();
            let mut event = None;
            let mut data = String::new();
            for line in raw.split('\n') {
                let line = line.trim_end_matches('\r');
                if let Some(rest) = line.strip_prefix("event:") {
                    event = Some(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                }
            }
            if !data.is_empty() || event.is_some() {
                frames.push((event, data));
            }
        }
        frames
    }
}

/// 跨协议流式桥接器
pub struct StreamBridge {
    out_tr: Box<dyn OutboundTranslator>,
    in_tr: Box<dyn InboundTranslator>,
    decoder: SseDecoder,
    parse: ParseState,
    render: RenderState,
    done: bool,
    usage: Usage,
    error: Option<String>,
}

impl StreamBridge {
    pub fn new(channel_wire: Wire, client_wire: Wire, model: &str) -> Self {
        Self {
            out_tr: outbound_for(channel_wire),
            in_tr: inbound_for(client_wire),
            decoder: SseDecoder::default(),
            parse: ParseState {
                model: model.to_string(),
                ..Default::default()
            },
            render: RenderState {
                model: model.to_string(),
                ..Default::default()
            },
            done: false,
            usage: Usage::default(),
            error: None,
        }
    }

    /// 是否已收到终止事件
    pub fn done(&self) -> bool {
        self.done
    }

    /// 桥接过程中累积的用量（取自上游 Usage 事件，缺失则回退解析状态）
    pub fn usage(&self) -> Usage {
        if self.usage != Usage::default() {
            self.usage.clone()
        } else {
            self.parse.usage.clone()
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// 喂入渠道字节，返回应转发给客户端的字节
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for (event, data) in self.decoder.push(bytes) {
            let events = self.out_tr.parse_stream(event.as_deref(), &data, &mut self.parse);
            for cev in events {
                if let StreamEvent::Usage(u) = &cev {
                    self.usage = u.clone();
                }
                if let StreamEvent::Error { message } = &cev {
                    self.error = Some(message.clone());
                    self.done = true;
                }
                if matches!(cev, StreamEvent::Done) {
                    if self.error.is_some() {
                        continue;
                    }
                    self.done = true;
                }
                for chunk in self.in_tr.render_stream(&cev, &mut self.render) {
                    out.extend(chunk.to_bytes());
                }
            }
        }
        out
    }

    /// 流结束时若上游未显式发送 Done，补发终止分片
    pub fn finish(&mut self) -> Vec<u8> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut out = Vec::new();
        for chunk in self.in_tr.render_stream(&StreamEvent::Done, &mut self.render) {
            out.extend(chunk.to_bytes());
        }
        out
    }
}
