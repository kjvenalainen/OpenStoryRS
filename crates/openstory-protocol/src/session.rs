//! Platform-independent session events and transport contract.
//!
//! Native TCP and browser WebSocket adapters implement this interface so the
//! client packet handlers can consume the same decrypted Maple packets.

/// Events delivered from a connected login or channel session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    /// The handshake completed. `redirected` is true for a channel connection
    /// reached through the login server's server-IP redirect.
    Connected { redirected: bool },
    /// A complete decrypted Maple packet, without its frame header.
    Packet(Vec<u8>),
    /// The session ended before the client requested it to close.
    Disconnected(String),
}

/// A nonblocking, platform-neutral Maple session.
///
/// Implementations queue outgoing packets and expose complete incoming packets
/// through `try_recv`, allowing the client event loop to remain responsive.
pub trait SessionTransport {
    /// Queue a packet for transmission on the current session.
    fn send_packet(&self, packet: Vec<u8>) -> Result<(), String>;

    /// Replace the current session and send `first_packet` after its handshake.
    fn reconnect(&self, address: String, first_packet: Vec<u8>) -> Result<(), String>;

    /// Return the next event if one is already available.
    fn try_recv(&self) -> Option<SessionEvent>;
}
