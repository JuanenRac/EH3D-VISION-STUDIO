//! Reading a file with a limit of size: the command line reads a picture or a file of points only when it is not larger than `max_file_mib` of the configuration file, so that a
//! file by mistake does not fill the memory. Every refusal starts with a code that does not change (`file_too_large`, `file_unreadable`, `file_not_text`).

use std::io::Read;
use std::path::Path;

/// The file is larger than the limit.
pub const TOO_LARGE: &str = "file_too_large";
/// The file cannot be opened or read.
pub const UNREADABLE: &str = "file_unreadable";
/// The file is not text.
pub const NOT_TEXT: &str = "file_not_text";

/// The bytes of a file that is not larger than `max_mib` MiB. At most one byte more than the limit is ever read, also when the file grows while it is being read.
pub fn read_limited(path: &Path, max_mib: u64) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let limit = max_mib.saturating_mul(1_048_576);
    let file = std::fs::File::open(path).map_err(|e| format!("{UNREADABLE}: {shown}: {e}"))?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{UNREADABLE}: {shown}: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{TOO_LARGE}: {shown}: the file is larger than {max_mib} MiB (max_file_mib in vision.cfg)"
        ));
    }
    Ok(bytes)
}

/// The text of a file that is not larger than `max_mib` MiB.
pub fn read_text_limited(path: &Path, max_mib: u64) -> Result<String, String> {
    let bytes = read_limited(path, max_mib)?;
    String::from_utf8(bytes)
        .map_err(|_| format!("{NOT_TEXT}: {}: the file is not text", path.display()))
}
