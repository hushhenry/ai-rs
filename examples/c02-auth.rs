//! This example demonstrates how to use a ServiceTarget to provide custom auth
//! by building a target with an explicit API key.

use zeroai::adapter::AdapterKind;
use zeroai::chat::printer::print_chat_stream;
use zeroai::chat::{ChatMessage, ChatRequest};
use zeroai::{AuthData, Client, Endpoint, ModelIden, ServiceTarget};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("zeroai=debug"))
		.init();

	let questions = &[
		"Why is the sky blue?",
		"Why is it red sometimes?",
	];

	let api_key = std::env::var("OPENAI_API_KEY")
		.map_err(|_| "OPENAI_API_KEY not set")?;

	println!("\n>> Using custom auth for OpenAI <<");

	let client = Client::default();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		let target = ServiceTarget {
			endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
			auth: AuthData::from_single(&api_key),
			model: ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini"),
		};

		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(target, chat_req.clone(), None).await?;

		println!("\n--- Answer: (streaming)");
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
