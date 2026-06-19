use crate::debug_log;
use crate::OutputProtocol;

use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// UDP broadcast output protocol.
///
/// Broadcasts messages to the subnet defined by `port` and `subnet_mask`.
/// The broadcast address is computed as `device_ip | (~subnet_mask)`.
#[derive(Debug)]
pub struct UDPBroadcastOutputProtocol {
    inner: Arc<UDPBroadcastInner>,
}

#[derive(Debug)]
struct UDPBroadcastInner {
    socket: Mutex<Option<Arc<UdpSocket>>>,
    broadcast_addr: Mutex<Option<String>>,
}

impl UDPBroadcastOutputProtocol {
    pub fn new(port: u16, subnet_mask: &str) -> Self {
        let inner = Arc::new(UDPBroadcastInner {
            socket: Mutex::new(None),
            broadcast_addr: Mutex::new(None),
        });

        let inner_clone = inner.clone();
        let mask = subnet_mask.to_string();

        tokio::spawn(async move {
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => {
                    s.set_broadcast(true).ok();
                    s
                }
                Err(e) => {
                    debug_log!("[UDP-Broadcast] Failed to create socket: {}", e);
                    return;
                }
            };

            // Compute broadcast address
            let broadcast = compute_broadcast_address(&mask);
            let addr = format!("{}:{}", broadcast, port);
            debug_log!("[UDP-Broadcast] Broadcasting to {}", addr);

            *inner_clone.socket.lock().await = Some(Arc::new(socket));
            *inner_clone.broadcast_addr.lock().await = Some(addr);
        });

        Self { inner }
    }
}

impl OutputProtocol for UDPBroadcastOutputProtocol {
    async fn init(&self) {
        // Everything set up in new()
    }

    async fn send(self: Arc<Self>, message: &str) {
        let socket_guard = self.inner.socket.lock().await;
        let addr_guard = self.inner.broadcast_addr.lock().await;
        if let Some(ref socket) = *socket_guard {
            if let Some(ref addr) = *addr_guard {
                let bytes = format!("{}\n", message);
                let _ = socket.send_to(bytes.as_bytes(), addr).await;
            }
        }
    }
}

/// Compute the broadcast address from the local machine's IP and subnet mask.
fn compute_broadcast_address(subnet_mask: &str) -> String {
    // Try to find a local IPv4 address
    if let Ok(addr) = get_local_ipv4() {
        let mask: u32 = subnet_mask
            .split('.')
            .filter_map(|o| o.parse::<u32>().ok())
            .fold(0u32, |acc, o| (acc << 8) | o);

        let ip_parts: Vec<u32> = addr
            .split('.')
            .filter_map(|o| o.parse::<u32>().ok())
            .collect();

        if ip_parts.len() == 4 && mask != 0 {
            let ip_int = ip_parts.iter().fold(0u32, |acc, o| (acc << 8) | o);
            let broadcast_int = ip_int | !mask;
            let b = broadcast_int.to_be_bytes();
            return format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
        }
    }

    // Fallback
    "255.255.255.255".to_string()
}

/// Get the local machine's primary IPv4 address.
fn get_local_ipv4() -> Result<String, std::io::Error> {
    // Use UDP connect trick to find the local IP
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    let local = socket.local_addr()?;
    Ok(local.ip().to_string())
}
