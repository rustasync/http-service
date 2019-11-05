//! Types and traits giving an interface between low-level http server implementations
//! and services that use them. The interface is based on the `std::futures` API.

#![warn(missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::mutex_atomic, clippy::module_inception)]
#![doc(test(attr(deny(rust_2018_idioms, warnings))))]
#![doc(test(attr(allow(unused_extern_crates, unused_variables))))]

use async_std::io::{self, prelude::*};
use async_std::prelude::*;
use async_std::task::{Context, Poll};

use std::fmt;
use std::pin::Pin;

/// The raw body of an http request or response.
pub struct Body {
    reader: Box<dyn Read + Unpin + Send + 'static>,
}

impl Body {
    /// Create a new empty body.
    pub fn empty() -> Self {
        Self {
            reader: Box::new(io::empty()),
        }
    }

    /// Create a new instance from a reader.
    pub fn from_reader(reader: impl Read + Unpin + Send + 'static) -> Self {
        Self {
            reader: Box::new(reader),
        }
    }
}

impl Read for Body {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body").field("reader", &"<hidden>").finish()
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Body {
        Self {
            reader: Box::new(io::Cursor::new(vec)),
        }
    }
}

impl<R: Read + Unpin + Send + 'static> From<Box<R>> for Body {
    /// Converts an `AsyncRead` into a Body.
    fn from(reader: Box<R>) -> Self {
        Self { reader }
    }
}

/// An HTTP request with a streaming body.
pub type Request = http::Request<Body>;

/// An HTTP response with a streaming body.
pub type Response = http::Response<Body>;

/// An async HTTP service
///
/// An instance represents a service as a whole. The associated `Conn` type
/// represents a particular connection, and may carry connection-specific state.
pub trait HttpService<E>: Send + Sync + 'static {
    /// The async computation for producing the response.
    ///
    /// Returning an error will result in the server immediately dropping
    /// the connection. It is usually preferable to instead return an HTTP response
    /// with an error status code.
    type ResponseFuture: Send + 'static + Future<Output = Result<Response, E>>;

    /// Begin handling a single request.
    ///
    /// The handler is given shared access to the service itself, and mutable access
    /// to the state for the connection where the request is taking place.
    fn respond(&self, req: Request) -> Self::ResponseFuture;
}

impl<F, R, E> HttpService<E> for F
where
    F: Send + Sync + 'static + Fn(Request) -> R,
    R: Send + 'static + Future<Output = Result<Response, E>>,
{
    type ResponseFuture = R;
    fn respond(&self, req: Request) -> Self::ResponseFuture {
        (self)(req)
    }
}
