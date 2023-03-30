use actix_web::Result;
#[cfg(feature = "validator")]
use actix_web::{http::StatusCode, Error, ResponseError};
#[cfg(feature = "validator")]
use std::fmt::{self, Debug, Display};
#[cfg(feature = "validator")]
use validator::{Validate, ValidationErrors};

#[cfg(feature = "validator")]
pub(crate) fn map_errors(e: ValidationErrors) -> Error {
    Error::from(BadRequest(e))
}

#[cfg(feature = "validator")]
struct BadRequest(ValidationErrors);

#[cfg(feature = "validator")]
impl Debug for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

#[cfg(feature = "validator")]
impl Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "validator")]
impl ResponseError for BadRequest {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NotValidated;

#[cfg(feature = "validator")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Validated;

pub(crate) trait Valid<T> {
    fn valid(value: &T) -> Result<()>;
}

impl<T> Valid<T> for NotValidated {
    fn valid(_: &T) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "validator")]
impl<T: Validate> Valid<T> for Validated {
    fn valid(value: &T) -> Result<()> {
        value.validate().map_err(map_errors)
    }
}
