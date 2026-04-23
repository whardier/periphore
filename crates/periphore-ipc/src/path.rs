use std::path::PathBuf;

use directories::ProjectDirs;

/// Resolve the platform-appropriate Unix domain socket path for Periphore.
///
/// Linux: `$XDG_RUNTIME_DIR/periphore/periphore.sock` (via `directories` crate `ProjectDirs`)
/// macOS: `$TMPDIR/periphore/periphore.sock` (`runtime_dir()` returns `None` on macOS -- no XDG)
///
/// The `directories` crate handles platform differences. On macOS, `runtime_dir()` returns
/// `None`, so we fall back to `$TMPDIR`. On Linux, `$XDG_RUNTIME_DIR` is used.
/// (Assumption A3 in RESEARCH.md)
pub fn socket_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("", "", "periphore") {
        if let Some(runtime) = dirs.runtime_dir() {
            // Linux: $XDG_RUNTIME_DIR/periphore/periphore.sock
            return runtime.join("periphore.sock");
        }
    }

    // macOS fallback: $TMPDIR/periphore/periphore.sock
    // (runtime_dir() returns None on macOS; TMPDIR is set by macOS launchd)
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    PathBuf::from(tmp).join("periphore").join("periphore.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_ends_in_periphore_sock() {
        let path = socket_path();
        assert!(
            path.to_str()
                .map_or(false, |s| s.ends_with("periphore.sock")),
            "socket path must end in periphore.sock, got: {path:?}"
        );
    }

    #[test]
    fn socket_path_parent_is_periphore_dir() {
        let path = socket_path();
        let parent = path.parent().expect("socket path must have parent");
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("periphore"),
            "socket path parent dir must be named 'periphore', got: {parent:?}"
        );
    }
}
