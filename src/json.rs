use crate::{validation::NotValidated, JsonExtractInternalFut};
use actix_web::{
    body::EitherBody,
    dev::{self},
    error::JsonPayloadError,
    Error, FromRequest, HttpRequest, HttpResponse, Responder, Result,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::{self, Debug, Display},
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Unwrap into inner `T` value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Debug> Debug for Json<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: Display> Display for Json<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T: Serialize> Serialize for Json<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// Creates response with OK status code, correct content type header, and serialized JSON payload.
///
/// If serialization failed
impl<T: Serialize> Responder for Json<T> {
    type Body = EitherBody<String>;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string(&self.0) {
            Ok(body) => match HttpResponse::Ok()
                .content_type(mime::APPLICATION_JSON)
                .message_body(body)
            {
                Ok(res) => res.map_into_left_body(),
                Err(err) => HttpResponse::from_error(err).map_into_right_body(),
            },

            Err(err) => {
                HttpResponse::from_error(JsonPayloadError::Serialize(err)).map_into_right_body()
            }
        }
    }
}

/// See [here](#extractor) for example of usage as an extractor.
impl<T: DeserializeOwned> FromRequest for Json<T> {
    type Error = Error;
    type Future = JsonExtractFut<T>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        JsonExtractFut(JsonExtractInternalFut::from_req_and_payload(req, payload))
    }
}

pub struct JsonExtractFut<T>(JsonExtractInternalFut<T, NotValidated>);

impl<T: for<'de> DeserializeOwned> Future for JsonExtractFut<T> {
    type Output = Result<Json<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Future::poll(unsafe { self.map_unchecked_mut(|v| &mut v.0) }, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(r) => Poll::Ready(r.map(Json)),
        }
    }
}
