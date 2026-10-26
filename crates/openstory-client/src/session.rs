use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::thread;
use std::time::Duration;

use openstory_protocol::{MapleCipher, MapleFrameDecoder, SessionEvent, SessionTransport};

const HANDSHAKE_LENGTH: usize = 16;
const READ_BUFFER_SIZE: usize = 8 * 1024;

/// Creates the native session implementation behind the shared client-facing
/// interface. A browser adapter can replace this factory without changing the
/// login and in-game packet consumers.
pub fn connect(address: String) -> Box<dyn SessionTransport> {
    Box::new(SessionWorker::connect(address))
}

enum SessionCommand {
    SendPacket(Vec<u8>),
    Reconnect {
        address: String,
        first_packet: Vec<u8>,
    },
}

pub struct SessionWorker {
    commands: Sender<SessionCommand>,
    events: Receiver<SessionEvent>,
}

impl SessionWorker {
    pub fn connect(address: String) -> Self {
        let (commands, command_receiver) = mpsc::channel();
        let (event_sender, events) = mpsc::sync_channel(256);
        let thread_events = event_sender.clone();
        let spawn_result = thread::Builder::new()
            .name("openstory-network".to_owned())
            .spawn(move || run_session(&address, command_receiver, thread_events));

        if let Err(error) = spawn_result {
            let _ = event_sender.send(SessionEvent::Disconnected(format!(
                "could not start network worker: {error}"
            )));
        }

        Self { commands, events }
    }

    pub fn send_packet(&self, packet: Vec<u8>) -> Result<(), String> {
        self.commands
            .send(SessionCommand::SendPacket(packet))
            .map_err(|_| "network worker is no longer available".to_owned())
    }

    pub fn reconnect(&self, address: String, first_packet: Vec<u8>) -> Result<(), String> {
        self.commands
            .send(SessionCommand::Reconnect {
                address,
                first_packet,
            })
            .map_err(|_| "network worker is no longer available".to_owned())
    }

