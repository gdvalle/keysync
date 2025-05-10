use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeySyncConfig {
    pub incoming: HashMap<String, String>,
    pub outgoing: HashMap<String, String>,
    #[serde(default)]
    pub devices: Option<Vec<String>>, // <-- Add this line
}

impl KeySyncConfig {
    /// Loads KeySyncConfig from any reader providing HJSON data.
    pub fn from_reader<R: Read>(reader: R) -> anyhow::Result<Self> {
        // Use serde_hjson::from_reader instead of serde_yaml
        let config: KeySyncConfig = serde_hjson::from_reader(reader)?;
        Ok(config)
    }

    /// Generates a string containing a default HJSON configuration.
    /// This configuration is fully commented out, serving as a template for users.
    pub fn generate_default_config_string() -> String {
        // Using raw string literals for easier multiline formatting
        r#"
{
  // This config is in HJSON format.
  // incoming defines how key presses received FROM the server
  // should be mapped on YOUR local machine.
  // Format: "REMOTE_KEY_NAME": "LOCAL_KEY_NAME"
  //
  // See https://wiki.archlinux.org/title/Keyboard_input#Identifying_scancodes for how to identify key codes.
  // Or try "evtest" (apt install evtest).
  //
  // Optional: restrict which keyboard devices to monitor.
  // Each entry can be a device path (starting with /) or a regex for the device name.
  // Examples:
  // devices: [
  //   "/dev/input/event3",
  //   "Logitech",
  //   "^My Keyboard$"
  // ],
  // If omitted, all detected keyboards will be monitored.
  // If empty, no devices will be monitored.
  // devices: [],

  incoming: {
    // Example 1: Relay the Escape key 1:1.
    // If the server sends KEY_ESC, your local machine will also process it as KEY_ESC.
    // "KEY_ESC": "KEY_ESC",

    // Example 2: Map remote KEY_F1 to local KEY_F2.
    // If the server sends KEY_F1, your local machine will interpret it as KEY_F2.
    // "KEY_F1": "KEY_F2",
  },

  // outgoing defines how key presses originating FROM your local machine
  // should be mapped BEFORE being sent TO the remote server.
  // Format: "LOCAL_KEY_NAME": "SERVER_KEY_NAME_OR_ACTION"
  //
  // If you uncomment these, they will become active mappings.
  outgoing: {
    // Example 1: Map your local Grave key (`) to the Escape key for the server.
    // If you press ` (GRAVE) on your keyboard, KEY_ESC will be sent to the server.
    // "KEY_GRAVE": "KEY_ESC",

    // Example 2: Send KEY_X as is.
    // If you press X on your keyboard, KEY_X will be sent to the server.
    // "KEY_X": "KEY_X",
  },
}
"#.trim_start().to_string() // trim_start() to remove leading newline from raw string
    }
}
