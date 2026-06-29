use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(path)
}

pub fn validate_single_path_component(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.trim() != value
        || matches!(value, "." | "..")
        || value.contains(['/', '\\', ':', '\0'])
        || value.chars().any(char::is_control)
        || Path::new(value).components().count() != 1
    {
        return Err(format!(
            "Invalid {label} '{value}': expected one safe path component."
        ));
    }
    Ok(())
}

pub fn guard_write_path(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let root = normalize_absolute(root)?;
    let candidate = normalize_absolute(candidate)?;
    if candidate == root || !candidate.starts_with(&root) {
        return Err(permission_denied(format!(
            "Write path must stay below allowed root {}: {}",
            display_path(&root),
            display_path(&candidate)
        )));
    }
    reject_symlink_components(&root, candidate.parent().unwrap_or(&candidate))?;
    Ok(candidate)
}

pub fn guard_existing_path(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let root = normalize_absolute(root)?;
    let candidate = guard_write_path(&root, candidate)?;
    reject_symlink_components(&root, &candidate)?;
    if fs::symlink_metadata(&candidate).is_err() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {}", display_path(&candidate)),
        ));
    }
    Ok(candidate)
}

pub fn path_is_within(root: &Path, candidate: &Path) -> io::Result<bool> {
    let root = normalize_absolute(root)?;
    let candidate = normalize_absolute(candidate)?;
    Ok(candidate == root || candidate.starts_with(root))
}

fn normalize_absolute(path: &Path) -> io::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(permission_denied(format!(
            "Safety-sensitive path must be absolute: {}",
            display_path(path)
        )));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(permission_denied(format!(
                    "ParentDir is forbidden in safety-sensitive paths: {}",
                    display_path(path)
                )));
            }
        }
    }
    Ok(normalized)
}

fn reject_symlink_components(root: &Path, candidate: &Path) -> io::Result<()> {
    if !candidate.starts_with(root) {
        return Err(permission_denied(format!(
            "Path must stay under allowed root {}: {}",
            display_path(root),
            display_path(candidate)
        )));
    }
    if fs::symlink_metadata(root)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(permission_denied(format!(
            "Allowed root must not be a symlink: {}",
            display_path(root)
        )));
    }

    let mut current = root.to_path_buf();
    for component in candidate
        .strip_prefix(root)
        .map_err(|_| permission_denied("Path escaped allowed root.".into()))?
        .components()
    {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(permission_denied(format!(
                    "Symlink traversal is forbidden for safety-sensitive path: {}",
                    display_path(&current)
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn permission_denied(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_components_and_root_writes() {
        let root = std::env::temp_dir().join("maa-path-safety-root");
        assert!(guard_write_path(&root, &root).is_err());
        assert!(guard_write_path(&root, &root.join("../escape")).is_err());
    }

    #[test]
    fn validates_single_components() {
        assert!(validate_single_path_component("review", "asset").is_ok());
        for invalid in ["", ".", "..", "a/b", "a\\b", "C:"] {
            assert!(validate_single_path_component(invalid, "asset").is_err());
        }
    }
}
