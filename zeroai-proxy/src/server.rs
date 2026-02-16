use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{get, post},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroai::auth::config::ConfigManager;
use zeroai::mapper::ModelMapper;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub config: ConfigManager,
    pub enabled_models: RwLock<Vec<String>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let config = ConfigManager::default_path();
        let enabled = config.get_enabled_models().unwrap_or_default();

        Ok(Self {
            config,
            enabled_models: RwLock::new(enabled),
        })
    }

    pub async fn refresh_models(&self) {
        let enabled = self.config.get_enabled_models().unwrap_or_default();
        *self.enabled_models.write().await = enabled;
    }

    pub async fn resolve_api_key(&self, provider: &str) -> Option<String> {
        self.config.resolve_api_key(provider).await.ok().flatten()
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

pub async fn run_server(host: &str, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(AppState::new().await?);

    let refresh_config = state.config.clone();
    refresh_config.start_auto_refresh_service(15 * 60, 20 * 60);

    let app = Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("ZeroAI proxy listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// GET /v1/models - OpenAI compatible
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ModelsResponse {
    object: String,
    data: Vec<ModelObject>,
}

#[derive(Serialize)]
struct ModelObject {
    id: String,
    object: String,
    created: i64,
    owned_by: String,
}

async fn list_models(State(state): State<Arc<AppState>>) -> Json<ModelsResponse> {
    let mapper = ModelMapper::new();
    let models = state.enabled_models.read().await;
    let data: Vec<ModelObject> = models
        .iter()
        .map(|full_id| {
            let owner = mapper
                .split_id(full_id)
                .map(|(p, _)| p.to_string())
                .unwrap_or_else(|| "unknown".into());
            ModelObject {
                id: full_id.clone(),
                object: "model".into(),
                created: 0,
                owned_by: owner,
            }
        })
        .collect();

    Json(ModelsResponse {
        object: "list".into(),
        data,
    })
}

// ---------------------------------------------------------------------------
// POST /v1/chat/completions - OpenAI compatible
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(default)]
    stream: Option<bool>,
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    tools: Option<Vec<OpenAITool>>,
}

#[derive(Deserialize)]
struct OpenAIMessage {
    role: String,
    #[serde(default)]
    content: Option<serde_json::Value>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCallReq>>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIToolCallReq {
    id: String,
    function: OpenAIFunctionReq,
}

#[derive(Deserialize)]
struct OpenAIFunctionReq {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAITool {
    function: OpenAIToolFunction,
}

#[derive(Deserialize)]
struct OpenAIToolFunction {
    name: String,
    description: Option<String>,
    parameters: Option<serde_json::Value>,
}

fn convert_messages(msgs: &[OpenAIMessage]) -> (Option<String>, Vec<zeroai::chat::ChatMessage>) {
    use zeroai::chat::{ChatMessage, ToolCall, ToolResponse};

    let mut system = None;
    let mut messages = Vec::new();

    for msg in msgs {
        match msg.role.as_str() {
            "system" => {
                if let Some(content) = &msg.content {
                    system = content.as_str().map(String::from);
                }
            }
            "user" => {
                let text = msg
                    .content
                    .as_ref()
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                messages.push(ChatMessage::user(text));
            }
            "assistant" => {
                if let Some(tcs) = &msg.tool_calls {
                    let calls: Vec<ToolCall> = tcs
                        .iter()
                        .map(|tc| ToolCall {
                            call_id: tc.id.clone(),
                            fn_name: tc.function.name.clone(),
                            fn_arguments: serde_json::from_str(&tc.function.arguments)
                                .unwrap_or(serde_json::json!({})),
                            thought_signatures: None,
                        })
                        .collect();
                    messages.push(ChatMessage::from(calls));
                } else {
                    let text = msg
                        .content
                        .as_ref()
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    messages.push(ChatMessage::assistant(text));
                }
            }
            "tool" => {
                let text = msg
                    .content
                    .as_ref()
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let call_id = msg.tool_call_id.clone().unwrap_or_default();
                messages.push(ChatMessage::from(ToolResponse::new(call_id, text)));
            }
            _ => {}
        }
    }

    (system, messages)
}

fn convert_tools(tools: &[OpenAITool]) -> Vec<zeroai::chat::Tool> {
    tools
        .iter()
        .map(|t| {
            zeroai::chat::Tool::new(t.function.name.clone())
                .with_description(t.function.description.clone().unwrap_or_default())
                .with_schema(t.function.parameters.clone().unwrap_or(json!({})))
        })
        .collect()
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    let mapper = ModelMapper::new();

    let (provider_name, _model_id) = match mapper.split_id(&req.model) {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": {"message": "Invalid model ID format, expected provider/model"}})),
            )
                .into_response();
        }
    };

    let api_key = match state.resolve_api_key(provider_name).await {
        Some(k) => k,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": {"message": format!("No credentials for provider: {}", provider_name)}})),
            )
                .into_response();
        }
    };

    let (system_prompt, messages) = convert_messages(&req.messages);
    let tools = req.tools.as_ref().map(|t| convert_tools(t)).unwrap_or_default();

    let mut chat_req = zeroai::chat::ChatRequest::from_messages(messages);
    if let Some(sys) = system_prompt {
        chat_req = chat_req.with_system(sys);
    }
    if !tools.is_empty() {
        chat_req = chat_req.with_tools(tools);
    }

    let mut chat_opts = zeroai::chat::ChatOptions::default();
    if let Some(temp) = req.temperature {
        chat_opts = chat_opts.with_temperature(temp);
    }
    if let Some(max) = req.max_tokens {
        chat_opts = chat_opts.with_max_tokens(max);
    }

    let client = zeroai::Client::default();

    let adapter_kind = match zeroai::adapter::AdapterKind::from_lower_str(provider_name) {
        Some(ak) => ak,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": {"message": format!("Unknown provider: {}", provider_name)}})),
            )
                .into_response();
        }
    };
    let model_iden = zeroai::ModelIden::new(adapter_kind, _model_id);
    let target = zeroai::ServiceTarget {
        endpoint: zeroai::Client::default_endpoint(adapter_kind),
        auth: zeroai::AuthData::from_single(api_key),
        model: model_iden,
    };

    let is_stream = req.stream.unwrap_or(false);

    if is_stream {
        let stream_opts = chat_opts
            .clone()
            .with_capture_content(true)
            .with_capture_usage(true);

        match client
            .exec_chat_stream(target.clone(), chat_req, Some(&stream_opts))
            .await
        {
            Ok(stream_res) => {
                let model_name = req.model.clone();
                let sse = stream_res.stream.map(move |event| {
                    let model_name = model_name.clone();
                    match event {
                        Ok(zeroai::chat::ChatStreamEvent::Chunk(chunk)) => {
                            let data = json!({
                                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                                "object": "chat.completion.chunk",
                                "created": chrono::Utc::now().timestamp(),
                                "model": model_name,
                                "choices": [{
                                    "index": 0,
                                    "delta": {"content": chunk.content},
                                    "finish_reason": null
                                }]
                            });
                            Ok::<_, std::convert::Infallible>(
                                Event::default().data(data.to_string()),
                            )
                        }
                        Ok(zeroai::chat::ChatStreamEvent::End(end)) => {
                            let usage = end.captured_usage.as_ref().map(|u| {
                                json!({
                                    "prompt_tokens": u.prompt_tokens.unwrap_or(0),
                                    "completion_tokens": u.completion_tokens.unwrap_or(0),
                                    "total_tokens": u.total_tokens.unwrap_or(0),
                                })
                            });
                            let data = json!({
                                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                                "object": "chat.completion.chunk",
                                "created": chrono::Utc::now().timestamp(),
                                "model": model_name,
                                "choices": [{
                                    "index": 0,
                                    "delta": {},
                                    "finish_reason": "stop"
                                }],
                                "usage": usage
                            });
                            Ok(Event::default().data(data.to_string()))
                        }
                        Ok(_) => Ok(Event::default().comment("")),
                        Err(e) => {
                            let data = json!({"error": {"message": e.to_string()}});
                            Ok(Event::default().data(data.to_string()))
                        }
                    }
                });

                Sse::new(sse).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": {"message": e.to_string()}})),
            )
                .into_response(),
        }
    } else {
        match client
            .exec_chat(target, chat_req, Some(&chat_opts))
            .await
        {
            Ok(res) => {
                let text = res.first_text().map(|t| t.to_string());
                let tool_calls: Vec<serde_json::Value> = res
                    .tool_calls()
                    .into_iter()
                    .map(|tc| {
                        json!({
                            "id": tc.call_id,
                            "type": "function",
                            "function": {
                                "name": tc.fn_name,
                                "arguments": tc.fn_arguments.to_string()
                            }
                        })
                    })
                    .collect();

                let finish_reason = if !tool_calls.is_empty() {
                    "tool_calls"
                } else {
                    "stop"
                };

                let response = json!({
                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                    "object": "chat.completion",
                    "created": chrono::Utc::now().timestamp(),
                    "model": req.model,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": text,
                            "tool_calls": if tool_calls.is_empty() { serde_json::Value::Null } else { json!(tool_calls) }
                        },
                        "finish_reason": finish_reason
                    }],
                    "usage": {
                        "prompt_tokens": res.usage.prompt_tokens.unwrap_or(0),
                        "completion_tokens": res.usage.completion_tokens.unwrap_or(0),
                        "total_tokens": res.usage.total_tokens.unwrap_or(0),
                    }
                });

                Json(response).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": {"message": e.to_string()}})),
            )
                .into_response(),
        }
    }
}
