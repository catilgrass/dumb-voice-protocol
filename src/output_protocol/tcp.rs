use crate::OutputProtocol;
use crate::debug_log;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct TCPOutputProtocol {
    port: u16,
    clients: Arc<Mutex<Vec<tokio::net::tcp::OwnedWriteHalf>>>,
}

impl TCPOutputProtocol {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl OutputProtocol for TCPOutputProtocol {
    async fn init(&self) {
        let port = self.port;
        let clients = self.clients.clone();

        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[TCP] Failed to bind to {}: {}", addr, e);
                    return;
                }
            };

            debug_log!("[TCP] Listening on {}", addr);

            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        debug_log!("[TCP] Client connected: {}", peer);
                        let (_, write_half) = stream.into_split();
                        clients.lock().await.push(write_half);
                    }
                    Err(e) => {
                        debug_log!("[TCP] Accept error: {}", e);
                    }
                }
            }
        });
    }

    async fn send(self: Arc<Self>, message: &str) {
        let mut clients = self.clients.lock().await;
        let mut i = 0;
        while i < clients.len() {
            let mut write_half = clients.remove(i);
            let bytes = format!("{}\n", message);
            match write_half.write_all(bytes.as_bytes()).await {
                Ok(_) => {
                    // Re-insert at the end if successful
                    clients.insert(i, write_half);
                    i += 1;
                }
                Err(_) => {
                    // Client disconnected, drop it
                    debug_log!("[TCP] Client disconnected, removing");
                }
            }
        }
    }
}
