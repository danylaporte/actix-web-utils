use crate::Valid;
use actix_web::{
    dev::{self, Payload},
    error::JsonPayloadError,
    http::header::CONTENT_LENGTH,
    web::{self, BytesMut},
    Error, HttpMessage, HttpRequest,
};
use futures::{Future, Stream};
use mime::Mime;
use serde::de::DeserializeOwned;
use std::{
    borrow::Cow,
    marker::PhantomData,
    mem::take,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tracing::{error, trace};

#[derive(Clone)]
pub struct JsonConfig {
    pub(super) content_type: Option<Arc<dyn Fn(Mime) -> bool + Send + Sync>>,
    pub(super) content_type_required: bool,
    pub(super) limit: usize,
}

impl JsonConfig {
    /// Set maximum accepted payload size. By default this limit is 2MB.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set predicate for allowed content types.
    pub fn content_type<F>(mut self, predicate: F) -> Self
    where
        F: Fn(Mime) -> bool + Send + Sync + 'static,
    {
        self.content_type = Some(Arc::new(predicate));
        self
    }

    /// Sets whether or not the request must have a `Content-Type` header to be parsed.
    pub fn content_type_required(mut self, content_type_required: bool) -> Self {
        self.content_type_required = content_type_required;
        self
    }

    /// Extract payload config from app data. Check both `T` and `Data<T>`, in that order, and fall
    /// back to the default payload config.
    pub(crate) fn from_req(req: &HttpRequest) -> &Self {
        req.app_data::<Self>()
            .or_else(|| req.app_data::<web::Data<Self>>().map(|d| d.as_ref()))
            .unwrap_or(&DEFAULT_CONFIG)
    }
}

const DEFAULT_LIMIT: usize = 2_097_152; // 2 mb

/// Allow shared refs used as default.
const DEFAULT_CONFIG: JsonConfig = JsonConfig {
    limit: DEFAULT_LIMIT,
    content_type: None,
    content_type_required: true,
};

impl Default for JsonConfig {
    fn default() -> Self {
        DEFAULT_CONFIG
    }
}

pub(super) enum JsonExtractInternalFut<T, V> {
    Error(Option<JsonPayloadError>),
    Body {
        limit: usize,
        /// Length as reported by `Content-Length` header, if present.
        length: Option<usize>,
        #[cfg(feature = "__compress")]
        payload: Decompress<Payload>,
        #[cfg(not(feature = "__compress"))]
        payload: Payload,
        buf: BytesMut,
        _res: PhantomData<T>,
        _v: PhantomData<V>,
    },
}

impl<T, V> Unpin for JsonExtractInternalFut<T, V> {}

impl<T: DeserializeOwned, V: Valid<T>> JsonExtractInternalFut<T, V> {
    pub fn from_req_and_payload(req: &HttpRequest, payload: &mut dev::Payload) -> Self {
        let config = JsonConfig::from_req(req);

        let limit = config.limit;
        let ctype_required = config.content_type_required;
        let ctype_fn = config.content_type.as_deref();

        Self::new(req, payload, ctype_fn, ctype_required).limit(limit)
    }

    /// Create a new future to decode a JSON request payload.
    #[allow(clippy::borrow_interior_mutable_const)]
    fn new(
        req: &HttpRequest,
        payload: &mut Payload,
        ctype_fn: Option<&(dyn Fn(mime::Mime) -> bool + Send + Sync)>,
        ctype_required: bool,
    ) -> Self {
        // check content-type
        let can_parse_json = if let Ok(Some(mime)) = req.mime_type() {
            mime.subtype() == mime::JSON
                || mime.suffix() == Some(mime::JSON)
                || ctype_fn.map_or(false, |predicate| predicate(mime))
        } else {
            // if `ctype_required` is false, assume payload is
            // json even when content-type header is missing
            !ctype_required
        };

        if !can_parse_json {
            return Self::Error(Some(JsonPayloadError::ContentType));
        }

        let length = req
            .headers()
            .get(&CONTENT_LENGTH)
            .and_then(|l| l.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        // Notice the content-length is not checked against limit of json config here.
        // As the internal usage always call JsonBody::limit after JsonBody::new.
        // And limit check to return an error variant of JsonBody happens there.

        let payload = {
            cfg_if::cfg_if! {
                if #[cfg(feature = "__compress")] {
                    Decompress::from_headers(payload.take(), req.headers())
                } else {
                    payload.take()
                }
            }
        };

        Self::Body {
            _res: PhantomData,
            _v: PhantomData,
            buf: BytesMut::with_capacity(8192),
            length,
            limit: DEFAULT_LIMIT,
            payload,
        }
    }

    /// Set maximum accepted payload size. The default limit is 2MB.
    pub fn limit(self, limit: usize) -> Self {
        match self {
            Self::Body {
                buf,
                length,
                payload,
                ..
            } => {
                if let Some(len) = length {
                    if len > limit {
                        return Self::Error(Some(JsonPayloadError::OverflowKnownLength {
                            length: len,
                            limit,
                        }));
                    }
                }

                Self::Body {
                    _res: PhantomData,
                    _v: PhantomData,
                    buf,
                    length,
                    limit,
                    payload,
                }
            }
            Self::Error(e) => Self::Error(e),
        }
    }

    fn poll_bytes(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<BytesMut, Error>> {
        let this = self.get_mut();

        match this {
            Self::Body {
                buf,
                limit,
                payload,
                ..
            } => loop {
                let res = ready!(Pin::new(&mut *payload).poll_next(cx));

                match res {
                    Some(chunk) => {
                        let chunk = chunk?;
                        let buf_len = buf.len() + chunk.len();

                        if buf_len > *limit {
                            trace_error(buf);

                            return Poll::Ready(Err(
                                JsonPayloadError::Overflow { limit: *limit }.into()
                            ));
                        } else {
                            buf.extend_from_slice(&chunk);
                        }
                    }
                    None => return Poll::Ready(Ok(take(buf))),
                }
            },
            Self::Error(e) => Poll::Ready(Err(e.take().unwrap().into())),
        }
    }
}

impl<T: DeserializeOwned, V: Valid<T>> Future for JsonExtractInternalFut<T, V> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_bytes(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(bytes)) => match serde_json::from_slice::<T>(&bytes) {
                Ok(v) => {
                    trace_ok(&bytes);
                    V::valid(&v)?;
                    Poll::Ready(Ok(v))
                }
                Err(e) => {
                    trace_error(&bytes);
                    Poll::Ready(Err(JsonPayloadError::Deserialize(e).into()))
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }
}

fn text_repr(mut bytes: &[u8]) -> Cow<str> {
    const KB: usize = 1024;
    const _30KB: usize = 30 * KB;

    if bytes.len() > _30KB {
        bytes = &bytes[0.._30KB];
    }

    String::from_utf8_lossy(bytes)
}

fn trace_error(bytes: &[u8]) {
    error!(text = %text_repr(bytes), "json");
}

fn trace_ok(bytes: &[u8]) {
    trace!(text = %text_repr(bytes), "json");
}
