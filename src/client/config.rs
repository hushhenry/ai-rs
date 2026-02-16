use crate::adapter::{AdapterDispatcher, AdapterKind};
use crate::chat::ChatOptions;
use crate::client::{ModelSpec, ServiceTarget};
use crate::embed::EmbedOptions;
use crate::mapper::ModelMapper as PrefixMapper;
use crate::{ModelIden, Result, WebConfig};

/// Configuration for building and customizing a `Client`.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(super) web_config: Option<WebConfig>,
	pub(super) chat_options: Option<ChatOptions>,
	pub(super) embed_options: Option<EmbedOptions>,
}

/// Chainable setters related to the ClientConfig.
impl ClientConfig {
	/// Sets default ChatOptions for chat requests.
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}

	/// Sets default EmbedOptions for embed requests.
	pub fn with_embed_options(mut self, options: EmbedOptions) -> Self {
		self.embed_options = Some(options);
		self
	}

	/// Sets the HTTP client configuration (reqwest).
	pub fn with_web_config(mut self, web_config: WebConfig) -> Self {
		self.web_config = Some(web_config);
		self
	}

	/// Returns the WebConfig, if set.
	pub fn web_config(&self) -> Option<&WebConfig> {
		self.web_config.as_ref()
	}
}

/// Getters for the fields of ClientConfig (as references).
impl ClientConfig {
	/// Returns the default ChatOptions, if set.
	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}

	/// Returns the default EmbedOptions, if set.
	pub fn embed_options(&self) -> Option<&EmbedOptions> {
		self.embed_options.as_ref()
	}
}

/// Resolvers
impl ClientConfig {
	/// Resolves a ServiceTarget for the given model.
	///
	/// Gets the adapter's default auth and endpoint for the model.
	pub async fn resolve_service_target(&self, model: ModelIden) -> Result<ServiceTarget> {
		let auth = AdapterDispatcher::default_auth(model.adapter_kind);
		let endpoint = AdapterDispatcher::default_endpoint(model.adapter_kind);

		let service_target = ServiceTarget {
			model,
			auth,
			endpoint,
		};

		Ok(service_target)
	}

	/// Resolves a [`ModelSpec`] to a [`ServiceTarget`].
	///
	/// If the name contains a `"provider/model"` prefix that maps to a known adapter,
	/// routes directly to that adapter with the short model name.
	/// Otherwise infers adapter from the name via `AdapterKind::from_model`.
	///
	/// Returns `(ServiceTarget, Option<String>)` where the second element is the
	/// provider prefix (if present) for response backfill.
	pub async fn resolve_model_spec(&self, spec: ModelSpec) -> Result<(ServiceTarget, Option<String>)> {
		match spec {
			ModelSpec::Name(name) => {
				let mapper = PrefixMapper::new();
				if let Some((provider, short)) = mapper.split_id(&name) {
					if let Some(adapter_kind) = AdapterKind::from_lower_str(provider) {
						let model = ModelIden::new(adapter_kind, short);
						let target = self.resolve_service_target(model).await?;
						return Ok((target, Some(provider.to_string())));
					}
				}
				let adapter_kind = AdapterKind::from_model(&name)?;
				let model = ModelIden::new(adapter_kind, name);
				let target = self.resolve_service_target(model).await?;
				Ok((target, None))
			}
			ModelSpec::Iden(model) => {
				let target = self.resolve_service_target(model).await?;
				Ok((target, None))
			}
			ModelSpec::Target(target) => Ok((target, None)),
		}
	}
}
