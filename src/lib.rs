//! `zeroai` library - A client library for any AI provider.
//! See [examples/c00-readme.rs](./examples/c00-readme.rs)

// region:    --- Modules

mod support;

mod client;
mod common;
mod error;
pub mod mapper;
pub mod auth;
pub mod oauth;

// -- Flatten
pub use client::*;
pub use common::*;
pub use error::{BoxError, Error, Result};

// -- Public Modules
pub mod adapter;
pub mod chat;
pub mod embed;
pub mod webc;

// endregion: --- Modules
