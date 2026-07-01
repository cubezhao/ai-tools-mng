//! Chat Completions ↔ canonical 的底层映射助手（canonical 即 Chat）。

use serde_json::{Map, Value, json};

use crate::core::gateway::canonical::{
    CanonicalRequest, ContentPart, Message, Role, Tool, ToolCall,
};

/// 已显式建模、不进入 `extra` 透传的顶层键
const KNOWN_KEYS: &[&str] = &[
    "model",
    "messages",
    "tools",
    "tool_choice",
    "temperature",
    "top_p",
    "max_tokens",
    "max_completion_tokens",
    "stream",
    "stop",
    "reasoning_effort",
    "stream_options",
];

pub fn parse_content(value: &Value) -> Vec<ContentPart> {
    match value {
        Value::String(s) => vec![ContentPart::Text(s.clone())],
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| {
                let ty = p.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match ty {
                    "text" | "input_text" | "output_text" => p
                        .get("text")
                        .and_then(|t| t.as_str())
                        .map(|t| ContentPart::Text(t.to_string())),
                    "image_url" => {
                        let url = p
                            .get("image_url")
                            .and_then(|u| u.get("url").or(Some(u)))
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        let detail = p
                            .get("image_url")
                            .and_then(|u| u.get("detail"))
                            .and_then(|d| d.as_str())
                            .map(String::from);
                        Some(ContentPart::ImageUrl { url, detail })
                    }
                    _ => None,
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn parse_tool_calls(value: &Value) -> Vec<ToolCall> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|tc| ToolCall {
                    id: tc
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: tc
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    arguments: tc
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_message(value: &Value) -> Message {
    let role = Role::from_str(value.get("role").and_then(|r| r.as_str()).unwrap_or("user"));
    let mut msg = Message::new(role);
    if let Some(c) = value.get("content") {
        msg.content = parse_content(c);
    }
    msg.reasoning = value
        .get("reasoning_content")
        .or_else(|| value.get("reasoning"))
        .and_then(|r| r.as_str())
        .map(String::from);
    if let Some(tc) = value.get("tool_calls") {
        msg.tool_calls = parse_tool_calls(tc);
    }
    msg.tool_call_id = value
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    msg.name = value.get("name").and_then(|v| v.as_str()).map(String::from);
    msg
}

pub fn parse_tools(value: &Value) -> Vec<Tool> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let f = t.get("function").unwrap_or(t);
                    let name = f.get("name").and_then(|v| v.as_str())?.to_string();
                    Some(Tool {
                        name,
                        description: f
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        parameters: f
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 解析 Chat 请求体为 canonical 请求
pub fn parse_chat_request(body: &Value) -> Result<CanonicalRequest, String> {
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
        ..Default::default()
    };
    if let Some(msgs) = obj.get("messages").and_then(|m| m.as_array()) {
        req.messages = msgs.iter().map(parse_message).collect();
    }
    if let Some(tools) = obj.get("tools") {
        req.tools = parse_tools(tools);
    }
    req.tool_choice = obj.get("tool_choice").cloned();
    req.temperature = obj.get("temperature").and_then(|v| v.as_f64());
    req.top_p = obj.get("top_p").and_then(|v| v.as_f64());
    req.max_tokens = obj
        .get("max_tokens")
        .or_else(|| obj.get("max_completion_tokens"))
        .and_then(|v| v.as_u64());
    req.stop = obj.get("stop").cloned();
    req.reasoning_effort = obj
        .get("reasoning_effort")
        .and_then(|v| v.as_str())
        .map(String::from);
    req.extra = collect_extra(obj);
    Ok(req)
}

fn collect_extra(obj: &Map<String, Value>) -> Map<String, Value> {
    obj.iter()
        .filter(|(k, _)| !KNOWN_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

pub fn content_to_value(parts: &[ContentPart]) -> Value {
    let only_text = parts.iter().all(|p| matches!(p, ContentPart::Text(_)));
    if only_text {
        let mut s = String::new();
        for p in parts {
            if let ContentPart::Text(t) = p {
                s.push_str(t);
            }
        }
        return Value::String(s);
    }
    let arr: Vec<Value> = parts
        .iter()
        .map(|p| match p {
            ContentPart::Text(t) => json!({"type": "text", "text": t}),
            ContentPart::ImageUrl { url, detail } => {
                let mut img = json!({"url": url});
                if let Some(d) = detail {
                    img["detail"] = json!(d);
                }
                json!({"type": "image_url", "image_url": img})
            }
        })
        .collect();
    Value::Array(arr)
}

pub fn tool_calls_to_value(calls: &[ToolCall]) -> Value {
    Value::Array(
        calls
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "type": "function",
                    "function": {"name": c.name, "arguments": c.arguments}
                })
            })
            .collect(),
    )
}

pub fn message_to_value(msg: &Message) -> Value {
    let mut m = Map::new();
    m.insert("role".into(), json!(msg.role.as_str()));
    if !msg.content.is_empty() {
        m.insert("content".into(), content_to_value(&msg.content));
    } else if msg.tool_calls.is_empty() {
        m.insert("content".into(), Value::String(String::new()));
    }
    if let Some(r) = &msg.reasoning {
        m.insert("reasoning_content".into(), json!(r));
    }
    if !msg.tool_calls.is_empty() {
        m.insert("tool_calls".into(), tool_calls_to_value(&msg.tool_calls));
    }
    if let Some(id) = &msg.tool_call_id {
        m.insert("tool_call_id".into(), json!(id));
    }
    if let Some(n) = &msg.name {
        m.insert("name".into(), json!(n));
    }
    Value::Object(m)
}

pub fn tools_to_value(tools: &[Tool]) -> Value {
    Value::Array(
        tools
            .iter()
            .map(|t| {
                let mut f = Map::new();
                f.insert("name".into(), json!(t.name));
                if let Some(d) = &t.description {
                    f.insert("description".into(), json!(d));
                }
                f.insert("parameters".into(), t.parameters.clone());
                json!({"type": "function", "function": Value::Object(f)})
            })
            .collect(),
    )
}

/// 将 canonical 请求的通用字段写入 Chat 请求体（不含 model/stream）
pub fn apply_common_fields(obj: &mut Map<String, Value>, req: &CanonicalRequest) {
    obj.insert(
        "messages".into(),
        Value::Array(req.messages.iter().map(message_to_value).collect()),
    );
    if !req.tools.is_empty() {
        obj.insert("tools".into(), tools_to_value(&req.tools));
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
        obj.insert("max_tokens".into(), json!(m));
    }
    if let Some(s) = &req.stop {
        obj.insert("stop".into(), s.clone());
    }
    if let Some(e) = &req.reasoning_effort {
        obj.insert("reasoning_effort".into(), json!(e));
    }
    for (k, v) in &req.extra {
        obj.entry(k.clone()).or_insert_with(|| v.clone());
    }
}
