//! This example demonstrates how to use a ServiceTarget directly to fully control
//! endpoint, auth, and model routing — e.g. pointing a model at a custom provider.

use zeroai::adapter::AdapterKind;
use zeroai::chat::{ChatMessage, ChatOptions, ChatRequest};
use zeroai::{AuthData, Client, Endpoint, ModelIden, ServiceTarget};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "meta-llama/Llama-3-8b-chat-hf";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("zeroai=debug"))
		.init();

	let questions = &[
		"Why is the sky blue?",
		"Why is it red sometimes?",
	];

	let target = ServiceTarget {
		endpoint: Endpoint::from_static("https://api.together.xyz/v1/"),
		auth: AuthData::from_env("TOGETHER_API_KEY"),
		model: ModelIden::new(AdapterKind::OpenAI, MODEL),
	};

	let client = Client::default();

	let chat_options = ChatOptions::default().with_normalize_reasoning_content(true);

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat(target.clone(), chat_req.clone(), Some(&chat_options)).await?;

		if let Some(reasoning_content) = chat_res.reasoning_content.as_deref() {
			println!("\n--- Reasoning:\n{reasoning_content}")
		}

		println!("\n--- Answer: ");
		let assistant_answer = chat_res.first_text().ok_or("Should have response")?;
		println!("{assistant_answer}");

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
