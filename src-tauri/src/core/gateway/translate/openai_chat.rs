//! OpenAI Chat Completions 协议转换。
//!
//! 既作入站协议，也作 `openai_compat`（chat 线型）渠道的出站协议。canonical 即 Chat，
//! 故请求/响应映射接近恒等。

use serde_json::{json, Map, Value};

use super::common;
use super::{InboundTranslator, OutboundTranslator, ParseState, RenderState, SseChunk};
use crate::core::gateway::canonical::{
    CanonicalRequest, CanonicalResponse, StreamEvent, Usage,
};

pub struct OpenAiChat;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn gen_id() -> String {
    format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis())
}

pub fn parse_usage(v: &Value) -> Option<Usage> {
    let u = v.get("usage").filter(|u| !u.is_null())?;
    Some(Usage {
        prompt_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0),
        completion_tokens: u
            .get("completion_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        total_tokens: u.get("total_tokens").and_then(|x| x.as_u64()).unwrap_or(0),
        cached_tokens: u
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        cache_write_tokens: 0,
        reasoning_tokens: u
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    })
}

pub fn usage_to_value(usage: &Usage) -> Value {
    json!({
        "prompt_tokens": usage.prompt_tokens,
        "completion_tokens": usage.completion_tokens,
        "total_tokens": usage.total_tokens,
        "prompt_tokens_details": {"cached_tokens": usage.cached_tokens},
        "completion_tokens_details": {"reasoning_tokens": usage.reasoning_tokens},
    })
}

/// 构造一个 chat.completion.chunk 信封
fn chunk(st: &RenderState, delta: Value, finish: Option<&str>) -> Value {
    let mut choice = Map::new();
    choice.insert("index".into(), json!(0));
    choice.insert("delta".into(), delta);
    choice.insert(
        "finish_reason".into(),
        finish.map(|f| json!(f)).unwrap_or(Value::Null),
    );
    json!({
        "id": st.id,
        "object": "chat.completion.chunk",
        "created": st.created,
        "model": st.model,
        "choices": [Value::Object(choice)],
    })
}

impl InboundTranslator for OpenAiChat {
    fn parse_request(&self, body: &Value) -> Result<CanonicalRequest, String> {
        common::parse_chat_request(body)
    }

    fn render_response(&self, resp: &CanonicalResponse) -> Value {
        let mut msg = Map::new();
        msg.insert("role".into(), json!("assistant"));
        if resp.content.is_empty() && !resp.tool_calls.is_empty() {
            msg.insert("content".into(), Value::Null);
        } else {
            msg.insert("content".into(), json!(resp.content));
        }
        if let Some(r) = &resp.reasoning {
            msg.insert("reasoning_content".into(), json!(r));
        }
        if !resp.tool_calls.is_empty() {
            msg.insert(
                "tool_calls".into(),
                common::tool_calls_to_value(&resp.tool_calls),
            );
        }
        let id = if resp.id.is_empty() { gen_id() } else { resp.id.clone() };
        json!({
            "id": id,
            "object": "chat.completion",
            "created": now(),
            "model": resp.model,
            "choices": [{
                "index": 0,
                "message": Value::Object(msg),
                "finish_reason": resp.finish_reason.clone().unwrap_or_else(|| "stop".into()),
            }],
            "usage": resp.usage.as_ref().map(usage_to_value).unwrap_or(Value::Null),
        })
    }

