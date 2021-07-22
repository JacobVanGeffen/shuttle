//! Shuttle's implementation of [`tokio::net`].

pub mod tcp;

// TODO Should I implement this?
pub use tokio::net::TcpSocket;
pub use tokio::net::unix;
pub use tcp::TcpStream;
pub use tcp::TcpListener;