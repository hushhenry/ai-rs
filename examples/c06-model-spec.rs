//! This example shows how to use ModelSpec to control model resolution at different levels:
//! - Name-based (full inference)
//! - Iden-based (explicit adapter)
//! - Target-based (full ServiceTarget, bypass all resolution)

use zeroai::adapter::AdapterKind;
use zeroai::chat::{ChatMessage, ChatRequest};
use zeroai::{AuthData, Client, Endpoint, ModelIden, ModelSpec, ServiceTarget};
use tracing_subscriber::EnvFilter;

pub enum AppModel {
	Fast,
	Pro,
	Local,
	Custom(String),
}

impl From<&AppModel> for ModelSpec {
	fn from(model: &AppModel) -> Self {
		match model {
			AppModel::Fast => ModelSpec::from_static_name("gemini-3-flash-preview"),
			AppModel::Pro => ModelSpec::from_iden((AdapterKind::Anthropic, "claude-opus-4-5")),
			AppModel::Local => ModelSpec::Target(ServiceTarget {
				model: ModelIden::from_static(AdapterKind::Ollama, "gemma3:1b"),
				endpoint: Endpoint::from_static("http://localhost:11434"),
				auth: AuthData::Key("".to_string()),
			}),
			AppModel::Custom(name) => ModelSpec::from_name(name),
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("zeroai=debug"))
		.init();

	let model_spec = AppModel::Fast;

	let question = "Why is the sky red? (be concise)";

	let client = Client::default();

	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat(&model_spec, chat_req.clone(), None).await?;

	let model_iden = chat_res.model_iden;
	let res_txt = chat_res.content.into_joined_texts().ok_or("Should have some response")?;

	println!("\n--- Answer: ({model_iden})\n{res_txt}");

	Ok(())
}
