//! OpenAI Responses 协议转换。
//!
//! 既作入站协议（/gateway/v1/responses），也作 `codex` 渠道的出站协议（线型）。
//! Responses↔Codex 同线型时走直转短路，不经此映射；此处用于跨协议互转。

use serde_json::{Map, Value, json};

use super::{InboundTranslator, OutboundTranslator, ParseState, RenderState, SseChunk};
use crate::core::gateway::canonical::{
    CanonicalRequest, CanonicalResponse, ContentPart, Message, Role, StreamEvent, Tool, ToolCall,
    Usage,
};

pub struct OpenAiResponses;

fn gen_id() -> String {
    format!("resp_{}", chrono::Utc::now().timestamp_millis())
}

/// 解析 Responses content（数组）→ 文本/图像分片
fn parse_content(value: &Value) -> Vec<ContentPart> {
    match value {
        Value::String(s) => vec![ContentPart::Text(s.clone())],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|p| match p.get("type").and_then(|t| t.as_str()) {
                Some("input_text") | Some("output_text") | Some("text") => p
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|t| ContentPart::Text(t.to_string())),
                Some("input_image") => {
                    let url = p
                        .get("image_url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(ContentPart::ImageUrl { url, detail: None })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 解析 Responses tools（{type:function, name, description, parameters}）
fn parse_tools(value: &Value) -> Vec<Tool> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let name = t.get("name").and_then(|v| v.as_str())?.to_string();
                    Some(Tool {
                        name,
                        description: t
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        parameters: t
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 解析 Responses input 项为 canonical 消息（function_call_output → tool 消息）
fn parse_input_item(out: &mut Vec<Message>, item: &Value) {
    match item.get("type").and_then(|t| t.as_str()) {
        Some("function_call") => {
            let mut m = Message::new(Role::Assistant);
            m.tool_calls.push(ToolCall {
                id: item
                    .get("call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                arguments: item
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string(),
            });
            out.push(m);
        }
        Some("function_call_output") => {
            let mut m = Message::new(Role::Tool);
            m.tool_call_id = item
                .get("call_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let output = item.get("output");
            m.content = match output {
                Some(Value::String(s)) => vec![ContentPart::Text(s.clone())],
                Some(other) => parse_content(other),
                None => Vec::new(),
            };
            out.push(m);
        }
        Some("reasoning") => {}
        _ => {
            let role = Role::from_str(item.get("role").and_then(|r| r.as_str()).unwrap_or("user"));
            let mut m = Message::new(role);
            if let Some(c) = item.get("content") {
                m.content = parse_content(c);
            }
            if !m.content.is_empty() {
                out.push(m);
            }
        }
    }
}

impl InboundTranslator for OpenAiResponses {
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
            temperature: obj.get("temperature").and_then(|v| v.as_f64()),
            top_p: obj.get("top_p").and_then(|v| v.as_f64()),
            max_tokens: obj.get("max_output_tokens").and_then(|v| v.as_u64()),
            ..Default::default()
        };
        if let Some(instr) = obj.get("instructions").and_then(|v| v.as_str()) {
            if !instr.is_empty() {
                let mut m = Message::new(Role::System);
                m.content = vec![ContentPart::Text(instr.to_string())];
                req.messages.push(m);
            }
        }
        match obj.get("input") {
            Some(Value::String(s)) => {
                let mut m = Message::new(Role::User);
                m.content = vec![ContentPart::Text(s.clone())];
                req.messages.push(m);
            }
            Some(Value::Array(items)) => {
                for item in items {
                    parse_input_item(&mut req.messages, item);
                }
            }
            _ => {}
        }
        if let Some(tools) = obj.get("tools") {
            req.tools = parse_tools(tools);
        }
        req.tool_choice = obj.get("tool_choice").cloned();
        req.reasoning_effort = obj
            .get("reasoning")
            .and_then(|r| r.get("effort"))
            .and_then(|e| e.as_str())
            .map(String::from);
        Ok(req)
    }

    fn render_response(&self, resp: &CanonicalResponse) -> Value {
        let id = if resp.id.is_empty() {
            gen_id()
        } else {
            resp.id.clone()
        };
        let mut output: Vec<Value> = Vec::new();
        if let Some(r) = &resp.reasoning {
            if !r.is_empty() {
                output.push(json!({
                    "type": "reasoning",
                    "id": format!("{}_rs", id),
                    "summary": [{"type": "summary_text", "text": r}],
                }));
            }
        }
        if !resp.content.is_empty() {
            output.push(json!({
                "type": "message",
                "id": format!("{}_msg", id),
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": resp.content, "annotations": []}],
            }));
        }
        for (i, tc) in resp.tool_calls.iter().enumerate() {
            output.push(json!({
                "type": "function_call",
                "id": format!("{}_fc_{}", id, i),
                "call_id": tc.id,
                "name": tc.name,
                "arguments": tc.arguments,
                "status": "completed",
            }));
        }
        json!({
            "id": id,
            "object": "response",
            "created_at": chrono::Utc::now().timestamp(),
            "model": resp.model,
            "status": "completed",
            "output": output,
            "usage": usage_to_value(resp.usage.as_ref()),
        })
    }

    fn render_stream(&self, ev: &StreamEvent, st: &mut RenderState) -> Vec<SseChunk> {
        render_stream_impl(ev, st)
    }
}

/// usage → Responses usage 对象
fn usage_to_value(usage: Option<&Usage>) -> Value {
    let u = usage.cloned().unwrap_or_default();
    json!({
        "input_tokens": u.prompt_tokens,
        "input_tokens_details": {"cached_tokens": u.cached_tokens},
        "output_tokens": u.completion_tokens,
        "output_tokens_details": {"reasoning_tokens": u.reasoning_tokens},
        "total_tokens": u.total_tokens,
    })
}

fn response_has_tool_call(resp: &Value) -> bool {
    resp.get("output")
        .and_then(|o| o.as_array())
        .map(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(|t| t.as_str()) == Some("function_call"))
        })
        .unwrap_or(false)
}

fn infer_finish_reason(resp: &Value) -> &'static str {
    if response_has_tool_call(resp) {
        return "tool_calls";
    }

    if resp.get("status").and_then(|s| s.as_str()) == Some("incomplete") {
        return match resp
            .get("incomplete_details")
            .and_then(|d| d.get("reason"))
            .and_then(|r| r.as_str())
        {
            Some("max_output_tokens") => "length",
            Some("content_filter") => "content_filter",
            _ => "length",
        };
    }

    "stop"
}

fn response_error_message(event: &Value) -> String {
    event
        .get("response")
        .and_then(|r| r.get("error"))
        .or_else(|| event.get("error"))
        .and_then(|e| {
            e.get("message")
                .and_then(|m| m.as_str())
                .or_else(|| e.as_str())
        })
        .or_else(|| event.get("message").and_then(|m| m.as_str()))
        .unwrap_or("upstream response failed")
        .to_string()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 构造带自增 sequence_number 的 Responses SSE 事件
fn ev(st: &mut RenderState, name: &str, mut data: Value) -> SseChunk {
    data["sequence_number"] = json!(st.resp.seq);
    st.resp.seq += 1;
    SseChunk::named(name, data.to_string())
}

/// 顶层 response 信封
fn envelope(st: &RenderState, status: &str, output: Value, usage: Value) -> Value {
    json!({
        "id": st.id,
        "object": "response",
        "created_at": st.created,
        "model": st.model,
        "status": status,
        "output": output,
        "usage": usage,
    })
}

fn finalize_reasoning(st: &mut RenderState, out: &mut Vec<SseChunk>) {
    if let Some(oi) = st.resp.reasoning_item.take() {
        let item_id = format!("{}_rs", st.id);
        let text = st.resp.reasoning_buf.clone();
        out.push(ev(
            st,
            "response.reasoning_summary_text.done",
            json!({
            "type": "response.reasoning_summary_text.done", "item_id": item_id,
            "output_index": oi, "summary_index": 0, "text": text}),
        ));
        out.push(ev(
            st,
            "response.reasoning_summary_part.done",
            json!({
            "type": "response.reasoning_summary_part.done", "item_id": item_id,
            "output_index": oi, "summary_index": 0,
            "part": {"type": "summary_text", "text": st.resp.reasoning_buf.clone()}}),
        ));
        out.push(ev(
            st,
            "response.output_item.done",
            json!({
            "type": "response.output_item.done", "output_index": oi,
            "item": {"type": "reasoning", "id": item_id,
                "summary": [{"type": "summary_text", "text": st.resp.reasoning_buf.clone()}]}}),
        ));
    }
}

fn finalize_text(st: &mut RenderState, out: &mut Vec<SseChunk>) {
    if let Some(oi) = st.resp.text_item.take() {
        let item_id = format!("{}_msg", st.id);
        let text = st.resp.text_buf.clone();
        out.push(ev(
            st,
            "response.output_text.done",
            json!({
            "type": "response.output_text.done", "item_id": item_id,
            "output_index": oi, "content_index": 0, "text": text}),
        ));
        out.push(ev(
            st,
            "response.content_part.done",
            json!({
            "type": "response.content_part.done", "item_id": item_id,
            "output_index": oi, "content_index": 0,
            "part": {"type": "output_text", "text": st.resp.text_buf.clone(), "annotations": []}}),
        ));
        out.push(ev(st, "response.output_item.done", json!({
            "type": "response.output_item.done", "output_index": oi,
            "item": {"type": "message", "id": item_id, "status": "completed", "role": "assistant",
                "content": [{"type": "output_text", "text": st.resp.text_buf.clone(), "annotations": []}]}})));
    }
}

fn finalize_tools(st: &mut RenderState, out: &mut Vec<SseChunk>) {
    let order = st.resp.tool_order.clone();
    for idx in order {
        let Some(tool) = st.resp.tools.get(&idx).cloned() else {
            continue;
        };
        let fc_id = format!("{}_fc_{}", st.id, idx);
        out.push(ev(
            st,
            "response.function_call_arguments.done",
            json!({
            "type": "response.function_call_arguments.done", "item_id": fc_id,
            "output_index": tool.output_index, "arguments": tool.args}),
        ));
        out.push(ev(
            st,
            "response.output_item.done",
            json!({
            "type": "response.output_item.done", "output_index": tool.output_index,
            "item": {"type": "function_call", "id": fc_id, "call_id": tool.id,
                "name": tool.name, "arguments": tool.args, "status": "completed"}}),
        ));
    }
}

/// 由累积缓冲重建最终 output 数组
fn build_final_output(st: &RenderState) -> Value {
    let mut output = Vec::new();
    if !st.resp.reasoning_buf.is_empty() {
        output.push(json!({"type": "reasoning", "id": format!("{}_rs", st.id),
            "summary": [{"type": "summary_text", "text": st.resp.reasoning_buf}]}));
    }
    if !st.resp.text_buf.is_empty() {
        output.push(json!({"type": "message", "id": format!("{}_msg", st.id),
            "status": "completed", "role": "assistant",
            "content": [{"type": "output_text", "text": st.resp.text_buf, "annotations": []}]}));
    }
    for idx in &st.resp.tool_order {
        if let Some(tool) = st.resp.tools.get(idx) {
            output.push(json!({"type": "function_call", "id": format!("{}_fc_{}", st.id, idx),
                "call_id": tool.id, "name": tool.name, "arguments": tool.args, "status": "completed"}));
        }
    }
    Value::Array(output)
}

fn render_stream_impl(event: &StreamEvent, st: &mut RenderState) -> Vec<SseChunk> {
    let mut out = Vec::new();
    if !st.started {
        st.started = true;
        if st.id.is_empty() {
            st.id = gen_id();
        }
        if st.created == 0 {
            st.created = now();
        }
        let env = envelope(st, "in_progress", json!([]), Value::Null);
        out.push(ev(
            st,
            "response.created",
            json!({"type": "response.created", "response": env}),
        ));
    }
    match event {
        StreamEvent::ContentDelta(t) => {
            if st.resp.text_item.is_none() {
                finalize_reasoning(st, &mut out);
                let oi = st.resp.next_output;
                st.resp.next_output += 1;
                st.resp.text_item = Some(oi);
                let item_id = format!("{}_msg", st.id);
                out.push(ev(
                    st,
                    "response.output_item.added",
                    json!({
                    "type": "response.output_item.added", "output_index": oi,
                    "item": {"type": "message", "id": item_id, "status": "in_progress",
                        "role": "assistant", "content": []}}),
                ));
                out.push(ev(st, "response.content_part.added", json!({
                    "type": "response.content_part.added", "item_id": item_id, "output_index": oi,
                    "content_index": 0, "part": {"type": "output_text", "text": "", "annotations": []}})));
            }
            st.resp.text_buf.push_str(t);
            let oi = st.resp.text_item.unwrap();
            let item_id = format!("{}_msg", st.id);
            out.push(ev(
                st,
                "response.output_text.delta",
                json!({
                "type": "response.output_text.delta", "item_id": item_id, "output_index": oi,
                "content_index": 0, "delta": t}),
            ));
        }
        StreamEvent::ReasoningDelta(t) => {
            if st.resp.reasoning_item.is_none() {
                let oi = st.resp.next_output;
                st.resp.next_output += 1;
                st.resp.reasoning_item = Some(oi);
                let item_id = format!("{}_rs", st.id);
                out.push(ev(
                    st,
                    "response.output_item.added",
                    json!({
                    "type": "response.output_item.added", "output_index": oi,
                    "item": {"type": "reasoning", "id": item_id, "summary": []}}),
                ));
                out.push(ev(
                    st,
                    "response.reasoning_summary_part.added",
                    json!({
                    "type": "response.reasoning_summary_part.added", "item_id": item_id,
                    "output_index": oi, "summary_index": 0,
                    "part": {"type": "summary_text", "text": ""}}),
                ));
            }
            st.resp.reasoning_buf.push_str(t);
            let oi = st.resp.reasoning_item.unwrap();
            let item_id = format!("{}_rs", st.id);
            out.push(ev(
                st,
                "response.reasoning_summary_text.delta",
                json!({
                "type": "response.reasoning_summary_text.delta", "item_id": item_id,
                "output_index": oi, "summary_index": 0, "delta": t}),
            ));
        }
        StreamEvent::ToolCallDelta {
            index,
            id,
            name,
            arguments,
        } => {
            if !st.resp.tools.contains_key(index) {
                finalize_reasoning(st, &mut out);
                finalize_text(st, &mut out);
                let oi = st.resp.next_output;
                st.resp.next_output += 1;
                st.resp.tool_order.push(*index);
                st.resp.tools.insert(
                    *index,
                    super::RespTool {
                        output_index: oi,
                        id: id.clone().unwrap_or_default(),
                        name: name.clone().unwrap_or_default(),
                        args: String::new(),
                    },
                );
                let fc_id = format!("{}_fc_{}", st.id, index);
                out.push(ev(
                    st,
                    "response.output_item.added",
                    json!({
                    "type": "response.output_item.added", "output_index": oi,
                    "item": {"type": "function_call", "id": fc_id,
                        "call_id": id.clone().unwrap_or_default(),
                        "name": name.clone().unwrap_or_default(), "arguments": "",
                        "status": "in_progress"}}),
                ));
            }
            let (oi, fc_id) = {
                let tool = st.resp.tools.get_mut(index).unwrap();
                if let Some(i) = id {
                    tool.id = i.clone();
                }
                if let Some(n) = name {
                    tool.name = n.clone();
                }
                tool.args.push_str(arguments);
                (tool.output_index, format!("{}_fc_{}", st.id, index))
            };
            if !arguments.is_empty() {
                out.push(ev(
                    st,
                    "response.function_call_arguments.delta",
                    json!({
                    "type": "response.function_call_arguments.delta", "item_id": fc_id,
                    "output_index": oi, "delta": arguments}),
                ));
            }
        }
        StreamEvent::Finish { reason } => st.finish_reason = reason.clone(),
        StreamEvent::Usage(u) => st.usage = Some(u.clone()),
        StreamEvent::Error { message } => {
            finalize_reasoning(st, &mut out);
            finalize_text(st, &mut out);
            finalize_tools(st, &mut out);
            let output = build_final_output(st);
            let usage = usage_to_value(st.usage.as_ref());
            let env = envelope(st, "failed", output, usage);
            out.push(ev(
                st,
                "response.failed",
                json!({"type": "response.failed", "response": env,
                    "error": {"type": "server_error", "message": message}}),
            ));
        }
        StreamEvent::Done => {
            finalize_reasoning(st, &mut out);
            finalize_text(st, &mut out);
            finalize_tools(st, &mut out);
            let output = build_final_output(st);
            let usage = usage_to_value(st.usage.as_ref());
            let env = envelope(st, "completed", output, usage);
            out.push(ev(
                st,
                "response.completed",
                json!({"type": "response.completed", "response": env}),
            ));
        }
    }
    out
}

/// canonical 内容分片 → Responses content（区分 input/output 文本类型）
fn parts_to_resp_content(parts: &[ContentPart], output: bool) -> Vec<Value> {
    let text_type = if output { "output_text" } else { "input_text" };
    parts
        .iter()
        .map(|p| match p {
            ContentPart::Text(t) => json!({"type": text_type, "text": t}),
            ContentPart::ImageUrl { url, .. } => json!({"type": "input_image", "image_url": url}),
        })
        .collect()
}

/// canonical 消息列表 → Responses input 项数组（system 抽出为 instructions）
fn build_resp_input(req: &CanonicalRequest) -> (String, Vec<Value>) {
    let mut instructions = String::new();
    let mut input: Vec<Value> = Vec::new();
    for msg in &req.messages {
        match msg.role {
            Role::System => {
                let t = msg.text();
                if !t.is_empty() {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(&t);
                }
            }
            Role::Tool => input.push(json!({
                "type": "function_call_output",
                "call_id": msg.tool_call_id.clone().unwrap_or_default(),
                "output": msg.text(),
            })),
            Role::User => {
                if !msg.content.is_empty() {
                    input.push(json!({"type": "message", "role": "user",
                        "content": parts_to_resp_content(&msg.content, false)}));
                }
            }
            Role::Assistant => {
                if !msg.content.is_empty() {
                    input.push(json!({"type": "message", "role": "assistant",
                        "content": parts_to_resp_content(&msg.content, true)}));
                }
                for tc in &msg.tool_calls {
                    input.push(json!({"type": "function_call", "call_id": tc.id,
                        "name": tc.name, "arguments": tc.arguments}));
                }
            }
        }
    }
    (instructions, input)
}

impl OutboundTranslator for OpenAiResponses {
    fn build_request(&self, req: &CanonicalRequest) -> Value {
        let (instructions, input) = build_resp_input(req);
        let mut obj = Map::new();
        obj.insert("model".into(), json!(req.model));
        obj.insert("stream".into(), json!(req.stream));
        obj.insert("input".into(), Value::Array(input));
        if !instructions.is_empty() {
            obj.insert("instructions".into(), json!(instructions));
        }
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "name": t.name,
                        "description": t.description.clone().unwrap_or_default(),
                        "parameters": t.parameters,
                    })
                })
                .collect();
            obj.insert("tools".into(), Value::Array(tools));
        }
        if let Some(tc) = &req.tool_choice {
            obj.insert("tool_choice".into(), tc.clone());
        }
        if let Some(t) = req.temperature {
            obj.insert("temperature".into(), json!(t));
        }
        if let Some(p) = req.top_p {
            obj.insert("top_p".into(), json!(p));
        }
        if let Some(m) = req.max_tokens {
            obj.insert("max_output_tokens".into(), json!(m));
        }
        if let Some(e) = &req.reasoning_effort {
            obj.insert("reasoning".into(), json!({"effort": e}));
        }
        Value::Object(obj)
    }

    fn parse_response(&self, body: &Value) -> Result<CanonicalResponse, String> {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        if let Some(items) = body.get("output").and_then(|o| o.as_array()) {
            for item in items {
                match item.get("type").and_then(|t| t.as_str()) {
                    Some("message") => {
                        if let Some(parts) = item.get("content").and_then(|c| c.as_array()) {
                            for p in parts {
                                if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                                    content.push_str(t);
                                }
                            }
                        }
                    }
                    Some("reasoning") => {
                        if let Some(sum) = item.get("summary").and_then(|s| s.as_array()) {
                            for s in sum {
                                if let Some(t) = s.get("text").and_then(|t| t.as_str()) {
                                    reasoning.push_str(t);
                                }
                            }
                        }
                    }
                    Some("function_call") => tool_calls.push(ToolCall {
                        id: item
                            .get("call_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        name: item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        arguments: item
                            .get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}")
                            .to_string(),
                    }),
                    _ => {}
                }
            }
        }
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
            finish_reason: Some(infer_finish_reason(body).to_string()),
            usage: body.get("usage").map(parse_resp_usage),
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
            Some("response.output_text.delta") => {
                if let Some(d) = v.get("delta").and_then(|d| d.as_str()) {
                    events.push(StreamEvent::ContentDelta(d.to_string()));
                }
            }
            Some("response.reasoning_summary_text.delta") => {
                if let Some(d) = v.get("delta").and_then(|d| d.as_str()) {
                    events.push(StreamEvent::ReasoningDelta(d.to_string()));
                }
            }
            Some("response.output_item.added") => {
                let item = v.get("item");
                if item.and_then(|i| i.get("type")).and_then(|t| t.as_str())
                    == Some("function_call")
                {
                    let key = v
                        .get("output_index")
                        .map(|o| o.to_string())
                        .unwrap_or_default();
                    let tool_index = st.tool_seq;
                    st.tool_seq += 1;
                    st.resp_tool_index.insert(key, tool_index);
                    events.push(StreamEvent::ToolCallDelta {
                        index: tool_index,
                        id: item
                            .and_then(|i| i.get("call_id"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        name: item
                            .and_then(|i| i.get("name"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        arguments: String::new(),
                    });
                }
            }
            Some("response.function_call_arguments.delta") => {
                let key = v
                    .get("output_index")
                    .map(|o| o.to_string())
                    .unwrap_or_default();
                let Some(tool_index) = st.resp_tool_index.get(&key).copied() else {
                    return events;
                };
                if let Some(d) = v.get("delta").and_then(|d| d.as_str()) {
                    events.push(StreamEvent::ToolCallDelta {
                        index: tool_index,
                        id: None,
                        name: None,
                        arguments: d.to_string(),
                    });
                }
            }
            Some("response.completed") | Some("response.incomplete") => {
                if let Some(resp) = v.get("response") {
                    events.push(StreamEvent::Finish {
                        reason: Some(infer_finish_reason(resp).to_string()),
                    });
                    if let Some(u) = resp.get("usage") {
                        events.push(StreamEvent::Usage(parse_resp_usage(u)));
                    }
                }
                events.push(StreamEvent::Done);
            }
            Some("response.failed") => {
                events.push(StreamEvent::Error {
                    message: response_error_message(&v),
                });
                events.push(StreamEvent::Done);
            }
            _ => {}
        }
        events
    }
}

/// 解析 Responses usage 对象
fn parse_resp_usage(u: &Value) -> Usage {
    let prompt = u.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let completion = u.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    Usage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: u
            .get("total_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(prompt + completion),
        cached_tokens: u
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
        cache_write_tokens: 0,
        reasoning_tokens: u
            .get("output_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    }
}