    fn render_stream(&self, ev: &StreamEvent, st: &mut RenderState) -> Vec<SseChunk> {
        if st.id.is_empty() {
            st.id = gen_id();
            st.created = now();
        }
        let mut out = Vec::new();
        if !st.started {
            st.started = true;
            out.push(SseChunk::data(
                chunk(st, json!({"role": "assistant", "content": ""}), None).to_string(),
            ));
        }
        match ev {
            StreamEvent::ContentDelta(t) => {
                out.push(SseChunk::data(chunk(st, json!({"content": t}), None).to_string()));
            }
            StreamEvent::ReasoningDelta(t) => {
                out.push(SseChunk::data(
                    chunk(st, json!({"reasoning_content": t}), None).to_string(),
                ));
            }
            StreamEvent::ToolCallDelta { index, id, name, arguments } => {
                let mut f = Map::new();
                if let Some(n) = name {
                    f.insert("name".into(), json!(n));
                }
                f.insert("arguments".into(), json!(arguments));
                let mut tc = Map::new();
                tc.insert("index".into(), json!(index));
                if let Some(i) = id {
                    tc.insert("id".into(), json!(i));
                }
                tc.insert("type".into(), json!("function"));
                tc.insert("function".into(), Value::Object(f));
                out.push(SseChunk::data(
                    chunk(st, json!({"tool_calls": [Value::Object(tc)]}), None).to_string(),
                ));
            }
            StreamEvent::Finish { reason } => {
                st.finish_reason = reason.clone();
            }
            StreamEvent::Usage(u) => st.usage = Some(u.clone()),
            StreamEvent::Error { message } => {
                out.push(SseChunk::data(
                    json!({"error": {"message": message, "type": "upstream_error"}}).to_string(),
                ));
                out.push(SseChunk::data("[DONE]"));
            }
            StreamEvent::Done => {
                let finish = st.finish_reason.clone().unwrap_or_else(|| "stop".into());
                out.push(SseChunk::data(chunk(st, json!({}), Some(&finish)).to_string()));
                if let Some(u) = &st.usage {
                    let mut envelope = chunk(st, json!({}), None);
                    envelope["choices"] = json!([]);
                    envelope["usage"] = usage_to_value(u);
                    out.push(SseChunk::data(envelope.to_string()));
                }
                out.push(SseChunk::data("[DONE]"));
            }
        }
        out
    }
}

impl OutboundTranslator for OpenAiChat {
    fn build_request(&self, req: &CanonicalRequest) -> Value {
        let mut obj = Map::new();
        obj.insert("model".into(), json!(req.model));
        obj.insert("stream".into(), json!(req.stream));
        if req.stream {
            obj.insert("stream_options".into(), json!({"include_usage": true}));
        }
        common::apply_common_fields(&mut obj, req);
        Value::Object(obj)
    }

    fn parse_response(&self, body: &Value) -> Result<CanonicalResponse, String> {
        let choice = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first());
        let msg = choice.and_then(|c| c.get("message"));
        let content = msg
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();
        let reasoning = msg
            .and_then(|m| m.get("reasoning_content").or_else(|| m.get("reasoning")))
            .and_then(|r| r.as_str())
            .map(String::from);
        let tool_calls = msg
            .and_then(|m| m.get("tool_calls"))
            .map(common::parse_tool_calls)
            .unwrap_or_default();
        Ok(CanonicalResponse {
            id: body.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            model: body.get("model").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            content,
            reasoning,
            tool_calls,
            finish_reason: choice
                .and_then(|c| c.get("finish_reason"))
                .and_then(|f| f.as_str())
                .map(String::from),
            usage: parse_usage(body),
        })
    }

    fn parse_stream(&self, _event: Option<&str>, data: &str, _st: &mut ParseState) -> Vec<StreamEvent> {
        let trimmed = data.trim();
        if trimmed == "[DONE]" {
            return vec![StreamEvent::Done];
        }
        let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
            return Vec::new();
        };
        let mut events = Vec::new();
        if let Some(u) = parse_usage(&v) {
            events.push(StreamEvent::Usage(u));
        }
        let choice = v
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first());
        if let Some(delta) = choice.and_then(|c| c.get("delta")) {
            if let Some(t) = delta.get("content").and_then(|c| c.as_str()) {
                if !t.is_empty() {
                    events.push(StreamEvent::ContentDelta(t.to_string()));
                }
            }
            if let Some(r) = delta
                .get("reasoning_content")
                .or_else(|| delta.get("reasoning"))
                .and_then(|r| r.as_str())
            {
                if !r.is_empty() {
                    events.push(StreamEvent::ReasoningDelta(r.to_string()));
                }
            }
            if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                for (i, tc) in tcs.iter().enumerate() {
                    let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(i as u64) as usize;
                    events.push(StreamEvent::ToolCallDelta {
                        index,
                        id: tc.get("id").and_then(|v| v.as_str()).map(String::from),
                        name: tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        arguments: tc
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    });
                }
            }
        }
        if let Some(reason) = choice
            .and_then(|c| c.get("finish_reason"))
            .and_then(|f| f.as_str())
        {
            events.push(StreamEvent::Finish {
                reason: Some(reason.to_string()),
            });
        }
        events
    }
}

