use crate::OutputProtocol;
use crate::debug_log;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

/// IPC output protocol.
///
/// Connects to a Unix domain socket and sends messages to it.
/// On Windows, this will fail with a clear error message since
/// Unix domain sockets are not supported.
#[derive(Debug)]
pub struct IPCOutputProtocol {
    socket_path: PathBuf,
    stream: tokio::sync::Mutex<Option<UnixStream>>,
}

impl IPCOutputProtocol {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            stream: tokio::sync::Mutex::new(None),
        }
    }
}

impl OutputProtocol for IPCOutputProtocol {
    async fn init(&self) {
        let path = self.socket_path.clone();
        let stream_lock = &self.stream;

        match UnixStream::connect(&path).await {
            Ok(stream) => {
                debug_log!("[IPC] Connected to {}", path.display());
                *stream_lock.lock().await = Some(stream);
            }
            Err(e) => {
                debug_log!("[IPC] Failed to connect to {}: {}", path.display(), e);
            }
        }
    }

    async fn send(self: Arc<Self>, message: &str) {
        let mut guard = self.stream.lock().await;
        if let Some(ref mut stream) = *guard {
            let bytes = format!("{}\n", message);
            if stream.write_all(bytes.as_bytes()).await.is_err() {
                debug_log!("[IPC] Write error, reconnecting...");
                *guard = None;
                // Try to reconnect
                if let Ok(new_stream) = UnixStream::connect(&self.socket_path).await {
                    debug_log!("[IPC] Reconnected to {}", self.socket_path.display());
                    *guard = Some(new_stream);
                }
            }
        } else {
            // Try to connect
            if let Ok(stream) = UnixStream::connect(&self.socket_path).await {
                debug_log!("[IPC] Connected to {}", self.socket_path.display());
                let bytes = format!("{}\n", message);
                let _ = stream.writable().await;
                let _ = stream.try_write(bytes.as_bytes());
                *guard = Some(stream);
            }
        }
    }
}