    pub fn try_recv(&self) -> Option<SessionEvent> {
        match self.events.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

impl SessionTransport for SessionWorker {
    fn send_packet(&self, packet: Vec<u8>) -> Result<(), String> {
        SessionWorker::send_packet(self, packet)
    }

    fn reconnect(&self, address: String, first_packet: Vec<u8>) -> Result<(), String> {
        SessionWorker::reconnect(self, address, first_packet)
    }

    fn try_recv(&self) -> Option<SessionEvent> {
        SessionWorker::try_recv(self)
    }
}

fn run_session(
    address: &str,
    commands: Receiver<SessionCommand>,
    events: SyncSender<SessionEvent>,
) {
    let mut next_address = address.to_owned();
    let mut first_packet = None;
    let mut redirected = false;
    loop {
        match run_connected_session(
            &next_address,
            first_packet.take(),
            redirected,
            &commands,
            &events,
        ) {
            Ok(Some((address, packet))) => {
                next_address = address;
                first_packet = Some(packet);
                redirected = true;
            }
            Ok(None) => return,
            Err(message) => {
                debug_log(&format!(
                    "OpenStory network: session disconnected: {message}"
                ));
                let _ = events.send(SessionEvent::Disconnected(message));
                return;
            }
        }
    }
}

fn run_connected_session(
    address: &str,
    first_packet: Option<Vec<u8>>,
    redirected: bool,
    commands: &Receiver<SessionCommand>,
    events: &SyncSender<SessionEvent>,
) -> Result<Option<(String, Vec<u8>)>, String> {
    let socket_addresses = address
        .to_socket_addrs()
        .map_err(|error| format!("invalid server address {address:?}: {error}"))?
        .collect::<Vec<_>>();
    if socket_addresses.is_empty() {
        return Err(format!(
            "server address {address:?} resolved to no endpoints"
        ));
    }
    let mut last_connect_error = None;
    let mut connected = None;
    for socket_address in socket_addresses.iter().take(8) {
        match TcpStream::connect_timeout(socket_address, Duration::from_secs(5)) {
            Ok(stream) => {
                connected = Some((stream, socket_address));
                break;
            }
            Err(error) => last_connect_error = Some((socket_address, error)),
        }
    }
    let (mut stream, _) = connected.ok_or_else(|| {
        let (address, error) = last_connect_error.expect("at least one endpoint was attempted");
        format!("could not connect to {address}: {error}")
    })?;
    stream
        .set_nodelay(true)
        .map_err(|error| format!("could not configure TCP socket: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("could not configure handshake timeout: {error}"))?;

    // read_exact handles servers that split the 16-byte handshake across TCP reads.
    let mut handshake = [0; HANDSHAKE_LENGTH];
    stream
        .read_exact(&mut handshake)
        .map_err(|error| format!("could not read Maple handshake: {error}"))?;
    debug_log(&format!(
        "OpenStory network: received Maple handshake from {address}"
    ));
    let cipher = MapleCipher::from_handshake(&handshake).map_err(|error| error.to_string())?;
    debug_log(&format!(
        "OpenStory network: session IVs send={:02X?}, receive={:02X?}",
        cipher.send_iv(),
        cipher.receive_iv()
    ));
    let mut decoder = MapleFrameDecoder::new(cipher);
    if let Some(packet) = first_packet {
        debug_packet("sending channel login", &packet);
        let frame = decoder
            .encode_frame(&packet)
            .map_err(|error| format!("could not encode channel login packet: {error}"))?;
        stream
            .write_all(&frame)
            .map_err(|error| format!("could not send channel login packet: {error}"))?;
    }
    stream
        .set_read_timeout(Some(Duration::from_millis(25)))
        .map_err(|error| format!("could not configure socket read timeout: {error}"))?;
    events
        .send(SessionEvent::Connected { redirected })
        .map_err(|_| "client closed the network event channel".to_owned())?;
    debug_log("OpenStory network: queued connected event for the client");

    let mut read_buffer = [0; READ_BUFFER_SIZE];
    loop {
        loop {
            match commands.try_recv() {
                Ok(SessionCommand::SendPacket(packet)) => {
                    debug_packet("sending", &packet);
                    let frame = decoder
                        .encode_frame(&packet)
                        .map_err(|error| format!("could not encode packet: {error}"))?;
                    stream
                        .write_all(&frame)
                        .map_err(|error| format!("could not send packet: {error}"))?;
                }
                Ok(SessionCommand::Reconnect {
                    address,
                    first_packet,
                }) => return Ok(Some((address, first_packet))),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(None),
            }
        }

        match stream.read(&mut read_buffer) {
            Ok(0) => return Err("server closed the connection".to_owned()),
            Ok(length) => {
                let packets = decoder
                    .push(&read_buffer[..length])
                    .map_err(|error| format!("invalid server packet stream: {error}"))?;
                for packet in packets {
                    debug_packet("received", &packet);
                    events
                        .send(SessionEvent::Packet(packet))
                        .map_err(|_| "client closed the network event channel".to_owned())?;
                }
            }
            Err(error) if is_read_timeout(&error) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(format!("could not read server packet: {error}")),
        }
    }
}

fn network_debug_enabled() -> bool {
    std::env::var("OPENSTORY_DEBUG_NETWORK").as_deref() == Ok("1")
}

fn debug_log(message: &str) {
    if !network_debug_enabled() {
        return;
    }
    eprintln!("{message}");
    if let Some(path) = std::env::var_os("OPENSTORY_DEBUG_LOG") {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{message}");
        }
    }
}

fn debug_packet(action: &str, packet: &[u8]) {
    if network_debug_enabled() {
        if let Some(opcode) = packet.get(..2) {
            debug_log(&format!(
                "OpenStory network: {action} packet opcode {}",
                u16::from_le_bytes([opcode[0], opcode[1]])
            ));
        } else {
            debug_log(&format!("OpenStory network: {action} malformed packet"));
        }
    }
}

fn is_read_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
