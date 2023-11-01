use std::task::{Context, Poll};

use axum::{
    body::Body,
    http::{header, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use time::{format_description::well_known::Rfc2822, OffsetDateTime};
use tower::{Layer, Service};

#[derive(Debug, Clone, Default)]
pub struct CacheLayer {
    pub options: Options,
}

impl CacheLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(options: Options) -> Self {
        Self { options }
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = ResponseService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseService {
            inner,
            options: self.options,
        }
    }
}

/// Options for the caching layer.
///
/// The default is:
/// - `Cache-Control: public, no-cache, max-age=0`
/// - `Last-Modified: <current time>`
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// The core use-cases of the `Cache-Control` header.
    pub cache_control: CacheControl,
    /// Should the `Cache-Control` use `private` (true) or `public` (false).
    pub private: bool,
    /// The strategy to use for generating ETags.
    pub etag_strategy: ETagStrategy,
}

/// The core use-cases of the `Cache-Control` header.
///
/// Note: This doesn't include *all* of the possible options.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheControl {
    /// Cache-Control: `max-age=<seconds>`
    MaxAge(u32),
    /// Cache-Control: `no-cache, max-age=0`
    #[default]
    NoCache,
    /// Cache-Control: `no-store`
    NoStore,
}

/// The strategy to use for generating ETags.
///
/// This is used to determine how the ETag is generated for a response.
///
/// - `None` (default): no ETag is generated and the header is not included
/// - `Weak`: TODO
/// - `Strong`: TODO
#[derive(Debug, Clone, Copy, Default)]
pub enum ETagStrategy {
    #[default]
    None,
    Weak,
    Strong,
}

#[derive(Debug, Clone)]
pub struct ResponseService<S> {
    inner: S,
    options: Options,
}

impl<S> Service<Request<Body>> for ResponseService<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;

    // TODO: use a custom error type to avoid using `.unwrap()`
    type Error = S::Error;

    // TODO: implement my own future that is not boxed (hard to do)
    // Source: https://github.com/tower-rs/tower/blob/master/guides/building-a-middleware-from-scratch.md
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let future = self.inner.call(request);

        let cache_control = {
            let publicity = if self.options.private {
                "private"
            } else {
                "public"
            };

            let core = match self.options.cache_control {
                CacheControl::MaxAge(secs) => format!("max-age={}", secs),
                CacheControl::NoCache => "no-cache, max-age=0".to_string(),
                CacheControl::NoStore => "no-store".to_string(),
            };

            format!("{}, {}", publicity, core)
        };

        Box::pin(async move {
            let mut response: Response = future.await?;

            response.headers_mut().append(
                header::CACHE_CONTROL,
                header::HeaderValue::try_from(&cache_control).unwrap(),
            );

            let now = OffsetDateTime::now_utc().format(&Rfc2822).unwrap();

            response.headers_mut().append(
                header::LAST_MODIFIED,
                header::HeaderValue::try_from(&now).unwrap(),
            );

            // TODO: ETag header

            Ok(response)
        })
    }
}
