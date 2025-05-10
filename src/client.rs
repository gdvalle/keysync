use anyhow::{Context, Result};
use evdev::{uinput::VirtualDevice, KeyCode};
use rand::Rng;
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;

use crate::config::KeySyncConfig;
use crate::keyboard::{KeyEvent, KeyboardMonitor};
use crate::reconnectable_stream::ReconnectableTcpStream;

fn make_client_id() -> String {
    let username = ["SUDO_USER", "USER", "LOGNAME", "USERNAME"]
        .iter()
        .find_map(|v| match std::env::var(v) {
            Ok(value) if value != "root" => Some(value),
            _ => None,
        });
    let user_id = username
        .or_else(|| {
            hostname::get()
                .ok()
                .and_then(|hostname| hostname.to_str().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string());

    let random_int: u32 = rand::rng().random_range(0..10_000);
    format!("{}-{}", user_id, random_int)
}

fn setup_virtual_device(config: &KeySyncConfig) -> Result<VirtualDevice> {
    let mut incoming_keys: Vec<KeyCode> = vec![];
    for (_key, mapped_key) in config.incoming.iter() {
        if let Ok(key_code) = parse_key_code(mapped_key) {
            incoming_keys.push(key_code);
        } else {
            return Err(anyhow::anyhow!("Failed to parse key code: {}", mapped_key));
        }
    }

    let mut key_set = evdev::AttributeSet::<KeyCode>::new();
    for key in incoming_keys {
        key_set.insert(key);
    }

    VirtualDevice::builder()
        .context("Failed to create virtual keyboard device")?
        .name("KeySync Virtual Keyboard")
        .with_keys(&key_set)
        .context("Failed to set keys for virtual keyboard")?
        .build()
        .context("Failed to build virtual keyboard")
}

fn handle_incoming_key(
    event: &KeyEvent,
    config: &KeySyncConfig,
    virtual_keyboard: &mut VirtualDevice,
) -> Result<()> {
    let mapped_key = match config.incoming.get(&event.key) {
        Some(key) => key,
        None => return Ok(()),
    };

    tracing::info!(
        received = %event.key,
        mapped = %mapped_key,
        client_id = %event.client_id,
        "Received key event"
    );

    let key_code =
        parse_key_code(mapped_key).context(format!("Unknown key code: {}", mapped_key))?;

    press_key(virtual_keyboard, key_code).context("Failed to simulate key press")?;

    Ok(())
}

fn receive_server_messages(
    mut stream: ReconnectableTcpStream,
    config: KeySyncConfig,
) -> Result<()> {
    let mut buffer = [0; 1024];
    let mut virtual_keyboard = setup_virtual_device(&config)?;

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                tracing::warn!("Server closed the connection");
                break;
            }
            Ok(bytes_read) => {
                tracing::trace!(message = %String::from_utf8_lossy(&buffer[..bytes_read]), "Received message from server");

                match serde_json::from_slice::<KeyEvent>(&buffer[..bytes_read]) {
                    Ok(event) => {
                        if let Err(e) = handle_incoming_key(&event, &config, &mut virtual_keyboard)
                        {
                            tracing::warn!(error = %e, "Error handling incoming key");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to parse key event");
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error reading from server: {}", e));
            }
        }
    }

    Ok(())
}

fn send_key_events(mut stream: ReconnectableTcpStream, rx: mpsc::Receiver<KeyEvent>) -> Result<()> {
    for event in rx {
        let json = serde_json::to_string(&event).context("Failed to serialize key event")?;

        stream
            .write_all(json.as_bytes())
            .context("Failed to send key event to server")?;
    }

    Ok(())
}

pub fn run(server_addr: &str) -> Result<()> {
    let client_id = make_client_id();
    let config_path = "config.hjson";

    let config_file =
        match crate::utils::open_or_create(config_path).context("Failed to open config file") {
            Ok((mut file, created)) if created => {
                file.write_all(KeySyncConfig::generate_default_config_string().as_bytes())?;
                file
            }
            Ok((file, _)) => {
                tracing::info!("Config file found, using existing file");
                file
            }
            Err(e) => {
                tracing::error!("Failed to open config file: {}", e);
                return Err(e);
            }
        };

    let config = KeySyncConfig::from_reader(config_file).context("failed to parse config file")?;

    if config.incoming.is_empty() && config.outgoing.is_empty() {
        return Err(anyhow::anyhow!(
            "No key mappings found in config file, please configure"
        ));
    }

    let (tx, rx) = mpsc::channel();

    let monitor = KeyboardMonitor::new(tx, config.clone(), client_id);

    let monitor_handle = thread::spawn(move || monitor.start());

    let stream = ReconnectableTcpStream::new(server_addr)
        .context(format!("Failed to connect to server at {}", server_addr))?;

    let receive_stream = stream.try_clone().context("Failed to clone stream")?;

    let receiver_config = config.clone();
    let receiver_handle =
        thread::spawn(move || receive_server_messages(receive_stream, receiver_config));

    let sender_result = send_key_events(stream, rx);

    monitor_handle
        .join()
        .map_err(|e| anyhow::anyhow!("Error joining keyboard monitor thread: {:?}", e))??;

    receiver_handle
        .join()
        .map_err(|e| anyhow::anyhow!("Error joining receiver thread: {:?}", e))??;

    sender_result
}

fn parse_key_code(key_name: &str) -> Result<KeyCode, evdev::EnumParseError> {
    use std::str::FromStr;
    KeyCode::from_str(key_name)
}

fn press_key(device: &mut VirtualDevice, key: KeyCode) -> io::Result<()> {
    device.emit(&[*evdev::KeyEvent::new(key, 1)])?;
    device.emit(&[*evdev::KeyEvent::new(key, 0)])?;
    Ok(())
}
