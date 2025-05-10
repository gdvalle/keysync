use std::fs::OpenOptions;
use std::io::{self, ErrorKind};

/// Opens a file for reading and writing, creating it if it doesn't exist.
/// If the file already exists, it opens it without creating a new one.
/// Returns a tuple containing the file handle and a boolean indicating
/// whether the file was newly created or already existed.
pub fn open_or_create(path: &str) -> io::Result<(std::fs::File, bool)> {
    // Try to create it exclusively first
    match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => Ok((file, true)), // File was created
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            // File existed already; open normally
            let file = OpenOptions::new().read(true).write(true).open(path)?;
            Ok((file, false))
        }
        Err(e) => Err(e), // Other error
    }
}
