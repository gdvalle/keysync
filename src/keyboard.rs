use anyhow::{Context, Result};
use evdev::{Device, KeyCode};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::config::{KeyCodeMap, KeySyncConfig};
use crate::protocol::KeyEvent;

pub struct KeyboardMonitor {
    config: KeySyncConfig,
    sender: mpsc::Sender<KeyEvent>,
    client_id: String,
}

impl KeyboardMonitor {
    pub fn new(sender: mpsc::Sender<KeyEvent>, config: KeySyncConfig, client_id: String) -> Self {
        KeyboardMonitor {
            config,
            sender,
            client_id,
        }
    }

    pub fn find_keyboards(&self) -> Result<Vec<Device>> {
        let selectors = if let Some(devices) = self.config.devices.as_ref() {
            let mut selectors = Vec::new();
            for entry in devices {
                if entry.starts_with('/') {
                    selectors.push(DeviceSelector::Path(entry.clone()));
                } else {
                    let re = Regex::new(entry)
                        .or_else(|_| Regex::new(&regex::escape(entry)))
                        .map_err(|e| anyhow::anyhow!("Invalid regex '{}': {}", entry, e))?;
                    selectors.push(DeviceSelector::Regex(re));
                }
            }
            selectors
        } else {
            Vec::new()
        };

        let mut devices = Vec::new();
        let input_path = Path::new("/dev/input");
        let entries = fs::read_dir(input_path).context("Failed to read input directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if !Self::is_event_device(&path) {
                continue;
            }

            match Self::try_open_keyboard_device(&path) {
                Some(device) => {
                    if !selectors.is_empty() {
                        let name = device.name();
                        let matched = selectors.iter().any(|sel| sel.matches(&path, name));
                        if !matched {
                            continue;
                        }
                    }
                    devices.push(device)
                }
                None => continue,
            }
        }

        Ok(devices)
    }

    fn is_event_device(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("event"))
            .unwrap_or(false)
    }

    fn try_open_keyboard_device(path: &Path) -> Option<Device> {
        match Device::open(path) {
            Ok(device) => {
                if Self::is_keyboard_device(&device) {
                    Self::log_keyboard_device(&device, path);
                    Some(device)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "Could not open device");
                None
            }
        }
    }

    fn log_keyboard_device(device: &Device, path: &Path) {
        tracing::info!(path = ?path, name = device.name(), physical = device.physical_path(), "Found keyboard device");
    }

    fn is_keyboard_device(device: &Device) -> bool {
        let valid_keys = [
            KeyCode::KEY_A,
            KeyCode::KEY_SPACE,
            KeyCode::KEY_ENTER,
            KeyCode::BTN_SIDE,
            KeyCode::BTN_EXTRA,
        ];
        for key in valid_keys.iter() {
            if device
                .supported_keys()
                .map(|keys| keys.contains(*key))
                .unwrap_or(false)
            {
                return true;
            }
        }

        if let Some(name) = device.name() {
            let name = name.to_lowercase();
            if name.contains("keyboard") || name.contains("kbd") {
                return true;
            }
        }

        false
    }

    fn process_key_event(
        outgoing_map: &KeyCodeMap,
        event: evdev::InputEvent,
        sender: &mpsc::Sender<KeyEvent>,
        client_id: &str,
    ) {
        if event.event_type() != evdev::EventType::KEY || event.value() != 1 {
            return;
        }

        let key = evdev::KeyCode::new(event.code());

        let mapped_key = match outgoing_map.get(&key) {
            Some(mapped_key) => {
                tracing::info!(original = ?key, mapped = ?mapped_key, "Key pressed and mapped");
                *mapped_key
            }
            None => return,
        };

        let key_event = KeyEvent {
            key: mapped_key.0,
            client_id: client_id.to_string(),
        };

        if let Err(e) = sender.send(key_event) {
            tracing::error!(error = %e, "Error sending key event");
        }
    }

    fn monitor_keyboard(
        outgoing_map: &KeyCodeMap,
        device: &mut Device,
        sender: &mpsc::Sender<KeyEvent>,
        client_id: String,
    ) -> Result<()> {
        tracing::info!(name = device.name(), "Monitoring keyboard");

        loop {
            for event in device
                .fetch_events()
                .context("Failed to fetch events from keyboard device")?
            {
                Self::process_key_event(outgoing_map, event, sender, &client_id);
            }

            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    pub fn start(&self) -> Result<()> {
        let keyboards = self.find_keyboards()?;

        if keyboards.is_empty() {
            return Err(anyhow::anyhow!("No keyboards found!"));
        }

        tracing::info!(count = keyboards.len(), "Found keyboards");

        self.start_keyboard_monitors(keyboards)
    }

    fn start_keyboard_monitors(&self, keyboards: Vec<Device>) -> Result<()> {
        let mut handles = Vec::new();

        for (i, mut keyboard) in keyboards.into_iter().enumerate() {
            let sender = self.sender.clone();
            let outgoing_map = self.config.outgoing.clone();
            let client_id = self.client_id.clone();

            let handle = thread::spawn(move || -> Result<()> {
                tracing::info!(
                    index = i,
                    name = keyboard.name(),
                    "Started monitoring keyboard"
                );

                Self::monitor_keyboard(&outgoing_map, &mut keyboard, &sender, client_id)
            });

            handles.push((i, handle));
        }

        for (i, handle) in handles {
            handle
                .join()
                .map_err(|e| anyhow::anyhow!("Error joining keyboard thread {}: {:?}", i, e))??;
        }

        Ok(())
    }
}

enum DeviceSelector {
    Path(String),
    Regex(Regex),
}

impl DeviceSelector {
    fn matches(&self, device_path: &Path, device_name: Option<&str>) -> bool {
        match self {
            DeviceSelector::Path(p) => device_path == Path::new(p),
            DeviceSelector::Regex(re) => {
                if let Some(name) = device_name {
                    re.is_match(name)
                } else {
                    false
                }
            }
        }
    }
}
