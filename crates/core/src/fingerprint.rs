use crate::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

pub struct PreviewFingerprint {
    hash: Sha256,
}

impl PreviewFingerprint {
    pub fn new(domain: &str) -> Self {
        let mut fingerprint = Self {
            hash: Sha256::new(),
        };
        fingerprint.add_bytes("domain", domain.as_bytes());
        fingerprint
    }

    pub fn add_bytes(&mut self, label: &str, bytes: &[u8]) {
        self.hash.update((label.len() as u64).to_le_bytes());
        self.hash.update(label.as_bytes());
        self.hash.update((bytes.len() as u64).to_le_bytes());
        self.hash.update(bytes);
    }

    pub fn add_u64(&mut self, label: &str, value: u64) {
        self.add_bytes(label, &value.to_le_bytes());
    }

    pub fn add_path(&mut self, label: &str, path: &Path) -> Result<()> {
        self.add_bytes(label, path.to_string_lossy().as_bytes());
        self.add_bytes("path-state", b"present");
        self.add_path_content(path)
    }

    pub fn add_path_if_present(&mut self, label: &str, path: &Path) -> Result<()> {
        if fs::symlink_metadata(path).is_ok() {
            self.add_path(label, path)
        } else {
            self.add_bytes(label, path.to_string_lossy().as_bytes());
            self.add_bytes("path-state", b"missing");
            Ok(())
        }
    }

    pub fn finish(self, prefix: &str) -> String {
        let digest = self.hash.finalize();
        let mut encoded = String::with_capacity(prefix.len() + 1 + digest.len() * 2);
        encoded.push_str(prefix);
        encoded.push('-');
        for byte in digest {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
        }
        encoded
    }

    fn add_path_content(&mut self, path: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            self.add_bytes("path-kind", b"symlink");
            self.add_bytes(
                "symlink-target",
                fs::read_link(path)?.to_string_lossy().as_bytes(),
            );
        } else if metadata.is_dir() {
            self.add_bytes("path-kind", b"directory");
            let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                self.add_bytes("entry-name", entry.file_name().to_string_lossy().as_bytes());
                self.add_path_content(&entry.path())?;
            }
        } else {
            self.add_bytes("path-kind", b"file");
            self.hash
                .update(("file-content".len() as u64).to_le_bytes());
            self.hash.update(b"file-content");
            self.hash.update(metadata.len().to_le_bytes());
            let mut file = fs::File::open(path)?;
            let mut buffer = [0_u8; 8192];
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                self.hash.update(&buffer[..read]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn assert_sha256_preview_id(preview_id: &str, prefix: &str) {
    let digest = preview_id
        .strip_prefix(prefix)
        .expect("preview id should use the expected prefix");
    assert_eq!(digest.len(), 64);
    assert!(digest
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn emits_sha256_and_changes_for_content_and_path_state() {
        let root = std::env::temp_dir().join(format!(
            "maa-fingerprint-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("asset.md");
        fs::write(&path, "one").unwrap();

        let mut first = PreviewFingerprint::new("import");
        first.add_path("source", &path).unwrap();
        let first = first.finish("import");
        assert_sha256_preview_id(&first, "import-");

        fs::write(&path, "two").unwrap();
        let mut second = PreviewFingerprint::new("import");
        second.add_path("source", &path).unwrap();
        assert_ne!(first, second.finish("import"));

        let mut missing = PreviewFingerprint::new("import");
        missing
            .add_path_if_present("target", &root.join("missing"))
            .unwrap();
        assert_ne!(first, missing.finish("import"));
        let _ = fs::remove_dir_all(root);
    }
}
