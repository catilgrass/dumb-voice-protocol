use std::sync::Arc;

#[cfg(unix)]
pub mod ipc;
pub mod stderr;
pub mod stdout;
pub mod tcp;
pub mod udp;
pub mod udp_broadcast;

#[cfg(unix)]
pub use ipc::*;
pub use stderr::*;
pub use stdout::*;
pub use tcp::*;
pub use udp::*;
pub use udp_broadcast::*;

/// Defines the protocol for an output channel that can be initialized and used to send messages.
///
/// Implementors of this trait provide the underlying mechanism for message delivery,
/// such as writing to stdout, a file, or a network socket.
///
/// # Type Parameters
///
/// The trait methods return `impl Future<Output = ()> + Send + Sync`, allowing
/// asynchronous implementations that can be shared across threads.
pub trait OutputProtocol {
    /// Initializes the output channel.
    ///
    /// Prepares the output resource (e.g., opening a file, establishing a connection).
    ///
    /// # Returns
    ///
    /// A future that resolves when initialization is complete.
    fn init(&self) -> impl Future<Output = ()> + Send + Sync;

    /// Sends a message string through the output channel.
    ///
    /// This method takes `self: Arc<Self>`, allowing the output channel to be shared
    /// across multiple concurrent senders (e.g., in multi-threaded or async contexts).
    ///
    /// # Arguments
    ///
    /// * `str` - The string message to send.
    ///
    /// # Returns
    ///
    /// A future that resolves when the message has been sent.
    fn send(self: Arc<Self>, str: &str) -> impl Future<Output = ()> + Send + Sync;
}
