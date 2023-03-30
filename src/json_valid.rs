use crate::{json_config::JsonExtractInternalFut, Validated};
use actix_web::{dev, Error, FromRequest, HttpRequest, Result};
use serde::de::DeserializeOwned;
use std::{
    fmt::{self, Debug, Display},
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};
use validator::Validate;

pub struct JsonValid<T>(pub T);

impl<T> JsonValid<T> {
    /// Unwrap into inner `T` value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for JsonValid<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for JsonValid<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Debug> Debug for JsonValid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: Display> Display for JsonValid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// See [here](#extractor) for example of usage as an extractor.
impl<T: DeserializeOwned + Validate> FromRequest for JsonValid<T> {
    type Error = Error;
    type Future = JsonValidExtractFut<T>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        JsonValidExtractFut(JsonExtractInternalFut::from_req_and_payload(req, payload))
    }
}

pub struct JsonValidExtractFut<T>(JsonExtractInternalFut<T, Validated>);

impl<T: DeserializeOwned + Validate> Future for JsonValidExtractFut<T> {
    type Output = Result<JsonValid<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Future::poll(unsafe { self.map_unchecked_mut(|v| &mut v.0) }, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => Poll::Ready(r.map(JsonValid)),
        }
    }
}
