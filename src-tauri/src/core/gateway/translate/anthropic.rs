//! Anthropic Messages 协议转换。
//!
//! 既作入站协议（/gateway/v1/messages），也作 `anthropic` 渠道的出站协议。
//! canonical 即 Chat，故此处负责 Messages ↔ Chat 的双向映射。

use serde_json::{Value, json};

use super::{InboundTranslator, OutboundTranslator, ParseState, RenderState, SseChunk};
use crate::core::gateway::canonical::{
    CanonicalRequest, CanonicalResponse, ContentPart, Message, Role, StreamEvent, ToolCall, Usage,
};

pub struct Anthropic;

fn gen_id() -> String {
    format!("msg_{}", chrono::Utc::now().timestamp_millis())
}

/// canonical(OpenAI) finish_reason → Anthropic stop_reason
fn finish_to_anth(reason: Option<&str>) -> &'static str {
    match reason {
        Some("length") => "max_tokens",
        Some("tool_calls") => "tool_use",
        _ => "end_turn",
    }
}

/// Anthropic stop_reason → canonical(OpenAI) finish_reason
fn anth_to_finish(reason: &str) -> &'static str {
    match reason {
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        _ => "stop",
    }
}

/// 解析 Anthropic content（string 或 block 数组）→ 文本/图像分片
fn parse_blocks_content(value: &Value) -> Vec<ContentPart> {
    match value {
        Value::String(s) => vec![ContentPart::Text(s.clone())],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|b| match b.get("type").and_then(|t| t.as_str()) {
                Some("text") => b
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|t| ContentPart::Text(t.to_string())),
                Some("image") => {
                    let src = b.get("source")?;
                    let url = match src.get("type").and_then(|t| t.as_str()) {
                        Some("url") => src.get("url").and_then(|u| u.as_str())?.to_string(),
                        _ => {
                            let media = src
                                .get("media_type")
                                .and_then(|m| m.as_str())
                                .unwrap_or("image/png");
                            let data = src.get("data").and_then(|d| d.as_str()).unwrap_or("");
                            format!("data:{};base64,{}", media, data)
                        }
                    };
                    Some(ContentPart::ImageUrl { url, detail: None })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 将一条 Anthropic 消息展开为一条或多条 canonical 消息（tool_result 拆为独立 tool 消息）
fn push_anth_message(out: &mut Vec<Message>, role: Role, content: &Value) {
    let blocks = match content {
        Value::Array(arr) => arr.clone(),
        other => {
            let mut m = Message::new(role);
            m.content = parse_blocks_content(other);
            out.push(m);
            return;
        }
    };
    // 先发出 tool_result 对应的 tool 消息
    for b in &blocks {
        if b.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
            let mut tm = Message::new(Role::Tool);
            tm.tool_call_id = b
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            tm.content = match b.get("content") {
                Some(c @ Value::Array(_)) | Some(c @ Value::String(_)) => parse_blocks_content(c),
                _ => Vec::new(),
            };
            out.push(tm);
        }
    }
    // 再发出本体（文本/图像 + assistant 的 tool_use / thinking）
    let mut msg = Message::new(role);
    let mut reasoning = String::new();
    for b in &blocks {
        match b.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                    msg.content.push(ContentPart::Text(t.to_string()));
                }
            }
            Some("image") => msg.content.extend(parse_blocks_content(&json!([b]))),
            Some("thinking") => {
                if let Some(t) = b.get("thinking").and_then(|t| t.as_str()) {
                    reasoning.push_str(t);
                }
            }
            Some("tool_use") => msg.tool_calls.push(ToolCall {
                id: b
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: b
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                arguments: b
                    .get("input")
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "{}".into()),
            }),
            _ => {}
        }
    }
    if !reasoning.is_empty() {
        msg.reasoning = Some(reasoning);
    }
    if !msg.content.is_empty() || !msg.tool_calls.is_empty() {
        out.push(msg);
    }
}

