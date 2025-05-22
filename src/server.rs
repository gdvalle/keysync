use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyEvent {
    pub key: String,
    pub source: String,
    pub client_id: u32,
}

pub struct Server {
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
}

impl Server {
    pub fn new() -> Self {
        Server {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, addr: &str) -> Result<(mpsc::Sender<()>, thread::JoinHandle<Result<()>>)> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let listener =
            TcpListener::bind(addr).context(format!("Failed to bind to address: {}", addr))?;
        tracing::info!("Server listening on {}", addr);

        let clients = Arc::clone(&self.clients);
        let handle = thread::spawn(move || -> Result<()> {
            listener
                .set_nonblocking(true)
                .context("Failed to set listener to non-blocking mode")?;

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    tracing::info!("Server shutting down");
                    break;
                }

                match listener.accept() {
                    Ok((stream, addr)) => {
                        tracing::info!("Client connected: {}", addr);

                        // Add client to the map
                        {
                            let mut clients_lock = clients.lock().unwrap();
                            clients_lock.insert(
                                addr,
                                stream
                                    .try_clone()
                                    .context("Failed to clone client stream")?,
                            );
                        }

                        let clients_clone = Arc::clone(&clients);
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, clients_clone, addr) {
                                tracing::error!("Error handling client {}: {}", addr, e);
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No connection available, sleep a bit
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Error accepting connection: {}", e));
                    }
                }
            }
            Ok(())
        });

        Ok((shutdown_tx, handle))
    }
}

fn handle_client(
    mut stream: TcpStream,
    clients: Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
    addr: SocketAddr,
) -> Result<()> {
    // Add client to the clients map
    {
        let mut clients_map = clients.lock().unwrap();
        clients_map.insert(
            addr,
            stream
                .try_clone()
                .context("Failed to clone client stream")?,
        );
    }

    let mut buf = [0; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                tracing::info!("Client disconnected: {}", addr);
                // Remove client from the map
                let mut clients_map = clients.lock().unwrap();
                clients_map.remove(&addr);
                break;
            }
            Ok(size) => {
                broadcast(&buf[..size], &clients, Some(&addr))?;
            }
            Err(e) => {
                // Remove client from the map
                let mut clients_map = clients.lock().unwrap();
                clients_map.remove(&addr);
                return Err(anyhow::anyhow!("Error reading from client {}: {}", addr, e));
            }
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(payload_size = payload.len(), sender = ?_sender), err(Debug))]
fn broadcast(
    payload: &[u8],
    clients: &Arc<Mutex<HashMap<SocketAddr, TcpStream>>>,
    _sender: Option<&SocketAddr>,
) -> Result<()> {
    let clients = clients.lock().unwrap();
    tracing::debug!(clients = ?clients, client_count = clients.len(), "Broadcasting payload");
    for (addr, client) in clients.iter() {
        let span = tracing::debug_span!("write_to_client", addr = %addr);
        let _enter = span.enter();
        tracing::debug!("write");

        client
            .try_clone()
            .context(format!("Failed to clone client stream for {}", addr))?
            .write_all(payload)
            .context(format!("Error broadcasting to {}", addr))?;
    }
    Ok(())
}

pub fn run(bind_address: &str) -> Result<()> {
    let server = Server::new();
    let (_chan, handle) = server.start(bind_address)?;
    match handle.join() {
        Ok(result) => result.context("Server execution failed")?,
        Err(e) => return Err(anyhow::anyhow!("Server thread panicked: {:?}", e)),
    }
    Ok(())
}
