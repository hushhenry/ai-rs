use std::sync::Arc;

/// A construct to store the endpoint of a service.
/// Efficiently clonable. For now supports only `base_url`.
#[derive(Debug, Clone)]
pub struct Endpoint {
	inner: EndpointInner,
}

#[derive(Debug, Clone)]
enum EndpointInner {
	Static(&'static str),
	Owned(Arc<str>),
}

/// Constructors
impl Endpoint {
	pub fn from_static(url: &'static str) -> Self {
		Endpoint {
			inner: EndpointInner::Static(url),
		}
	}

	pub fn from_owned(url: impl Into<Arc<str>>) -> Self {
		Endpoint {
			inner: EndpointInner::Owned(url.into()),
		}
	}
}

/// Getters
impl Endpoint {
	pub fn base_url(&self) -> &str {
		match &self.inner {
			EndpointInner::Static(url) => url,
			EndpointInner::Owned(url) => url,
		}
	}
}
