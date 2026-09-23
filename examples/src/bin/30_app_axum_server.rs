//! Example 30: REST API with Axum.
//! Run with: `cargo run --bin 30_app_axum_server`
//!
//! This example demonstrates wrapping `AdsRuntime` in an HTTP server using
//! `axum`. It shows how the high-level `read_value`, `write_value`,
//! `get_symbol_info`, `rpc`, and `subscribe_value` methods compose into a
//! small REST API with almost no glue code.
//!
//! ## PREREQUISITE
//!
//! Open `twincat/TcAdsExamples.sln` in TwinCAT XAE,
//! activate the configuration on your local machine, and put the PLC into RUN mode.
//!
//! ## Usage
//!
//!Read a symbol:
//!
//!```console
//! curl http://127.0.0.1:9090/symbols/MAIN/nCount
//!```
//!
//! Write a symbol:
//!
//! ```console
//! curl -X PUT http://127.0.0.1:9090/symbols/MAIN/nCount -H 'content-type: application/json' -d '42'
//! ```
//!
//! Symbol metadata:
//!
//! ```console
//! curl http://127.0.0.1:9090/symbol-info/MAIN/fbCurrentRecipe
//! ```
//!
//! Call an RPC method (single input, single output)
//!
//! ```console
//! curl -X POST http://127.0.0.1:9090/methods/MAIN/fbMath/SumValues -H 'content-type: application/json' -d '[50, 25]'
//! ```
//!
//! Subscribe to value changes (Server-Sent Events)
//!
//! ```console
//! curl -N http://127.0.0.1:9090/subscriptions/MAIN/nCount
//! ```

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use serde_json::Value as JsonValue;
use std::convert::Infallible;
use std::time::Duration;
use tcads::client::Error as ClientError;
use tcads::client::devices::tokio::AdsRuntime;
use tcads::core::{AdsReturnCode, AdsSymbolInfo, AdsTransMode, AmsAddr};
use tcads::serde::{Error as SerdeError, Value as PlcValue};
use tokio::net::TcpListener;
use tokio_stream::Stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = AdsRuntime::connect(AmsAddr::from_local(851), Duration::from_secs(5)).await?;

    let app = Router::new()
        .route(
            "/",
            get(|| async { "Welcome to the TwinCAT PLC REST API!" }),
        )
        .route("/symbols/{*path}", get(read_symbol).put(write_symbol))
        .route("/symbol-info/{*path}", get(symbol_info))
        .route("/methods/{*path}", axum::routing::post(invoke_method))
        .route("/subscriptions/{*path}", get(subscribe_symbol))
        .with_state(device);

    let listener = TcpListener::bind("127.0.0.1:9090").await?;
    println!("Listening on http://{}", listener.local_addr()?);
    Ok(axum::serve(listener, app).await?)
}

async fn read_symbol(
    Path(path): Path<String>,
    State(device): State<AdsRuntime>,
) -> Result<Json<JsonValue>, AppError> {
    let value: PlcValue = device.read_value(path.replace('/', ".")).await?;
    Ok(Json(serde_json::to_value(&value)?))
}

async fn write_symbol(
    Path(path): Path<String>,
    State(device): State<AdsRuntime>,
    Json(payload): Json<JsonValue>,
) -> Result<StatusCode, AppError> {
    device.write_value(path.replace('/', "."), payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn symbol_info(
    Path(path): Path<String>,
    State(device): State<AdsRuntime>,
) -> Result<Json<AdsSymbolInfo>, AppError> {
    let info = device.get_symbol_info(path.replace('/', ".")).await?;
    Ok(Json(info))
}

async fn invoke_method(
    Path(path): Path<String>,
    State(device): State<AdsRuntime>,
    Json(inputs): Json<JsonValue>,
) -> Result<Json<JsonValue>, AppError> {
    let (fb, method) = path
        .rsplit_once('/')
        .ok_or_else(|| AppError::new(StatusCode::BAD_REQUEST, "expected '<fb>/<method>'"))?;

    let result: Result<PlcValue, _> = device.rpc(fb.replace('/', "."), method, &inputs).await;
    match result {
        Ok(value) => Ok(Json(serde_json::to_value(&value)?)),
        Err(ClientError::Serde(SerdeError::InvalidRpcShape { .. })) => {
            device
                .rpc::<()>(fb.replace('/', "."), method, &inputs)
                .await?;
            Ok(Json(JsonValue::Null))
        }
        Err(e) => Err(e.into()),
    }
}

async fn subscribe_symbol(
    Path(path): Path<String>,
    State(device): State<AdsRuntime>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let symbol_path = path.replace('/', ".");
    let (mut rx, _handle) = device
        .subscribe_value::<PlcValue>(
            &symbol_path,
            AdsTransMode::ServerOnChange,
            Duration::ZERO,
            Duration::ZERO,
        )
        .await?;

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(value) => match Event::default().json_data(&value) {
                    Ok(event) => yield Ok(event),
                    Err(_) => break,
                },
                Err(e) => {
                    yield Ok(Event::default().event("error").data(e.to_string()));
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

impl From<ClientError> for AppError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::Timeout => Self::new(StatusCode::GATEWAY_TIMEOUT, "PLC timed out"),
            ClientError::Disconnected => {
                Self::new(StatusCode::BAD_GATEWAY, "AMS router unreachable")
            }
            ClientError::HandleInvalidated(path) => Self::new(
                StatusCode::CONFLICT,
                format!("'{path}' invalidated by a symbol version change; retry"),
            ),
            ClientError::AdsReturnCode(code) => match code {
                AdsReturnCode::AdsErrDeviceSymbolNotFound => {
                    Self::new(StatusCode::NOT_FOUND, "symbol not found")
                }
                other => Self::new(StatusCode::BAD_GATEWAY, format!("PLC returned {other:?}")),
            },
            ClientError::Serde(e) => e.into(),
            other => Self::new(StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        }
    }
}

impl From<SerdeError> for AppError {
    fn from(err: SerdeError) -> Self {
        match err {
            SerdeError::TypeNotFound(name) => {
                Self::new(StatusCode::NOT_FOUND, format!("type '{name}' not found"))
            }
            SerdeError::TypeMismatch { expected } => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("expected {expected}"),
            ),
            SerdeError::LossyConversion { from, to } => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("cannot losslessly convert {from} to {to}"),
            ),
            SerdeError::ShapeMismatch { expected, got } => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("expected {expected} fields, got {got}"),
            ),
            other => Self::new(StatusCode::UNPROCESSABLE_ENTITY, other.to_string()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(StatusCode::BAD_REQUEST, format!("invalid JSON: {err}"))
    }
}
