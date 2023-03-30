# actix-web-utils

A list of functions and types for improving actix-web productivity

## Example tracing Json

Tracing json to error when they are deserialized.

```rust
use actix_web::{Result, post, HttpResponse};
use actix_web_utils::Json;

#[post("/")]
async fn login(data: Json<TodoData>) -> Result<HttpResponse> {
    // do something with data...
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct TodoData {
    title: String,
}

```

## Example with Json Validation

This is still tracing but also implement validation. This requires feature `validator`

```rust
use actix_web::{Result, post, HttpResponse};
use actix_web_utils::JsonValid;

#[post("/")]
async fn login(data: JsonValid<TodoData>) -> Result<HttpResponse> {
    // do something with data...
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize, validator::Validate)]
struct TodoData {
    #[validate(length(min = 1, max = 10))]
    title: String,
}

```

## License

The MIT license.