/// 解析 Anthropic tools（{name, description, input_schema}）
fn parse_anth_tools(value: &Value) -> Vec<crate::core::gateway::canonical::Tool> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let name = t.get("name").and_then(|v| v.as_str())?.to_string();
                    Some(crate::core::gateway::canonical::Tool {
                        name,
                        description: t
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        parameters: t
                            .get("input_schema")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Anthropic tool_choice → canonical(OpenAI) tool_choice
fn parse_anth_tool_choice(value: &Value) -> Value {
    match value.get("type").and_then(|t| t.as_str()) {
        Some("auto") => json!("auto"),
        Some("any") => json!("required"),
        Some("none") => json!("none"),
        Some("tool") => json!({
            "type": "function",
            "function": {"name": value.get("name").and_then(|n| n.as_str()).unwrap_or("")}
        }),
        _ => json!("auto"),
    }
}

impl InboundTranslator for Anthropic {
    fn parse_request(&self, body: &Value) -> Result<CanonicalRequest, String> {
        let obj = body
            .as_object()
            .ok_or("request body must be a JSON object")?;
        let mut req = CanonicalRequest {
            model: obj
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            stream: obj.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
            max_tokens: obj.get("max_tokens").and_then(|v| v.as_u64()),
            temperature: obj.get("temperature").and_then(|v| v.as_f64()),
            top_p: obj.get("top_p").and_then(|v| v.as_f64()),
            stop: obj.get("stop_sequences").cloned(),
            ..Default::default()
        };
        if let Some(sys) = obj.get("system") {
            let mut m = Message::new(Role::System);
            m.content = parse_blocks_content(sys);
            if !m.content.is_empty() {
                req.messages.push(m);
            }
        }
        if let Some(msgs) = obj.get("messages").and_then(|m| m.as_array()) {
            for msg in msgs {
                let role =
                    Role::from_str(msg.get("role").and_then(|r| r.as_str()).unwrap_or("user"));
                if let Some(c) = msg.get("content") {
                    push_anth_message(&mut req.messages, role, c);
                }
            }
        }
        if let Some(tools) = obj.get("tools") {
            req.tools = parse_anth_tools(tools);
        }
        req.tool_choice = obj.get("tool_choice").map(parse_anth_tool_choice);
        Ok(req)
    }

    fn render_response(&self, resp: &CanonicalResponse) -> Value {
        let mut blocks: Vec<Value> = Vec::new();
        if let Some(r) = &resp.reasoning {
            if !r.is_empty() {
                blocks.push(json!({"type": "thinking", "thinking": r}));
            }
        }
        if !resp.content.is_empty() {
            blocks.push(json!({"type": "text", "text": resp.content}));
        }
        for tc in &resp.tool_calls {
            let input: Value = serde_json::from_str(&tc.arguments).unwrap_or_else(|_| json!({}));
            blocks.push(json!({"type": "tool_use", "id": tc.id, "name": tc.name, "input": input}));
        }
        let id = if resp.id.is_empty() {
            gen_id()
        } else {
            resp.id.clone()
        };
        let usage = resp.usage.clone().unwrap_or_default();
        json!({
            "id": id,
            "type": "message",
            "role": "assistant",
            "model": resp.model,
            "content": blocks,
            "stop_reason": finish_to_anth(resp.finish_reason.as_deref()),
            "stop_sequence": Value::Null,
            "usage": {
                "input_tokens": anth_net_input(&usage),
                "output_tokens": usage.completion_tokens,
                "cache_read_input_tokens": usage.cached_tokens,
                "cache_creation_input_tokens": usage.cache_write_tokens,
            },
        })
    }

    fn render_stream(&self, ev: &StreamEvent, st: &mut RenderState) -> Vec<SseChunk> {
        let mut out = Vec::new();
        ensure_message_start(st, &mut out);
        match ev {
            StreamEvent::ContentDelta(t) => {
                open_text_like(st, &mut out, "text");
                out.push(SseChunk::named(
                    "content_block_delta",
                    json!({"type": "content_block_delta", "index": st.anth_open_index,
                        "delta": {"type": "text_delta", "text": t}})
                    .to_string(),
                ));
            }
            StreamEvent::ReasoningDelta(t) => {
                open_text_like(st, &mut out, "thinking");
                out.push(SseChunk::named(
                    "content_block_delta",
                    json!({"type": "content_block_delta", "index": st.anth_open_index,
                        "delta": {"type": "thinking_delta", "thinking": t}})
                    .to_string(),
                ));
            }
            StreamEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments,
            } => {
                let block = open_tool_block(st, &mut out, *index, id.as_deref(), name.as_deref());
                if !arguments.is_empty() {
                    out.push(SseChunk::named(
                        "content_block_delta",
                        json!({"type": "content_block_delta", "index": block,
                            "delta": {"type": "input_json_delta", "partial_json": arguments}})
                        .to_string(),
                    ));
                }
            }
            StreamEvent::Finish { reason } => st.finish_reason = reason.clone(),
            StreamEvent::Usage(u) => st.usage = Some(u.clone()),
            StreamEvent::Error { message } => {
                close_open_block(st, &mut out);
                out.push(SseChunk::named(
                    "error",
                    json!({"type": "error", "error": {"type": "api_error", "message": message}})
                        .to_string(),
                ));
                out.push(SseChunk::named(
                    "message_stop",
                    json!({"type": "message_stop"}).to_string(),
                ));
            }
            StreamEvent::Done => {
                close_open_block(st, &mut out);
                let out_tokens = st.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
                out.push(SseChunk::named(
                    "message_delta",
                    json!({"type": "message_delta",
                        "delta": {"stop_reason": finish_to_anth(st.finish_reason.as_deref()), "stop_sequence": Value::Null},
                        "usage": {"output_tokens": out_tokens}})
                    .to_string(),
                ));
                out.push(SseChunk::named(
                    "message_stop",
                    json!({"type": "message_stop"}).to_string(),
                ));
            }
        }
        out
    }
}

fn ensure_message_start(st: &mut RenderState, out: &mut Vec<SseChunk>) {
    if st.anth_message_started {
        return;
    }
    st.anth_message_started = true;
    if st.id.is_empty() {
        st.id = gen_id();
    }
    let (in_tokens, cache_read, cache_write) = st
        .usage
        .as_ref()
        .map(|u| (anth_net_input(u), u.cached_tokens, u.cache_write_tokens))
        .unwrap_or((0, 0, 0));
    out.push(SseChunk::named(
        "message_start",
        json!({"type": "message_start", "message": {
            "id": st.id, "type": "message", "role": "assistant", "model": st.model,
            "content": [], "stop_reason": Value::Null, "stop_sequence": Value::Null,
            "usage": {"input_tokens": in_tokens, "output_tokens": 0,
                "cache_read_input_tokens": cache_read, "cache_creation_input_tokens": cache_write}
        }})
        .to_string(),
    ));
}

/// Anthropic 净输入 token = 全量 prompt 扣除读/写缓存（还原其原生 usage 语义）
fn anth_net_input(u: &Usage) -> u64 {
    u.prompt_tokens
        .saturating_sub(u.cached_tokens)
        .saturating_sub(u.cache_write_tokens)
}

fn close_open_block(st: &mut RenderState, out: &mut Vec<SseChunk>) {
    if st.anth_open_kind.is_empty() {
        return;
    }
    out.push(SseChunk::named(
        "content_block_stop",
        json!({"type": "content_block_stop", "index": st.anth_open_index}).to_string(),
    ));
    st.anth_open_kind.clear();
}

/// 打开 text / thinking 块（已打开同类则复用）
fn open_text_like(st: &mut RenderState, out: &mut Vec<SseChunk>, kind: &str) {
    if st.anth_open_kind == kind {
        return;
    }
    close_open_block(st, out);
    let index = st.anth_next_block;
    st.anth_next_block += 1;
    st.anth_open_kind = kind.to_string();
    st.anth_open_index = index;
    let block = if kind == "thinking" {
        json!({"type": "thinking", "thinking": ""})
    } else {
        json!({"type": "text", "text": ""})
    };
    out.push(SseChunk::named(
        "content_block_start",
        json!({"type": "content_block_start", "index": index, "content_block": block}).to_string(),
    ));
}

/// 打开/复用 tool_use 块，返回其 block index
fn open_tool_block(
    st: &mut RenderState,
    out: &mut Vec<SseChunk>,
    tool_index: usize,
    id: Option<&str>,
    name: Option<&str>,
) -> i64 {
    if let Some(&block) = st.anth_tool_blocks.get(&tool_index) {
        return block;
    }
    close_open_block(st, out);
    let index = st.anth_next_block;
    st.anth_next_block += 1;
    st.anth_open_kind = "tool".to_string();
    st.anth_open_index = index;
    st.anth_tool_blocks.insert(tool_index, index);
    out.push(SseChunk::named(
        "content_block_start",
        json!({"type": "content_block_start", "index": index, "content_block": {
            "type": "tool_use", "id": id.unwrap_or(""), "name": name.unwrap_or(""), "input": {}
        }})
        .to_string(),
    ));
    index
}

/// canonical 内容分片 → Anthropic content block 数组
fn parts_to_anth_blocks(parts: &[ContentPart]) -> Vec<Value> {
    parts
        .iter()
        .map(|p| match p {
            ContentPart::Text(t) => json!({"type": "text", "text": t}),
            ContentPart::ImageUrl { url, .. } => {
                if let Some(rest) = url.strip_prefix("data:") {
                    if let Some((media, data)) = rest.split_once(";base64,") {
                        return json!({"type": "image", "source": {
                            "type": "base64", "media_type": media, "data": data
                        }});
                    }
                }
                json!({"type": "image", "source": {"type": "url", "url": url}})
            }
        })
        .collect()
}

/// assistant 消息 → Anthropic content block 数组（thinking + text + tool_use）
fn assistant_to_anth_blocks(msg: &Message) -> Vec<Value> {
    let mut blocks = Vec::new();
    if let Some(r) = &msg.reasoning {
        if !r.is_empty() {
            blocks.push(json!({"type": "thinking", "thinking": r}));
        }
    }
    blocks.extend(parts_to_anth_blocks(&msg.content));
    for tc in &msg.tool_calls {
        let input: Value = serde_json::from_str(&tc.arguments).unwrap_or_else(|_| json!({}));
        blocks.push(json!({"type": "tool_use", "id": tc.id, "name": tc.name, "input": input}));
    }
    blocks
}

/// 将 canonical 消息列表转为 Anthropic（system 抽出，tool 并入 user 的 tool_result）
fn build_anth_messages(req: &CanonicalRequest) -> (String, Vec<Value>) {
    let mut system = String::new();
    let mut messages: Vec<Value> = Vec::new();
    for msg in &req.messages {
        match msg.role {
            Role::System => {
                let t = msg.text();
                if !t.is_empty() {
                    if !system.is_empty() {
                        system.push('\n');
                    }
                    system.push_str(&t);
                }
            }
            Role::Tool => {
                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": msg.tool_call_id.clone().unwrap_or_default(),
                    "content": msg.text(),
                });
                match messages.last_mut() {
                    Some(last)
                        if last.get("role").and_then(|r| r.as_str()) == Some("user")
                            && last.get("content").map(|c| c.is_array()).unwrap_or(false) =>
                    {
                        last["content"].as_array_mut().unwrap().push(block);
                    }
                    _ => messages.push(json!({"role": "user", "content": [block]})),
                }
            }
            Role::User => {
                messages
                    .push(json!({"role": "user", "content": parts_to_anth_blocks(&msg.content)}));
            }
            Role::Assistant => {
                messages
                    .push(json!({"role": "assistant", "content": assistant_to_anth_blocks(msg)}));
            }
        }
    }
    (system, messages)
}

