use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT, HS_SERVER_HELLO, HS_SESSION_CODE,
    InputPacket,
};

const MAX_CLIENTS: usize = 10;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ServerHandle {
    cancel: CancellationToken,
    /// Join handle for the TCP accept task. Awaiting this guarantees the
    /// TcpListener has been dropped (port released) before returning.
    tcp_task: tokio::task::JoinHandle<()>,
}

impl ServerHandle {
    /// Cancel the server and wait until the TCP accept loop has fully exited,
    /// ensuring the port is released before the caller attempts to rebind.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        let _ = self.tcp_task.await;
    }
}

struct ConnectedClient {
    addr: SocketAddr,
    udp_addr: SocketAddr,
    cipher: Arc<SessionCipher>,
}

pub async fn start_server(
    session_code: String,
    input_rx: mpsc::Receiver<InputPacket>,
    client_count: Arc<AtomicUsize>,
    control_port: u16,
    udp_port: u16,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
) -> Result<ServerHandle> {
    let cancel = CancellationToken::new();

    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", control_port)).await?;
    let udp_socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", udp_port)).await?);

    tracing::info!(
        "Server listening — TCP :{}  UDP :{}",
        control_port,
        udp_port
    );

    let clients: Arc<Mutex<Vec<ConnectedClient>>> = Arc::new(Mutex::new(Vec::new()));
    let session_code = Arc::new(session_code);
    let tls_acceptor = tls_acceptor.map(Arc::new);

    // ── TCP accept loop ──────────────────────────────────────────────────
    let clients_tcp = clients.clone();
    let code_clone = session_code.clone();
    let count_tcp = client_count.clone();
    let cancel_tcp = cancel.clone();
    let tls_clone = tls_acceptor.clone();

    let tcp_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cancel_tcp.cancelled() => {
                    tracing::info!("TCP accept loop stopped, port {} released", control_port);
                    // tcp_listener dropped here, releasing the port immediately.
                    break;
                }
                result = tcp_listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            tracing::info!("Incoming connection from {}", addr);
                            let clients = clients_tcp.clone();
                            let code = code_clone.clone();
                            let count = count_tcp.clone();
                            let tls = tls_clone.clone();
                            tokio::spawn(async move {
                                let result = tokio::time::timeout(
                                    HANDSHAKE_TIMEOUT,
                                    async {
                                        if let Some(acceptor) = tls {
                                            match acceptor.accept(stream).await {
                                                Ok(tls_stream) => {
                                                    handle_client_handshake(
                                                        tls_stream, addr, code, clients, count,
                                                    ).await
                                                }
                                                Err(e) => Err(anyhow::anyhow!("TLS accept failed: {}", e)),
                                            }
                                        } else {
                                            handle_client_handshake(stream, addr, code, clients, count).await
                                        }
                                    }
                                ).await;

                                match result {
                                    Ok(Ok(())) => tracing::info!("Client {} session ended", addr),
                                    Ok(Err(e)) => tracing::warn!("Session error for {}: {}", addr, e),
                                    Err(_) => tracing::warn!("Handshake timeout for {}", addr),
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("TCP accept error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });
// ── UDP broadcast loop ───────────────────────────────────────────────
let udp = udp_socket.clone();
let clients_udp = clients.clone();
let mut input_rx = input_rx;
let cancel_udp = cancel.clone();
tokio::spawn(async move {
    let mut pkt_counter: u64 = 0;
    let mut ping_interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            biased;
            _ = cancel_udp.cancelled() => {
                tracing::info!("UDP broadcast loop stopped");
                break;
            }
            _ = ping_interval.tick() => {
                // Send heartbeats to keep UDP NAT mappings open and detect dead clients
                let targets: Vec<(Arc<SessionCipher>, SocketAddr)> = {
                    let locked = clients_udp.lock().await;
                    locked.iter().map(|c| (c.cipher.clone(), c.udp_addr)).collect()
                };

                let ping_pkt = InputPacket::ping(0).to_wire();
                for (cipher, addr) in &targets {
                    if let Ok(encrypted) = cipher.encrypt(&ping_pkt, pkt_counter) {
                        let _ = udp.send_to(&encrypted, *addr).await;
                    }
                }
                pkt_counter = pkt_counter.wrapping_add(1);
            }
            maybe_pkt = input_rx.recv() => {
...
                    let pkt = match maybe_pkt {
                        Some(p) => p,
                        None => {
                            tracing::info!("UDP broadcast loop exiting (input channel closed)");
                            break;
                        }
                    };

                    let wire = pkt.to_wire();

                    // Clone client data under the lock, then send without holding it.
                    // Holding the Mutex across async UDP sends would block the TCP accept loop.
                    let targets: Vec<(Arc<SessionCipher>, SocketAddr)> = {
                        let locked = clients_udp.lock().await;
                        locked.iter().map(|c| (c.cipher.clone(), c.udp_addr)).collect()
                    };

                    for (cipher, addr) in &targets {
                        match cipher.encrypt(&wire, pkt_counter) {
                            Ok(encrypted) => {
                                if let Err(e) = udp.send_to(&encrypted, *addr).await {
                                    tracing::warn!("UDP send → {} failed: {}", addr, e);
                                }
                            }
                            Err(e) => tracing::warn!("Encrypt failed: {}", e),
                        }
                    }

                    pkt_counter = pkt_counter.wrapping_add(1);
                }
            }
        }
    });

    Ok(ServerHandle { cancel, tcp_task })
}

async fn handle_client_handshake<S>(
    mut stream: S,
    addr: SocketAddr,
    expected_code: Arc<String>,
    clients: Arc<Mutex<Vec<ConnectedClient>>>,
    client_count: Arc<AtomicUsize>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Step 1: Session code
    let msg_type = stream.read_u8().await?;
    if msg_type != HS_SESSION_CODE {
        stream.write_u8(HS_REJECT).await?;
        bail!("Expected HS_SESSION_CODE, got 0x{:02X}", msg_type);
    }
    let code_len = stream.read_u8().await? as usize;
    let mut code_buf = vec![0u8; code_len];
    stream.read_exact(&mut code_buf).await?;
    let received_code = String::from_utf8(code_buf)?.trim().to_uppercase();

    if received_code != expected_code.as_str() {
        stream.write_u8(HS_REJECT).await?;
        bail!("Invalid session code from {}", addr);
    }
    tracing::info!("Session code valid — {}", addr);

    // Step 2: Client public key
    let msg_type = stream.read_u8().await?;
    if msg_type != HS_CLIENT_HELLO {
        stream.write_u8(HS_REJECT).await?;
        bail!("Expected HS_CLIENT_HELLO, got 0x{:02X}", msg_type);
    }
    let mut client_pubkey_bytes = [0u8; 32];
    stream.read_exact(&mut client_pubkey_bytes).await?;
    let client_pubkey = x25519_dalek::PublicKey::from(client_pubkey_bytes);

    // Step 3: Send server public key
    let server_kp = EphemeralKeypair::generate();
    stream.write_u8(HS_SERVER_HELLO).await?;
    stream.write_all(&server_kp.public.to_bytes()).await?;
    stream.flush().await?;

    let shared = server_kp.diffie_hellman(&client_pubkey);
    let session_key = derive_session_key(&shared, &expected_code);

    // Step 4: Nonce exchange
    let server_nonce = SessionCipher::generate_base_nonce();
    stream.write_u8(HS_NONCE_EXCHANGE).await?;
    stream.write_all(&server_nonce).await?;
    stream.flush().await?;

    let msg_type = stream.read_u8().await?;
    if msg_type != HS_NONCE_EXCHANGE {
        bail!("Expected HS_NONCE_EXCHANGE, got 0x{:02X}", msg_type);
    }
    let mut client_nonce = [0u8; 12];
    stream.read_exact(&mut client_nonce).await?;

    let combined = combine_nonces(&server_nonce, &client_nonce);
    let cipher = Arc::new(SessionCipher::new(&session_key, combined));

    // Step 5: Client UDP port
    let udp_port = stream.read_u16().await?;
    let client_udp_addr: SocketAddr = format!("{}:{}", addr.ip(), udp_port).parse()?;

    stream.write_u8(HS_ACK).await?;
    stream.flush().await?;

    tracing::info!("Client {} authenticated → UDP {}", addr, client_udp_addr);

    // Check client limit before accepting
    {
        let mut locked = clients.lock().await;
        if locked.len() >= MAX_CLIENTS {
            bail!("Maximum client limit ({}) reached", MAX_CLIENTS);
        }
        locked.push(ConnectedClient {
            addr,
            udp_addr: client_udp_addr,
            cipher,
        });
    }
    client_count.fetch_add(1, Ordering::Relaxed);

    // Keep-alive: read loop detects disconnect
    let mut buf = [0u8; 1];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => {
                tracing::info!("Client {} disconnected (TCP EOF)", addr);
                break;
            }
            Err(e) => {
                tracing::info!("Client {} TCP closed: {}", addr, e);
                break;
            }
            _ => {}
        }
    }

    client_count.fetch_sub(1, Ordering::Relaxed);
    clients.lock().await.retain(|c| c.addr != addr);
    Ok(())
}
