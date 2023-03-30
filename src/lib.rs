//! # actix-web-utils
//!
//! A list of functions and types for improving actix-web productivity
//!
//! ## Example tracing Json
//!
//! tracing json to error when they are deserialized.
//!
//! ```
//! use actix_web::{Result, post, HttpResponse};
//! use actix_web_utils::Json;
//!
//! #[post("/")]
//! async fn login(data: Json<TodoData>) -> Result<HttpResponse> {
//!     // do something with data...
//!     Ok(HttpResponse::Ok().finish())
//! }
//!
//! #[derive(serde::Deserialize)]
//! struct TodoData {
//!     title: String,
//! }
//!
//! ```
//!
//! # Example with Json Validation
//!
//! This is still tracing but also implement validation. This requires feature `validator`
//!
//! ```
//! use actix_web::{Result, post, HttpResponse};
//! use actix_web_utils::JsonValid;
//!
//! #[post("/")]
//! async fn login(data: JsonValid<TodoData>) -> Result<HttpResponse> {
//!     // do something with data...
//!     Ok(HttpResponse::Ok().finish())
//! }
//!
//! #[derive(serde::Deserialize, validator::Validate)]
//! struct TodoData {
//!     #[validate(length(min = 1, max = 10))]
//!     title: String,
//! }
//!
//! ```
//!

mod json;
mod json_config;
#[cfg(feature = "validator")]
mod json_valid;
mod validation;

pub use json::Json;
pub use json_config::JsonConfig;
use json_config::JsonExtractInternalFut;
#[cfg(feature = "validator")]
pub use json_valid::JsonValid;
use validation::*;
