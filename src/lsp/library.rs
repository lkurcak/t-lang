//! Editor-readable copies of the sources built into this binary. Ordinary
//! `file:` URIs work with any LSP client; no virtual-document extension is
//! needed. A private directory per server avoids reusing another build's
//! sources, even when the compiler's package version hasn't changed.

use crate::load::LIBRARIES;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct LibrarySources {
    root: PathBuf,
}

impl LibrarySources {
    pub(super) fn new() -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut attempt = 0u64;
        let root = loop {
            let root = std::env::temp_dir().join(format!(
                "t-lsp-sources-{}-{nonce}-{attempt}",
                std::process::id()
            ));
            // The system temporary directory can be shared by other users.
            #[cfg(unix)]
            let result = {
                use std::os::unix::fs::DirBuilderExt;
                fs::DirBuilder::new().mode(0o700).create(&root)
            };
            #[cfg(not(unix))]
            let result = fs::create_dir(&root);
            match result {
                Ok(()) => break root,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => attempt += 1,
                Err(e) => return Err(e),
            }
        };
        for (name, files, _) in LIBRARIES {
            let dir = root.join(name);
            fs::create_dir(&dir)?;
            for (file, text) in files {
                let path = dir.join(file);
                fs::write(&path, text.as_bytes())?;
                let mut permissions = fs::metadata(&path)?.permissions();
                permissions.set_readonly(true);
                fs::set_permissions(path, permissions)?;
            }
        }
        // Keep the files after shutdown: an editor may still have a link
        // or buffer open. The operating system can clear its temp directory.
        Ok(Self { root })
    }

    /// The on-disk copy of a compiler source path such as `<core>/option.t`.
    pub(super) fn path(&self, path: &Path) -> Option<PathBuf> {
        LIBRARIES.iter().find_map(|(name, files, dir)| {
            let file = path.strip_prefix(dir).ok()?;
            files
                .iter()
                .any(|(f, _)| file == Path::new(f))
                .then(|| self.root.join(name).join(file))
        })
    }

    /// Recognize only this server's exported files, never a program's files
    /// with similar names. Their semantic text always comes from the binary.
    pub(super) fn source(&self, path: &Path) -> Option<&'static str> {
        let path = path.strip_prefix(&self.root).ok()?;
        LIBRARIES.iter().find_map(|(name, files, _)| {
            let file = path.strip_prefix(name).ok()?;
            files
                .iter()
                .find(|(f, _)| file == Path::new(f))
                .map(|(_, text)| *text)
        })
    }
}
