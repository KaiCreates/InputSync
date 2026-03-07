use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT, HS_SERVER_HELLO,
    HS_SESSION_CODE, InputPacket,
};

pub const SERVER_TCP_PORT: u16 = 24800;
pub const SERVER_UDP_PORT: u16 = 24801;

/// State for one connected client
struct ConnectedClient {
    addr: SocketAddr,
    udp_addr: SocketAddr,
    cipher: Arc<SessionCipher>,
}

pub struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl ServerHandle {
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Start the server:
/// 1. Listens on TCP for client connections / handshakes
/// 2. After handshake, receives input packets on UDP and relays encrypted to clients
/// 3. `input_rx` receives captured input packets to forward to clients
pub async fn start_server(
    session_code: String,
    input_rx: mpsc::UnboundedReceiver<InputPacket>,
) -> Result<ServerHandle> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let tcp_addr = format!("0.0.0.0:{}", SERVER_TCP_PORT);
    let udp_addr = format!("0.0.0.0:{}", SERVER_UDP_PORT);

    let tcp_listener = TcpListener::bind(&tcp_addr).await?;
    let udp_socket = Arc::new(UdpSocket::bind(&udp_addr).await?);

    log::info!("Server listening on TCP {} and UDP {}", tcp_addr, udp_addr);

    let clients: Arc<Mutex<Vec<ConnectedClient>>> = Arc::new(Mutex::new(Vec::new()));
    let session_code = Arc::new(session_code);

    // Spawn TCP accept loop
    let clients_tcp = clients.clone();
    let code_clone = session_code.clone();
    tokio::spawn(async move {
        loop {
            match tcp_listener.accept().await {
                Ok((stream, addr)) => {
                    log::info!("New connection from {}", addr);
                    let clients = clients_tcp.clone();
                    let code = code_clone.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client_handshake(stream, addr, code, clients).await {
                            log::warn!("Handshake failed for {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("TCP accept error: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn UDP broadcast loop
    let udp = udp_socket.clone();
    let clients_udp = clients.clone();
    let mut input_rx = input_rx;
    tokio::spawn(async move {
        let mut pkt_counter: u64 = 0;
        while let Some(pkt) = input_rx.recv().await {
            let wire = pkt.to_wire();
            let locked = clients_udp.lock().await;
            for client in locked.iter() {
                match client.cipher.encrypt(&wire, pkt_counter) {
                    Ok(encrypted) => {
                        if let Err(e) = udp.send_to(&encrypted, client.udp_addr).await {
                            log::warn!("UDP send to {} failed: {}", client.udp_addr, e);
                        }
                    }
                    Err(e) => log::warn!("Encrypt failed: {}", e),
                }
            }
            pkt_counter = pkt_counter.wrapping_add(1);
        }
    });

    Ok(ServerHandle { shutdown_tx })
}

/// Full handshake for a new TCP connection:
/// 1. Receive session code from client
/// 2. Validate it
/// 3. ECDH key exchange
/// 4. Nonce exchange
/// 5. Register client for UDP delivery
async fn handle_client_handshake(
    mut stream: TcpStream,
    addr: SocketAddr,
    expected_code: Arc<String>,
    clients: Arc<Mutex<Vec<ConnectedClient>>>,
) -> Result<()> {
    // Step 1: Read session code (HS_SESSION_CODE | 1 byte len | N bytes code)
    let msg_type = stream.read_u8().await?;
    if msg_type != HS_SESSION_CODE {
        stream.write_u8(HS_REJECT).await?;
        bail!("Expected HS_SESSION_CODE, got 0x{:02X}", msg_type);
    }
    let code_len = stream.read_u8().await? as usize;
    let mut code_buf = vec![0u8; code_len];
    stream.read_exact(&mut code_buf).await?;
    let received_code = String::from_utf8(code_buf)?;

    if received_code.trim() != expected_code.as_str() {
        stream.write_u8(HS_REJECT).await?;
        bail!("Invalid session code: {}", received_code);
    }
    log::info!("Session code validated for {}", addr);

    // Step 2: Read client public key (HS_CLIENT_HELLO | pubkey[32])
    let msg_type = stream.read_u8().await?;
    if msg_type != HS_CLIENT_HELLO {
        stream.write_u8(HS_REJECT).await?;
        bail!("Expected HS_CLIENT_HELLO, got 0x{:02X}", msg_type);
    }
    let mut client_pubkey_bytes = [0u8; 32];
    stream.read_exact(&mut client_pubkey_bytes).await?;
    let client_pubkey = x25519_dalek::PublicKey::from(client_pubkey_bytes);

    // Step 3: Generate server keypair, send server public key
    let server_kp = EphemeralKeypair::generate();
    let server_pubkey_bytes = server_kp.public.to_bytes();

    stream.write_u8(HS_SERVER_HELLO).await?;
    stream.write_all(&server_pubkey_bytes).await?;
    stream.flush().await?;

    // Compute shared secret and derive session key
    let shared = server_kp.diffie_hellman(&client_pubkey);
    let session_key = derive_session_key(&shared, &expected_code);

    // Step 4: Nonce exchange (server sends its nonce, receives client nonce)
    let server_base_nonce = SessionCipher::generate_base_nonce();
    stream.write_u8(HS_NONCE_EXCHANGE).await?;
    stream.write_all(&server_base_nonce).await?;
    stream.flush().await?;

    let msg_type = stream.read_u8().await?;
    if msg_type != HS_NONCE_EXCHANGE {
        bail!("Expected HS_NONCE_EXCHANGE, got 0x{:02X}", msg_type);
    }
    let mut client_base_nonce = [0u8; 12];
    stream.read_exact(&mut client_base_nonce).await?;

    // Combined nonce prevents replay even if one side is compromised
    let combined_nonce = combine_nonces(&server_base_nonce, &client_base_nonce);
    let cipher = Arc::new(SessionCipher::new(&session_key, combined_nonce));

    // Step 5: Read client UDP port
    let client_udp_port = stream.read_u16().await?;
    let client_udp_addr: SocketAddr = format!("{}:{}", addr.ip(), client_udp_port).parse()?;

    // Acknowledge
    stream.write_u8(HS_ACK).await?;
    stream.flush().await?;

    log::info!(
        "Client {} authenticated. UDP delivery → {}",
        addr,
        client_udp_addr
    );

    clients.lock().await.push(ConnectedClient {
        addr,
        udp_addr: client_udp_addr,
        cipher,
    });

    // Keep the TCP connection alive for future control messages / disconnect detection
    let mut buf = [0u8; 1];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                log::info!("Client {} disconnected", addr);
                break;
            }
            Err(e) => {
                log::warn!("Client {} TCP error: {}", addr, e);
                break;
            }
            _ => {}
        }
    }

    // Remove client
    clients.lock().await.retain(|c| c.addr != addr);
    Ok(())
}
