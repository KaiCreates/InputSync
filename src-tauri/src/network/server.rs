use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::core::crypto::{combine_nonces, derive_session_key, EphemeralKeypair, SessionCipher};
use crate::core::protocol::{
    HS_ACK, HS_CLIENT_HELLO, HS_NONCE_EXCHANGE, HS_REJECT, HS_SERVER_HELLO, HS_SESSION_CODE,
    InputPacket,
};

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
                    // tcp_listener is dropped here, releasing the port immediately.
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
                                let result = if let Some(acceptor) = tls {
                                    match acceptor.accept(stream).await {
                                        Ok(tls_stream) => {
                                            handle_client_handshake(tls_stream, addr, code, clients.clone()).await
                                        }
                                        Err(e) => Err(anyhow::anyhow!("TLS accept failed: {}", e)),
                                    }
                                } else {
                                    handle_client_handshake(stream, addr, code, clients.clone()).await
                                };

                                match result {
                                    Ok(()) => {
                                        count.fetch_sub(1, Ordering::Relaxed);
                                        tracing::info!("Client {} session ended", addr);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Handshake/session failed for {}: {}", addr, e);
                                    }
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
    tokio::spawn(async move {
        let mut pkt_counter: u64 = 0;
        while let Some(pkt) = input_rx.recv().await {
            let wire = pkt.to_wire();
            let locked = clients_udp.lock().await;
            for client in locked.iter() {
                match client.cipher.encrypt(&wire, pkt_counter) {
                    Ok(encrypted) => {
                        if let Err(e) = udp.send_to(&encrypted, client.udp_addr).await {
                            tracing::warn!("UDP send → {} failed: {}", client.udp_addr, e);
                        }
                    }
                    Err(e) => tracing::warn!("Encrypt failed: {}", e),
                }
            }
            pkt_counter = pkt_counter.wrapping_add(1);
        }
        tracing::info!("UDP broadcast loop exiting (input channel closed)");
    });

    Ok(ServerHandle { cancel, tcp_task })
}

async fn handle_client_handshake<S>(
    mut stream: S,
    addr: SocketAddr,
    expected_code: Arc<String>,
    clients: Arc<Mutex<Vec<ConnectedClient>>>,
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

    clients.lock().await.push(ConnectedClient {
        addr,
        udp_addr: client_udp_addr,
        cipher,
    });

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

    clients.lock().await.retain(|c| c.addr != addr);
    Ok(())
}
