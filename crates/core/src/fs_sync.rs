use std::io;
use std::path::Path;

#[cfg(unix)]
pub fn sync_directory(path: &Path) -> io::Result<()> {
    use std::fs::OpenOptions;

    OpenOptions::new().read(true).open(path)?.sync_all()
}

#[cfg(not(unix))]
pub fn sync_directory(_path: &Path) -> io::Result<()> {
    // Windows rejects FlushFileBuffers for directory/read-only handles.
    Ok(())
}
