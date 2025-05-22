use evdev::KeyCode;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::io::Read;
use std::str::FromStr;

pub type KeyCodeMap = HashMap<KeyCode, KeyCode>;

#[derive(Debug, Clone)]
pub struct KeySyncConfig {
    pub incoming: KeyCodeMap,
    pub outgoing: KeyCodeMap,
    pub devices: Option<Vec<String>>,
}

// Helper struct for raw deserialization (string keys/values)
#[derive(Deserialize)]
struct RawKeySyncConfig {
    #[serde(default)]
    incoming: HashMap<String, String>,
    #[serde(default)]
    outgoing: HashMap<String, String>,
    #[serde(default)]
    devices: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for KeySyncConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawKeySyncConfig::deserialize(deserializer)?;

        let parse_key_code_map =
            |map: HashMap<String, String>, which: &str| -> Result<KeyCodeMap, D::Error> {
                let mut result = HashMap::new();
                for (k, v) in map {
                    let key_code = KeyCode::from_str(&k);
                    let val_code = KeyCode::from_str(&v);
                    match (key_code, val_code) {
                        (Ok(kc), Ok(vc)) => {
                            result.insert(kc, vc);
                        }
                        (r1, r2) => {
                            return Err(serde::de::Error::custom(format!(
                                "invalid {} key mapping: {} -> {} ({:?} -> {:?})",
                                which, k, v, r1, r2
                            )));
                        }
                    }
                }
                Ok(result)
            };

        Ok(KeySyncConfig {
            incoming: parse_key_code_map(raw.incoming, "incoming")?,
            outgoing: parse_key_code_map(raw.outgoing, "outgoing")?,
            devices: raw.devices,
        })
    }
}

impl KeySyncConfig {
    pub fn file_name() -> &'static str {
        "config.yaml"
    }
    pub fn from_reader<R: Read>(reader: R) -> anyhow::Result<Self> {
        let config: KeySyncConfig = serde_norway::from_reader(reader)?;
        Ok(config)
    }

    // Generate a default config file with comments.
    pub fn default_config_string() -> &'static str {
        r#"
# KeySync config.
# devices: (optional) List of keyboard devices to monitor.
#   Each entry can be a device path (starting with /) or a regex for the device name.
#   If omitted (null), all detected keyboards will be monitored.
#   If empty, no devices will be monitored.
# devices:
#   - /dev/input/event3 # Path directly to the device
#   - MyKeyboard        # A substring match (regex)
#   - '^My Keyboard$'   # An anchored regex

# incoming: Maps key presses received FROM the server to your local machine.
#   Format: "REMOTE_KEY_NAME": "LOCAL_KEY_NAME"
incoming:
  # Example 1: Relay the Escape key 1:1.
  # If the server sends KEY_ESC, your local machine will also process it as KEY_ESC.
  # "KEY_ESC": "KEY_ESC"

  # Example 2: Map remote KEY_F1 to local KEY_F2.
  # If the server sends KEY_F1, your local machine will interpret it as KEY_F2.
  # "KEY_F1": "KEY_F2"

# outgoing: Maps key presses FROM your local machine to be sent TO the server.
#   Format: "LOCAL_KEY_NAME": "KEY_TO_SEND"
outgoing:
  # Example 1: Map your local Grave key (`) to the Escape key for the server.
  # If you press ` (GRAVE) on your keyboard, KEY_ESC will be sent to the server.
  # "KEY_GRAVE": "KEY_ESC"

  # Example 2: Send KEY_X as is.
  # If you press X on your keyboard, KEY_X will be sent to the server.
  # "KEY_X": "KEY_X"
"#
        .trim_start()
    }
}
