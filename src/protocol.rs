use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyEvent {
    pub key: u16,
    pub client_id: String,
}

impl KeyEvent {
    pub fn to_payload(&self) -> Result<Vec<u8>, bitcode::Error> {
        // TODO: compression?
        bitcode::serialize(self)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, bitcode::Error> {
        bitcode::deserialize(slice)
    }
}
