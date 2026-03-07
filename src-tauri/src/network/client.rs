/// Fix #1: InputSimulator runs on a dedicated std::thread (enigo requires non-tokio context)
/// Fix #4: Counter window resync — try up to COUNTER_WINDOW ahead before failing
/// Fix #13: Errors propagated back via status_tx channel
use anyhow::{bail, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot};

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    PacketHeader, HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT,
    HS_SERVER_HELLO, HS_SESSION_CODE,
};
use crate::input::simulation::InputSimulator;

pub const CLIENT_UDP_PORT: u16 = 24802;

/// Max packets to look ahead when counter drifts due to drops
const COUNTER_WINDOW: u64 = 64;

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
    status_tx: mpsc::UnboundedSender<String>,
) -> Result<ClientHandle> {
    let server_tcp_addr = format!("{}:24800", server_host);
    let mut stream = TcpStream::connect(&server_tcp_addr).await?;
    log::info!("TCP connected to {}", server_tcp_addr);

    // ── Handshake ──────────────────────────────────────────────────────────

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

    // Step 5: Send UDP listen port
    stream.write_u16(CLIENT_UDP_PORT).await?;
    stream.flush().await?;

    let combined_nonce = combine_nonces(&server_base_nonce, &client_base_nonce);
    let cipher = Arc::new(SessionCipher::new(&session_key, combined_nonce));

    let ack = stream.read_u8().await?;
    if ack != HS_ACK {
        bail!("Server did not acknowledge: 0x{:02X}", ack);
    }

    log::info!("Handshake complete — session established.");
    let _ = status_tx.send("connected".to_string());

    // ── UDP receive socket ─────────────────────────────────────────────────
    let udp_socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", CLIENT_UDP_PORT)).await?);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    // ── Simulator thread (Fix #1) ──────────────────────────────────────────
    // enigo requires a non-tokio OS thread. We send decoded plain packets via
    // a sync channel; the simulator thread blocks on recv() and dispatches.
    let (sim_tx, sim_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let status_sim = status_tx.clone();
    std::thread::Builder::new()
        .name("inputsync-simulator".into())
        .spawn(move || {
            let mut simulator = match InputSimulator::new() {
                Ok(s) => s,
                Err(e) => {
                    log::error!("InputSimulator init failed: {}", e);
                    let _ = status_sim.send(format!("simulator_error: {}", e));
                    return;
                }
            };
            log::info!("Input simulator thread ready");

            // Block waiting for plaintext packets from the UDP receiver task
            while let Ok(plain) = sim_rx.recv() {
                if plain.len() < 12 {
                    continue;
                }
                if let Ok(header) = PacketHeader::from_bytes(&plain) {
                    let payload = &plain[12..];
                    if let Err(e) = simulator.dispatch(&header, payload) {
                        log::debug!("Simulation dispatch: {}", e);
                    }
                }
            }
            log::info!("Input simulator thread exiting");
        })
        .expect("Failed to spawn simulator thread");

    // ── UDP receive task with counter-window resync (Fix #4) ───────────────
    let cipher_udp = cipher.clone();
    let status_clone = status_tx.clone();
    let counter = Arc::new(AtomicU64::new(0));

    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    log::info!("Client UDP loop: shutdown signal");
                    break;
                }
                result = udp_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, _src)) => {
                            let encrypted = &buf[..len];
                            let current = counter.load(Ordering::Relaxed);

                            // Try current counter first, then look forward up to COUNTER_WINDOW.
                            // Forward-only window preserves replay protection.
                            let mut decrypted: Option<(Vec<u8>, u64)> = None;
                            for delta in 0..=COUNTER_WINDOW {
                                if let Ok(plain) = cipher_udp.decrypt(encrypted, current + delta) {
                                    decrypted = Some((plain, current + delta));
                                    break;
                                }
                            }

                            match decrypted {
                                Some((plain, matched)) => {
                                    // Advance counter past the matched position
                                    counter.store(matched + 1, Ordering::Relaxed);
                                    if delta_skipped(matched, current) > 0 {
                                        log::debug!(
                                            "Counter resync: skipped {} dropped packets",
                                            matched - current
                                        );
                                    }
                                    let _ = sim_tx.send(plain);
                                }
                                None => {
                                    log::warn!(
                                        "Decrypt failed for counter window [{}, {}] — dropping",
                                        current,
                                        current + COUNTER_WINDOW
                                    );
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

        // sim_tx dropped here → simulator thread exits its recv() loop cleanly
        let _ = status_clone.send("disconnected".to_string());
    });

    Ok(ClientHandle { shutdown_tx })
}

#[inline]
fn delta_skipped(matched: u64, current: u64) -> u64 {
    matched.saturating_sub(current)
}
