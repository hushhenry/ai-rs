//! This example demonstrates how to use Google Cloud Platform (GCP) service account
//! authentication with zeroai, constructing a ServiceTarget with a Bearer token.

use gcp_auth::{CustomServiceAccount, TokenProvider};
use zeroai::adapter::AdapterKind;
use zeroai::chat::printer::print_chat_stream;
use zeroai::chat::{ChatMessage, ChatRequest};
use zeroai::{AuthData, Client, Endpoint, Headers, ModelIden, ServiceTarget};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gemini-2.0-flash";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("zeroai=debug"))
		.init();

	let gcp_env_name: Arc<str> = "GCP_SERVICE_ACCOUNT".into();

	let gcp_json = std::env::var(&*gcp_env_name)
		.map_err(|_| format!("Environment variable {} not set", gcp_env_name))?;

	let account = CustomServiceAccount::from_json(&gcp_json)?;
	let scopes: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];
	let token = account.token(scopes).await?;

	let location = std::env::var("GCP_LOCATION").unwrap_or("us-central1".to_string());
	let project_id = account
		.project_id()
		.ok_or("GCP Auth: Service account has no project_id")?;

	let url = format!(
		"https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
		location, project_id, location, MODEL
	);

	let auth_value = format!("Bearer {}", token.as_str());
	let auth_header = Headers::from(("Authorization", auth_value));

	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::Gemini, MODEL),
		endpoint: Endpoint::from_owned(&*url),
		auth: AuthData::RequestOverride {
			headers: auth_header,
			url,
		},
	};

	let client = Client::default();

	let chat_request = ChatRequest::default().with_system("Answer in one sentence");
	let chat_request = chat_request.append_message(ChatMessage::user("Why is the sky blue?"));

	let stream = client.exec_chat_stream(target, chat_request, None).await?;

	print_chat_stream(stream, None).await?;
	Ok(())
}
