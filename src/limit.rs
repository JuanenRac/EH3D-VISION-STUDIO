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

/// The file to write is already there and `--replace` was not given.
pub const OUTPUT_EXISTS: &str = "file_exists";
/// The file cannot be written.
pub const UNWRITABLE: &str = "file_unwritable";

/// Writes `text` to a new file. An existing file is never replaced unless `replace` is true, and even then not when it is a link (a link could lead anywhere); the refusals start with a
/// code that does not change (`file_exists`, `file_unwritable`).
pub fn write_text(path: &Path, text: &str, replace: bool) -> Result<(), String> {
    write_bytes(path, text.as_bytes(), replace)
}

/// Writes `bytes` to a new file, with the same rules as `write_text` (a picture is written with this). A file that is replaced is written whole beside
/// itself and renamed, so that a cut or a full disk leaves the old one as it was; a new file that cannot be written whole is taken away again.
pub fn write_bytes(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    use std::io::Write;
    let shown = path.display();
    let unwritable = |e: std::io::Error| format!("{UNWRITABLE}: {shown}: {e}");
    if replace {
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("{UNWRITABLE}: {shown}: it is a link, not a file"));
            }
            // a device or a pipe cannot be replaced: it is written to as it is (a folder is refused by the system)
            Ok(m) if !m.is_file() => return std::fs::write(path, bytes).map_err(unwritable),
            Ok(m) if m.permissions().readonly() => {
                return Err(format!("{UNWRITABLE}: {shown}: the file is read only"));
            }
            Ok(m) => return replace_file(path, bytes, Some(m.permissions())).map_err(unwritable),
            Err(_) => return replace_file(path, bytes, None).map_err(unwritable),
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "{OUTPUT_EXISTS}: {shown}: the file is already there (--replace writes over it)"
                )
            } else {
                unwritable(e)
            }
        })?;
    let written = file.write_all(bytes);
    drop(file);
    written.map_err(|e| {
        let _ = std::fs::remove_file(path);
        unwritable(e)
    })
}

/// Writes `bytes` to a temporary file of its own beside `path` (a name for each process and call, only ever made new) and renames it over `path`.
fn replace_file(
    path: &Path,
    bytes: &[u8],
    permissions: Option<std::fs::Permissions>,
) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = std::path::PathBuf::from(tmp);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| match permissions {
            Some(p) => file.set_permissions(p),
            None => Ok(()),
        })
        .and_then(|()| file.sync_all());
    drop(file);
    written
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
}
