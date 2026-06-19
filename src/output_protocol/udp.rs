use crate::debug_log;
use crate::OutputProtocol;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// UDP output protocol.
///
/// Binds to `0.0.0.0:{port}`. The first incoming datagram determines the
/// target address; all subsequent `send` calls deliver messages to that peer.
#[derive(Debug)]
pub struct UDPOutputProtocol {
    inner: Arc<UDPInner>,
}

#[derive(Debug)]
struct UDPInner {
    socket: Mutex<Option<Arc<UdpSocket>>>,
    target: Mutex<Option<SocketAddr>>,
    target_learned: tokio::sync::watch::Sender<bool>,
}

impl UDPOutputProtocol {
    pub fn new(port: u16) -> Self {
        let (tx, _rx) = tokio::sync::watch::channel(false);
        let inner = Arc::new(UDPInner {
            socket: Mutex::new(None),
            target: Mutex::new(None),
            target_learned: tx,
        });

        let inner_clone = inner.clone();
        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{}", port);
            let socket = match UdpSocket::bind(&addr).await {
                Ok(s) => {
                    debug_log!("[UDP] Listening on {}", addr);
                    Arc::new(s)
                }
                Err(e) => {
                    debug_log!("[UDP] Failed to bind: {}", e);
                    return;
                }
            };

            // Store the socket
            *inner_clone.socket.lock().await = Some(socket.clone());

            // Learn target from first incoming packet
            let mut buf = [0u8; 4096];
            let (len, src) = match socket.recv_from(&mut buf).await {
                Ok(r) => r,
                Err(e) => {
                    debug_log!("[UDP] Recv error: {}", e);
                    return;
                }
            };
            debug_log!("[UDP] Learned target address: {} (got {} bytes)", src, len);
            *inner_clone.target.lock().await = Some(src);
            let _ = inner_clone.target_learned.send(true);
        });

        Self { inner }
    }
}

impl OutputProtocol for UDPOutputProtocol {
    async fn init(&self) {
        // Everything is set up in new() — nothing more to do.
    }

    async fn send(self: Arc<Self>, message: &str) {
        let socket_opt = self.inner.socket.lock().await;
        if let Some(ref socket) = *socket_opt {
            let target_opt = self.inner.target.lock().await;
            if let Some(target) = *target_opt {
                let bytes = format!("{}\n", message);
                let _ = socket.send_to(bytes.as_bytes(), target).await;
            }
        }
    }
}