/// canonical(OpenAI) tool_choice → Anthropic tool_choice
fn tool_choice_to_anth(tc: &Value) -> Value {
    match tc {
        Value::String(s) if s == "required" => json!({"type": "any"}),
        Value::String(s) if s == "none" => json!({"type": "none"}),
        Value::String(_) => json!({"type": "auto"}),
        Value::Object(o) => {
            let name = o
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str());
            match name {
                Some(n) => json!({"type": "tool", "name": n}),
                None => json!({"type": "auto"}),
            }
        }
        _ => json!({"type": "auto"}),
    }
}

impl OutboundTranslator for Anthropic {
    fn build_request(&self, req: &CanonicalRequest) -> Value {
        let (system, messages) = build_anth_messages(req);
        let mut obj = serde_json::Map::new();
        obj.insert("model".into(), json!(req.model));
        obj.insert("stream".into(), json!(req.stream));
        obj.insert("max_tokens".into(), json!(req.max_tokens.unwrap_or(4096)));
        if !system.is_empty() {
            obj.insert("system".into(), json!(system));
        }
        obj.insert("messages".into(), Value::Array(messages));
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description.clone().unwrap_or_default(),
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            obj.insert("tools".into(), Value::Array(tools));
        }
        if let Some(tc) = &req.tool_choice {
            obj.insert("tool_choice".into(), tool_choice_to_anth(tc));
        }
        if let Some(t) = req.temperature {
            obj.insert("temperature".into(), json!(t));
        }
        if let Some(p) = req.top_p {
            obj.insert("top_p".into(), json!(p));
        }
        if let Some(s) = &req.stop {
            obj.insert("stop_sequences".into(), s.clone());
        }
        Value::Object(obj)
    }

    fn parse_response(&self, body: &Value) -> Result<CanonicalResponse, String> {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        if let Some(blocks) = body.get("content").and_then(|c| c.as_array()) {
            for b in blocks {
                match b.get("type").and_then(|t| t.as_str()) {
                    Some("text") => {
                        if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                            content.push_str(t);
                        }
                    }
                    Some("thinking") => {
                        if let Some(t) = b.get("thinking").and_then(|t| t.as_str()) {
                            reasoning.push_str(t);
                        }
                    }
                    Some("tool_use") => tool_calls.push(ToolCall {
                        id: b
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        name: b
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        arguments: b
                            .get("input")
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "{}".into()),
                    }),
                    _ => {}
                }
            }
        }
        let usage = body.get("usage").map(parse_anth_usage);
        Ok(CanonicalResponse {
            id: body
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            model: body
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            content,
            reasoning: if reasoning.is_empty() {
                None
            } else {
                Some(reasoning)
            },
            tool_calls,
            finish_reason: body
                .get("stop_reason")
                .and_then(|s| s.as_str())
                .map(|s| anth_to_finish(s).to_string()),
            usage,
        })
    }

    fn parse_stream(
        &self,
        _event: Option<&str>,
        data: &str,
        st: &mut ParseState,
    ) -> Vec<StreamEvent> {
        let Ok(v) = serde_json::from_str::<Value>(data.trim()) else {
            return Vec::new();
        };
        let mut events = Vec::new();
        match v.get("type").and_then(|t| t.as_str()) {
            Some("message_start") => {
                if let Some(u) = v.get("message").and_then(|m| m.get("usage")) {
                    let parsed = parse_anth_usage(u);
                    st.usage.prompt_tokens = parsed.prompt_tokens;
                    st.usage.cached_tokens = parsed.cached_tokens;
                    st.usage.cache_write_tokens = parsed.cache_write_tokens;
                }
            }
            Some("content_block_start") => {
                let idx = v.get("index").and_then(|i| i.as_i64()).unwrap_or(0);
                let block = v.get("content_block");
                let kind = block
                    .and_then(|b| b.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut info = super::AnthBlock {
                    kind: kind.clone(),
                    tool_index: 0,
                };
                if kind == "tool_use" {
                    info.tool_index = st.tool_seq;
                    st.tool_seq += 1;
                    events.push(StreamEvent::ToolCallDelta {
                        index: info.tool_index,
                        id: block
                            .and_then(|b| b.get("id"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        name: block
                            .and_then(|b| b.get("name"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        arguments: String::new(),
                    });
                }
                st.anth_blocks.insert(idx, info);
            }
            Some("content_block_delta") => {
                let idx = v.get("index").and_then(|i| i.as_i64()).unwrap_or(0);
                let delta = v.get("delta");
                match delta.and_then(|d| d.get("type")).and_then(|t| t.as_str()) {
                    Some("text_delta") => {
                        if let Some(t) = delta.and_then(|d| d.get("text")).and_then(|t| t.as_str())
                        {
                            events.push(StreamEvent::ContentDelta(t.to_string()));
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(t) = delta
                            .and_then(|d| d.get("thinking"))
                            .and_then(|t| t.as_str())
                        {
                            events.push(StreamEvent::ReasoningDelta(t.to_string()));
                        }
                    }
                    Some("input_json_delta") => {
                        let tool_index =
                            st.anth_blocks.get(&idx).map(|b| b.tool_index).unwrap_or(0);
                        if let Some(pj) = delta
                            .and_then(|d| d.get("partial_json"))
                            .and_then(|t| t.as_str())
                        {
                            events.push(StreamEvent::ToolCallDelta {
                                index: tool_index,
                                id: None,
                                name: None,
                                arguments: pj.to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            Some("message_delta") => {
                if let Some(reason) = v
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(|s| s.as_str())
                {
                    events.push(StreamEvent::Finish {
                        reason: Some(anth_to_finish(reason).to_string()),
                    });
                }
                if let Some(out_tokens) = v
                    .get("usage")
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|x| x.as_u64())
                {
                    st.usage.completion_tokens = out_tokens;
                    st.usage.total_tokens = st.usage.prompt_tokens + out_tokens;
                    events.push(StreamEvent::Usage(st.usage.clone()));
                }
            }
            Some("message_stop") => events.push(StreamEvent::Done),
            _ => {}
        }
        events
    }
}

/// 解析 Anthropic usage 对象
///
/// 注意：Anthropic 的 `input_tokens` 不含缓存读/写 token，故 `prompt_tokens`
/// 归一为「净输入 + 读缓存 + 写缓存」全量，与 OpenAI 系语义对齐。
fn parse_anth_usage(u: &Value) -> Usage {
    let input = u.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let cached = u
        .get("cache_read_input_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let cache_write = parse_anth_cache_write(u);
    let completion = u.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let prompt = input + cached + cache_write;
    Usage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        cached_tokens: cached,
        cache_write_tokens: cache_write,
        reasoning_tokens: 0,
    }
}

/// 解析写缓存 token：优先取 `cache_creation` 细分（5m/1h）之和，回退到 `cache_creation_input_tokens`
fn parse_anth_cache_write(u: &Value) -> u64 {
    if let Some(c) = u.get("cache_creation").filter(|c| !c.is_null()) {
        let m5 = c
            .get("ephemeral_5m_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let h1 = c
            .get("ephemeral_1h_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        if m5 + h1 > 0 {
            return m5 + h1;
        }
    }
    u.get("cache_creation_input_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(0)
}
