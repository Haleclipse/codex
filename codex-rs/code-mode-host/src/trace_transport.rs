//! OTLP trace-only WebSocket transport for forwarding code-mode host spans.

use std::net::SocketAddr;

use anyhow::Context;
use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::http::header::ORIGIN;
use axum::middleware;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::any;
use axum::routing::post;
use futures::SinkExt;
use futures::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;
use tracing::warn;

const TRACE_CHANNEL_CAPACITY: usize = 1_024;
const MAX_TRACE_WEBSOCKET_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Creates the channel shared by the loopback OTLP receiver and trace WebSocket.
pub fn trace_batch_channel() -> broadcast::Sender<Vec<u8>> {
    let (sender, _) = broadcast::channel(TRACE_CHANNEL_CAPACITY);
    sender
}

/// Binds the loopback OTLP receiver used when trace WebSocket forwarding is enabled.
pub async fn bind_otlp_trace_receiver() -> Result<(TcpListener, String)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to bind loopback OTLP trace receiver")?;
    let local_address = listener
        .local_addr()
        .context("failed to read OTLP trace receiver listen address")?;
    let endpoint = format!("http://{local_address}/v1/traces");
    Ok((listener, endpoint))
}

/// Receives OTLP/HTTP JSON trace batches from this process's local exporter.
pub async fn run_otlp_trace_receiver(
    listener: TcpListener,
    sender: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let local_address = listener
        .local_addr()
        .context("failed to read OTLP trace receiver listen address")?;
    info!("codex-code-mode-host OTLP trace receiver listening on http://{local_address}");
    let router = Router::new()
        .route("/v1/traces", post(receive_otlp_trace_batch))
        .with_state(sender);
    axum::serve(listener, router)
        .await
        .context("code-mode host OTLP trace receiver failed")
}

/// Serves a second WebSocket that streams only raw OTLP trace batches.
pub async fn run_otel_trace_listener(
    listen_url: &str,
    sender: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let bind_address = parse_listen_url(listen_url, "--otel-trace-listen", "ws://")?;
    let listener = TcpListener::bind(bind_address)
        .await
        .with_context(|| format!("failed to bind OTEL trace websocket to {bind_address}"))?;
    let local_address = listener
        .local_addr()
        .context("failed to read OTEL trace websocket listen address")?;
    info!("codex-code-mode-host OTEL trace websocket listening on ws://{local_address}");
    let router = Router::new()
        .route("/", any(trace_websocket_upgrade_handler))
        .layer(middleware::from_fn(reject_requests_with_origin_header))
        .with_state(sender);
    axum::serve(listener, router)
        .await
        .context("code-mode host OTEL trace websocket listener failed")
}

async fn reject_requests_with_origin_header(
    request: Request<Body>,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    if request.headers().contains_key(ORIGIN) {
        warn!(
            method = %request.method(),
            uri = %request.uri(),
            "rejecting code-mode host OTEL trace websocket request with Origin header"
        );
        Err(StatusCode::FORBIDDEN)
    } else {
        Ok(next.run(request).await)
    }
}

fn parse_listen_url(listen_url: &str, flag: &str, scheme: &str) -> Result<SocketAddr> {
    listen_url
        .strip_prefix(scheme)
        .ok_or_else(|| anyhow::anyhow!("unsupported {flag} URL '{listen_url}'"))?
        .parse::<SocketAddr>()
        .with_context(|| format!("invalid {flag} URL '{listen_url}'; expected '{scheme}IP:PORT'"))
}

async fn receive_otlp_trace_batch(
    State(sender): State<broadcast::Sender<Vec<u8>>>,
    body: Bytes,
) -> impl IntoResponse {
    let _ = sender.send(body.to_vec());
    ([(CONTENT_TYPE, "application/json")], "{}")
}

async fn trace_websocket_upgrade_handler(
    websocket: WebSocketUpgrade,
    State(sender): State<broadcast::Sender<Vec<u8>>>,
) -> impl IntoResponse {
    websocket
        .max_frame_size(MAX_TRACE_WEBSOCKET_FRAME_BYTES)
        .max_message_size(MAX_TRACE_WEBSOCKET_FRAME_BYTES)
        .on_upgrade(move |stream| stream_trace_batches(stream, sender))
}

async fn stream_trace_batches(stream: WebSocket, sender: broadcast::Sender<Vec<u8>>) {
    let (mut writer, mut reader) = stream.split();
    let mut receiver = sender.subscribe();
    info!("code-mode host OTEL trace websocket client connected");
    loop {
        tokio::select! {
            incoming = reader.next() => match incoming {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Text(_))) => {}
                Some(Err(error)) => {
                    warn!(%error, "code-mode host OTEL trace websocket reader failed");
                    break;
                }
            },
            outgoing = receiver.recv() => match outgoing {
                Ok(batch) => {
                    if let Err(error) = writer.send(Message::Binary(batch.into())).await {
                        warn!(%error, "code-mode host OTEL trace websocket writer failed");
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "code-mode host OTEL trace websocket client lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
}
