//! ZAI API Documentation
//! API Documentation:     <https://api.z.ai>
//! Model Names:           GLM series models
//! Pricing:               <https://api.z.ai/pricing>
//!
//! ## Dual Endpoint Support
//!
//! ZAI supports two different API endpoints via prefix routing:
//!
//! ### Regular API (Credit-based) (default for those models or with `zai/` prefix)
//! - Endpoint: `<https://api.z.ai/api/paas/v4/>`
//! - Models: `glm-4.6`, `glm-4.5`, etc.
//! - Usage: Standard API calls billed per token
//!
//! ### Coding Plan (Subscription-based only with the `zai-coding/` prefix)
//! - Endpoint: `<https://api.z.ai/api/coding/paas/v4/>`
//! - Models: `zai-coding/glm-4.6`, `zai-coding/glm-4.5`, etc.
//! - Usage: Fixed monthly subscription for coding tasks
//!
//! ## For example
//!
//! ```rust
//! use zeroai::Client;
//!
//! let client = Client::default();
//!
//! // Use regular API
//! let response = client.exec_chat("glm-4.6", chat_request, None).await?;
//! // Same, regular API with prefix
//! let response = client.exec_chat("zai/glm-4.6", chat_request, None).await?;
//! ```

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules
