use std::task::{Context, Poll};

use axum::{
    body::Body,
    http::{header, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

#[derive(Debug, Clone, Default)]
pub struct CacheLayer {
    pub options: Options,
}

impl CacheLayer {
    pub fn new(options: Options) -> Self {
        Self { options }
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            options: self.options,
        }
    }
}

/// Options for the caching layer.
///
/// The default is:
/// - `Cache-Control: public, no-cache, max-age=0`
///
/// **Note:** the default doesn't use either `Last-Modified` or `ETag` headers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// The core use-cases of the `Cache-Control` header.
    pub cache_control: CacheControl,
    /// Should the `Cache-Control` use `private` or `public` (default: false).
    pub private: bool,
    /// Should the `Cache-Control` use `immutable` (default: false).
    pub immutable: bool,
}

/// The core use-cases of the `Cache-Control` header.
///
/// Note: This doesn't include *all* of the possible options.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheControl {
    /// Cache-Control: `no-cache, max-age=0`
    #[default]
    NoCache,
    /// Cache-Control: `no-store`
    NoStore,
    /// Cache-Control: `max-age=<seconds>`
    MaxAge(u32),
    /// Cache-Control: `max-age=<seconds>, must-revalidate`
    MustRevalidate(u32),
}

#[derive(Debug, Clone)]
pub struct CacheService<S> {
    inner: S,
    options: Options,
}

impl<S> Service<Request<Body>> for CacheService<S>
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

    #[instrument(skip_all)]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let future = self.inner.call(request);

        let cache_control = {
            let publicity = if self.options.private {
                "private, "
            } else {
                "public, "
            };

            let core = match self.options.cache_control {
                CacheControl::NoCache => "no-cache, max-age=0".to_string(),
                CacheControl::NoStore => "no-store".to_string(),
                CacheControl::MaxAge(secs) => format!("max-age={}", secs),
                CacheControl::MustRevalidate(secs) => {
                    format!("max-age={}, must-revalidate", secs)
                }
            };

            let immutable = if self.options.immutable {
                ", immutable"
            } else {
                ""
            };

            format!("{}{}{}", publicity, core, immutable)
        };
        trace!("cache-control header created: {}", cache_control);

        Box::pin(async move {
            let mut response: Response = future.await?;

            if let Some(cc) = response.headers().get(header::CACHE_CONTROL) {
                debug!("response already contains the cache-control header: {cc:?}");
                return Ok(response);
            }

            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::try_from(&cache_control).unwrap(),
            );

            Ok(response)
        })
    }
}
