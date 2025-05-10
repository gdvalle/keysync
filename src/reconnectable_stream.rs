use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

const INITIAL_BACKOFF_MS: u64 = 50;
const MAX_BACKOFF_MS: u64 = 10_000; // 10 seconds max backoff
const CONNECTION_TIMEOUT_SECS: u64 = 5; // 5 second connection timeout

pub struct ReconnectableTcpStream {
    stream: Option<TcpStream>,
    server_addr: String,
    current_backoff: Duration,
}

impl ReconnectableTcpStream {
    pub fn new<A: ToSocketAddrs>(server_addr: A) -> Result<Self> {
        // Convert to string for storage and future reconnects
        let addr_str = match server_addr.to_socket_addrs()?.next() {
            Some(addr) => addr.to_string(),
            None => return Err(anyhow::anyhow!("Invalid server address")),
        };

        tracing::info!(server_addr = %addr_str, "Connecting to server");

        // Set a timeout for the initial connection
        let stream = TcpStream::connect_timeout(
            &addr_str.parse().context("Failed to parse server address")?,
            Duration::from_secs(CONNECTION_TIMEOUT_SECS),
        )
        .context(format!("Failed to connect to server at {}", addr_str))?;

        tracing::info!(server_addr = %addr_str, "Connected to server");

        Ok(Self {
            stream: Some(stream),
            server_addr: addr_str,
            current_backoff: Duration::from_millis(INITIAL_BACKOFF_MS),
        })
    }

    pub fn try_clone(&self) -> Result<Self> {
        let cloned_stream = match &self.stream {
            Some(stream) => Some(stream.try_clone().context("Failed to clone stream")?),
            None => None,
        };

        Ok(Self {
            stream: cloned_stream,
            server_addr: self.server_addr.clone(),
            current_backoff: self.current_backoff,
        })
    }

    fn reconnect(&mut self) -> io::Result<()> {
        // Try to reconnect with exponential backoff
        let mut attempt = 1;

        loop {
            tracing::warn!(
                attempt = attempt,
                backoff_ms = self.current_backoff.as_millis(),
                "Connection lost. Reconnecting"
            );

            thread::sleep(self.current_backoff);

            match TcpStream::connect_timeout(
                &self.server_addr.parse().unwrap(),
                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
            ) {
                Ok(stream) => {
                    tracing::info!(server_addr = %self.server_addr, "Reconnected to server successfully");
                    self.stream = Some(stream);
                    // Reset backoff on success
                    self.current_backoff = Duration::from_millis(INITIAL_BACKOFF_MS);
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!(
                        attempt = attempt,
                        error = %e,
                        "Reconnection attempt failed"
                    );
                    // Increase backoff exponentially (2x), capped at max_backoff
                    self.current_backoff = Duration::from_millis(
                        (self.current_backoff.as_millis() as u64 * 2).min(MAX_BACKOFF_MS),
                    );
                    attempt += 1;
                }
            }
        }
    }
}

impl Read for ReconnectableTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match &mut self.stream {
                Some(stream) => {
                    match stream.read(buf) {
                        Ok(0) => {
                            // Connection closed
                            self.stream = None;
                            self.reconnect()?;
                            continue;
                        }
                        Ok(n) => return Ok(n),
                        Err(e) => {
                            // Any error triggers reconnection
                            tracing::warn!("Read error: {}, attempting reconnect", e);
                            self.stream = None;
                            self.reconnect()?;
                            continue;
                        }
                    }
                }
                None => {
                    self.reconnect()?;
                    continue;
                }
            }
        }
    }
}

impl Write for ReconnectableTcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match &mut self.stream {
                Some(stream) => {
                    match stream.write(buf) {
                        Ok(n) => return Ok(n),
                        Err(e) => {
                            // Any error triggers reconnection
                            tracing::warn!("Write error: {}, attempting reconnect", e);
                            self.stream = None;
                            self.reconnect()?;
                            continue;
                        }
                    }
                }
                None => {
                    self.reconnect()?;
                    continue;
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.stream {
            Some(stream) => stream.flush(),
            None => Ok(()), // Nothing to flush if no stream
        }
    }
}
