use anyhow::{bail, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot};
use std::time::Duration;

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    PacketHeader, HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT, HS_SERVER_HELLO,
    HS_SESSION_CODE,
};
use crate::input::simulation::InputSimulator;

const COUNTER_WINDOW: u64 = 64;
/// Bounded simulator channel capacity — drops old packets when the
/// simulator thread falls behind, preventing unbounded heap growth.
const SIM_CHANNEL_CAP: usize = 512;

pub struct ClientHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl ClientHandle {
    pub fn disconnect(self) {
        let _ = self.shutdown_tx.send(());
    }
}

pub async fn connect_to_server(
    server_host: &str,
    session_code: &str,
    control_port: u16,
    status_tx: mpsc::UnboundedSender<String>,
    tls_connector: Option<tokio_rustls::TlsConnector>,
) -> Result<ClientHandle> {
    let server_tcp_addr = format!("{}:{}", server_host, control_port);
    let tcp_stream = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        TcpStream::connect(&server_tcp_addr)
    ).await
    .map_err(|_| anyhow::anyhow!("Connection timed out (server unreachable)"))??;
    tracing::info!("TCP connected to {}", server_tcp_addr);

    if let Some(connector) = tls_connector {
        // Handle both IP addresses and DNS hostnames.
        // AcceptAnyCert ignores the server name for validation, but rustls
        // still requires a syntactically valid ServerName.
        let server_name = server_host
            .parse::<std::net::IpAddr>()
            .map(|ip| rustls::pki_types::ServerName::IpAddress(ip.into()))
            .unwrap_or_else(|_| {
                rustls::pki_types::ServerName::try_from(server_host.to_string())
                    .unwrap_or_else(|_| {
                        rustls::pki_types::ServerName::try_from("inputsync.local")
                            .expect("inputsync.local is a valid DNS name")
                    })
            });
        let tls_stream = connector.connect(server_name, tcp_stream).await?;
        tracing::info!("TLS handshake complete");
        do_handshake_and_run(tls_stream, session_code, status_tx).await
    } else {
        do_handshake_and_run(tcp_stream, session_code, status_tx).await
    }
}

async fn do_handshake_and_run<S>(
    mut stream: S,
    session_code: &str,
    status_tx: mpsc::UnboundedSender<String>,
) -> Result<ClientHandle>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
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
        bail!("Server rejected connection — check session code");
    }
    if msg_type != HS_SERVER_HELLO {
        bail!("Unexpected handshake message: 0x{:02X}", msg_type);
    }
    let mut server_pubkey_bytes = [0u8; 32];
    stream.read_exact(&mut server_pubkey_bytes).await?;
    let server_pubkey = x25519_dalek::PublicKey::from(server_pubkey_bytes);

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

    // Step 5: Bind UDP to OS-assigned port (port 0) to avoid hardcoded port
    // conflicts when two clients run on the same machine. Report actual port
    // to the server so it knows where to send events.
    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let actual_udp_port = udp_socket.local_addr()?.port();
    stream.write_u16(actual_udp_port).await?;
    stream.flush().await?;

    let combined_nonce = combine_nonces(&server_base_nonce, &client_base_nonce);
    let cipher = Arc::new(SessionCipher::new(&session_key, combined_nonce));

    let ack = stream.read_u8().await?;
    if ack != HS_ACK {
        bail!("Server did not acknowledge: 0x{:02X}", ack);
    }

    tracing::info!("Handshake complete — session established. UDP port: {}", actual_udp_port);
    let _ = status_tx.send("connected".to_string());

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    // ── Simulator thread ─────────────────────────────────────────────────
    // Bounded sync_channel prevents heap growth when simulator falls behind.
    // The async UDP task uses try_send to avoid blocking the executor.
    let (sim_tx, sim_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(SIM_CHANNEL_CAP);
    let status_sim = status_tx.clone();
    std::thread::Builder::new()
        .name("inputsync-simulator".into())
        .spawn(move || {
            let mut simulator = match InputSimulator::new() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("InputSimulator init failed: {}", e);
                    let _ = status_sim.send(format!("simulator_error: {}", e));
                    return;
                }
            };
            tracing::info!("Input simulator thread ready");

            while let Ok(plain) = sim_rx.recv() {
                if plain.len() < 12 {
                    continue;
                }
                if let Ok(header) = PacketHeader::from_bytes(&plain) {
                    let payload = &plain[12..];
                    if let Err(e) = simulator.dispatch(&header, payload) {
                        tracing::debug!("Simulation dispatch: {}", e);
                    }
                }
            }
            tracing::info!("Input simulator thread exiting");
        })
        .expect("Failed to spawn simulator thread");

    // ── UDP receive task ─────────────────────────────────────────────────
    let cipher_udp = cipher.clone();
    let status_clone = status_tx.clone();
    let counter = Arc::new(AtomicU64::new(0));

    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let mut ping_interval = tokio::time::interval(Duration::from_secs(5));
        let mut last_pkt_time = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    tracing::info!("Client UDP loop: shutdown signal");
                    break;
                }
                _ = ping_interval.tick() => {
                    // If no packets received for 15 seconds, assume disconnected
                    if last_pkt_time.elapsed() > Duration::from_secs(15) {
                        tracing::warn!("Connection timeout (no UDP packets for 15s)");
                        break;
                    }
                }
                result = udp_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, _src)) => {
                            last_pkt_time = std::time::Instant::now();
                            let encrypted = &buf[..len];
                            let current = counter.load(Ordering::Relaxed);

                            let mut decrypted: Option<(Vec<u8>, u64)> = None;
                            for delta in 0..=COUNTER_WINDOW {
                                if let Ok(plain) = cipher_udp.decrypt(encrypted, current + delta) {
                                    decrypted = Some((plain, current + delta));
                                    break;
                                }
                            }

                            match decrypted {
                                Some((plain, matched)) => {
                                    counter.store(matched + 1, Ordering::Relaxed);

                                    // Handle Enter/Exit signals and Pings immediately
                                    if !plain.is_empty() {
                                        match plain[0] {
                                            crate::core::protocol::PKT_ENTER_SCREEN => tracing::debug!("Received signal: ENTER_SCREEN"),
                                            crate::core::protocol::PKT_EXIT_SCREEN => tracing::debug!("Received signal: EXIT_SCREEN"),
                                            crate::core::protocol::PKT_PING => {
                                                // Server heartbeat received
                                            }
                                            _ => {}
                                        }
                                    }

                                    if matched > current {
                                        tracing::debug!(
                                            "Counter resync: skipped {} dropped packets",
                                            matched - current
                                        );
                                    }
                                    // try_send: discard packet if simulator is backed up
                                    if sim_tx.try_send(plain).is_err() {
                                        tracing::trace!("Simulator queue full — packet dropped");
                                    }
                                }
                                None => {
                                    tracing::warn!(
                                        "Decrypt failed for counter window [{}, {}] — dropping",
                                        current,
                                        current + COUNTER_WINDOW
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("UDP recv error: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        let _ = status_clone.send("disconnected".to_string());
    });

    Ok(ClientHandle { shutdown_tx })
}