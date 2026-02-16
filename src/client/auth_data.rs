use crate::Headers;
use derive_more::From;
use std::collections::HashMap;

// region:    --- AuthDataError

/// Error type for AuthData resolution.
pub type AuthDataResult<T> = core::result::Result<T, AuthDataError>;

#[derive(Debug, From)]
pub enum AuthDataError {
	ApiKeyEnvNotFound { env_name: String },
	AuthDataNotSingleValue,
	#[from]
	Custom(String),
}

impl core::fmt::Display for AuthDataError {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for AuthDataError {}

// endregion: --- AuthDataError

// region:    --- AuthData

/// `AuthData` specifies either how or the key itself for authentication.
#[derive(Clone)]
pub enum AuthData {
	/// Specify the environment name to get the key value from.
	FromEnv(String),

	/// The key value itself.
	Key(String),

	/// Override headers and request url for unorthodox authentication schemes.
	RequestOverride { url: String, headers: Headers },

	/// Multiple key/values (adapter-specific, not yet used).
	MultiKeys(HashMap<String, String>),

	None,
}

/// Constructors
impl AuthData {
	pub fn from_env(env_name: impl Into<String>) -> Self {
		AuthData::FromEnv(env_name.into())
	}

	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Key(value.into())
	}

	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::MultiKeys(data)
	}
}

/// Getters
impl AuthData {
	pub fn single_key_value(&self) -> AuthDataResult<String> {
		match self {
			AuthData::RequestOverride { .. } => Ok(String::new()),
			AuthData::FromEnv(env_name) => {
				let value = std::env::var(env_name).map_err(|_| AuthDataError::ApiKeyEnvNotFound {
					env_name: env_name.to_string(),
				})?;
				Ok(value)
			}
			AuthData::Key(value) => Ok(value.to_string()),
			_ => Err(AuthDataError::AuthDataNotSingleValue),
		}
	}
}

impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AuthData::FromEnv(_) => write!(f, "AuthData::FromEnv(REDACTED)"),
			AuthData::Key(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::MultiKeys(_) => write!(f, "AuthData::Multi(REDACTED)"),
			AuthData::RequestOverride { .. } => {
				write!(f, "AuthData::RequestOverride {{ url: REDACTED, headers: REDACTED }}")
			}
			AuthData::None => write!(f, "None"),
		}
	}
}

// endregion: --- AuthData
