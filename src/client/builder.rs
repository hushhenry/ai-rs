use crate::chat::ChatOptions;
use crate::webc::WebClient;
use crate::{Client, ClientConfig, WebConfig};
use std::sync::Arc;

/// Builder for `Client`.
///
/// Create via:
/// - `ClientBuilder::default()`
/// - `Client::builder()`
#[derive(Debug, Default)]
pub struct ClientBuilder {
	web_client: Option<WebClient>,
	config: Option<ClientConfig>,
}

/// Builder methods
impl ClientBuilder {
	/// Use a custom `reqwest::Client`.
	pub fn with_reqwest(mut self, reqwest_client: reqwest::Client) -> Self {
		self.web_client = Some(WebClient::from_reqwest_client(reqwest_client));
		self
	}

	/// Set a `ClientConfig`.
	pub fn with_config(mut self, config: ClientConfig) -> Self {
		self.config = Some(config);
		self
	}

	/// Set `WebConfig` used to build the internal `reqwest::Client` (creates `ClientConfig` if absent).
	pub fn with_web_config(mut self, req_options: WebConfig) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.web_config = Some(req_options);
		self
	}
}

/// Builder ClientConfig passthrough convenient setters.
impl ClientBuilder {
	/// Set `ChatOptions` on `ClientConfig` (creates it if absent).
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.chat_options = Some(options);
		self
	}
}

impl ClientBuilder {
	/// Build a `Client`.
	pub fn build(self) -> Client {
		let config = self.config.unwrap_or_default();

		let web_client = if let Some(web_client) = self.web_client {
			web_client
		} else if let Some(req_config) = config.web_config() {
			let mut builder = reqwest::Client::builder();
			builder = req_config.apply_to_builder(builder);
			let reqwest_client = builder.build().expect("Failed to build reqwest client");
			WebClient::from_reqwest_client(reqwest_client)
		} else {
			WebClient::default()
		};

		let inner = super::ClientInner { web_client, config };
		Client { inner: Arc::new(inner) }
	}
}
