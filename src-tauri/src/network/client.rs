use anyhow::{bail, Result};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot};

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    InputPacket, PacketHeader, HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT,
    HS_SERVER_HELLO, HS_SESSION_CODE,
};
use crate::input::simulation::InputSimulator;

pub const CLIENT_UDP_PORT: u16 = 24802;

pub struct ClientHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl ClientHandle {
    pub fn disconnect(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Connect to a server and start receiving/simulating input events
pub async fn connect_to_server(
    server_host: &str,
    session_code: &str,
    status_tx: mpsc::UnboundedSender<String>,
) -> Result<ClientHandle> {
    let server_tcp_addr = format!("{}:24800", server_host);
    let mut stream = TcpStream::connect(&server_tcp_addr).await?;
    log::info!("TCP connected to {}", server_tcp_addr);

    // --- Handshake ---

    // Step 1: Send session code
    let code_bytes = session_code.as_bytes();
    stream.write_u8(HS_SESSION_CODE).await?;
    stream.write_u8(code_bytes.len() as u8).await?;
    stream.write_all(code_bytes).await?;
    stream.flush().await?;

    // Step 2: Send client public key
    let client_kp = EphemeralKeypair::generate();
    stream.write_u8(HS_CLIENT_HELLO).await?;
    stream.write_all(client_kp.public.as_bytes()).await?;
    stream.flush().await?;

    // Step 3: Read server public key
    let msg_type = stream.read_u8().await?;
    if msg_type == HS_REJECT {
        bail!("Server rejected connection (invalid session code)");
    }
    if msg_type != HS_SERVER_HELLO {
        bail!("Unexpected handshake message: 0x{:02X}", msg_type);
    }
    let mut server_pubkey_bytes = [0u8; 32];
    stream.read_exact(&mut server_pubkey_bytes).await?;
    let server_pubkey = x25519_dalek::PublicKey::from(server_pubkey_bytes);

    // Compute shared secret
    let shared = client_kp.diffie_hellman(&server_pubkey);
    let session_key = derive_session_key(&shared, session_code);

    // Step 4: Nonce exchange
    let msg_type = stream.read_u8().await?;
    if msg_type != HS_NONCE_EXCHANGE {
        bail!("Expected HS_NONCE_EXCHANGE, got 0x{:02X}", msg_type);
    }
    let mut server_base_nonce = [0u8; 12];
    stream.read_exact(&mut server_base_nonce).await?;

    let client_base_nonce = SessionCipher::generate_base_nonce();
    stream.write_u8(HS_NONCE_EXCHANGE).await?;
    stream.write_all(&client_base_nonce).await?;

    // Step 5: Send UDP port
    stream.write_u16(CLIENT_UDP_PORT).await?;
    stream.flush().await?;

    let combined_nonce = combine_nonces(&server_base_nonce, &client_base_nonce);
    let cipher = Arc::new(SessionCipher::new(&session_key, combined_nonce));

    // Wait for ACK
    let ack = stream.read_u8().await?;
    if ack != HS_ACK {
        bail!("Server did not acknowledge: 0x{:02X}", ack);
    }

    log::info!("Handshake complete. Session established.");
    let _ = status_tx.send("connected".to_string());

    // Bind UDP socket for receiving input events
    let udp_bind = format!("0.0.0.0:{}", CLIENT_UDP_PORT);
    let udp_socket = Arc::new(UdpSocket::bind(&udp_bind).await?);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    // Spawn UDP receive + simulate loop
    let cipher_udp = cipher.clone();
    let status_tx_clone = status_tx.clone();
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let mut pkt_counter: u64 = 0;
        let mut simulator = match InputSimulator::new() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create input simulator: {}", e);
                let _ = status_tx_clone.send(format!("error: {}", e));
                return;
            }
        };

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    log::info!("Client shutdown signal received");
                    break;
                }
                result = udp_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, _src)) => {
                            let encrypted = &buf[..len];
                            match cipher_udp.decrypt(encrypted, pkt_counter) {
                                Ok(plain) => {
                                    if let Ok(header) = PacketHeader::from_bytes(&plain) {
                                        let payload = &plain[12..];
                                        if let Err(e) = simulator.dispatch(&header, payload) {
                                            log::warn!("Simulation error: {}", e);
                                        }
                                    }
                                    pkt_counter = pkt_counter.wrapping_add(1);
                                }
                                Err(e) => {
                                    log::warn!("Decrypt error (counter={}): {}", pkt_counter, e);
                                    // Try to resync by allowing a window of counters
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("UDP recv error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        let _ = status_tx_clone.send("disconnected".to_string());
    });

    Ok(ClientHandle { shutdown_tx })
}
