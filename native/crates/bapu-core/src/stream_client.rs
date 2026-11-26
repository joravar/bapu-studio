//! Real-time WebSocket and Server-Sent Events (SSE) client for Stream Studio.

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    WebSocket,
    Sse,
}

#[derive(Debug, Clone)]
pub struct StreamFrame {
    pub direction: FrameDirection,
    pub payload: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDirection {
    Incoming,
    Outgoing,
    System,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Connected,
    Frame(StreamFrame),
    Error(String),
    Closed,
}

pub struct StreamHandle {
    pub outbound_tx: Option<mpsc::UnboundedSender<String>>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl StreamHandle {
    pub fn send(&self, text: String) -> Result<(), String> {
        match &self.outbound_tx {
            Some(tx) => tx.send(text).map_err(|_| "Stream is closed".to_string()),
            None => Err("Sending is only supported on WebSocket streams".into()),
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.outbound_tx = None;
    }
}

fn now_hms() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

pub fn connect_websocket(
    runtime: &tokio::runtime::Handle,
    url: String,
    events_tx: std::sync::mpsc::Sender<StreamEvent>,
) -> StreamHandle {
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    runtime.spawn(async move {
        let ws_result = tokio_tungstenite::connect_async(&url).await;
        let (ws_stream, _) = match ws_result {
            Ok(pair) => pair,
            Err(e) => {
                let _ = events_tx.send(StreamEvent::Error(format!("WebSocket connect failed: {e}")));
                let _ = events_tx.send(StreamEvent::Closed);
                return;
            }
        };

        let _ = events_tx.send(StreamEvent::Connected);
        let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
            direction: FrameDirection::System,
            payload: format!("Connected to {url}"),
            timestamp: now_hms(),
        }));

        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    let _ = write.close().await;
                    break;
                }
                Some(msg_text) = out_rx.recv() => {
                    if let Err(e) = write.send(Message::Text(msg_text.clone())).await {
                        let _ = events_tx.send(StreamEvent::Error(format!("Send failed: {e}")));
                        break;
                    }
                    let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
                        direction: FrameDirection::Outgoing,
                        payload: msg_text,
                        timestamp: now_hms(),
                    }));
                }
                incoming = read.next() => {
                    match incoming {
                        Some(Ok(Message::Text(txt))) => {
                            let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
                                direction: FrameDirection::Incoming,
                                payload: txt,
                                timestamp: now_hms(),
                            }));
                        }
                        Some(Ok(Message::Binary(bin))) => {
                            let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
                                direction: FrameDirection::Incoming,
                                payload: format!("<binary {} bytes>", bin.len()),
                                timestamp: now_hms(),
                            }));
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(e)) => {
                            let _ = events_tx.send(StreamEvent::Error(format!("Read error: {e}")));
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
            direction: FrameDirection::System,
            payload: "Connection closed".into(),
            timestamp: now_hms(),
        }));
        let _ = events_tx.send(StreamEvent::Closed);
    });

    StreamHandle {
        outbound_tx: Some(out_tx),
        shutdown_tx: Some(shutdown_tx),
    }
}

pub fn connect_sse(
    runtime: &tokio::runtime::Handle,
    client: reqwest::Client,
    url: String,
    events_tx: std::sync::mpsc::Sender<StreamEvent>,
) -> StreamHandle {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    runtime.spawn(async move {
        let resp = match client.get(&url).header("Accept", "text/event-stream").send().await {
            Ok(r) => r,
            Err(e) => {
                let _ = events_tx.send(StreamEvent::Error(format!("SSE connect failed: {e}")));
                let _ = events_tx.send(StreamEvent::Closed);
                return;
            }
        };

        if !resp.status().is_success() {
            let _ = events_tx.send(StreamEvent::Error(format!("SSE HTTP {}", resp.status())));
            let _ = events_tx.send(StreamEvent::Closed);
            return;
        }

        let _ = events_tx.send(StreamEvent::Connected);
        let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
            direction: FrameDirection::System,
            payload: format!("Subscribed to SSE stream {url}"),
            timestamp: now_hms(),
        }));

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            buf.push_str(&String::from_utf8_lossy(&bytes));
                            while let Some(pos) = buf.find("\n\n") {
                                let event_block = buf[..pos].trim().to_string();
                                buf = buf[pos + 2..].to_string();
                                if !event_block.is_empty() {
                                    let _ = events_tx.send(StreamEvent::Frame(StreamFrame {
                                        direction: FrameDirection::Incoming,
                                        payload: event_block,
                                        timestamp: now_hms(),
                                    }));
                                }
                            }
                        }
                        Some(Err(e)) => {
                            let _ = events_tx.send(StreamEvent::Error(format!("SSE read error: {e}")));
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        let _ = events_tx.send(StreamEvent::Closed);
    });

    StreamHandle {
        outbound_tx: None,
        shutdown_tx: Some(shutdown_tx),
    }
}
