use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Maximum log file size in bytes (10 MB). When exceeded the file is truncated.
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

/// A simple single-file writer that truncates on size limit.
///
/// Wraps a `Mutex<std::fs::File>` so it can be cloned cheaply and shared across
/// the tracing subscriber's internal layering.
#[derive(Clone)]
pub struct RollingFileWriter {
    inner: Arc<Mutex<fs::File>>,
    max_bytes: u64,
}

impl RollingFileWriter {
    /// Open `path` for appending, creating it (and parent dirs) if necessary.
    /// If the file already exceeds `max_bytes`, truncate it before opening.
    pub fn open(path: PathBuf, max_bytes: u64) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            validate_log_directory(parent)?;
        }

        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    anyhow::bail!("refusing unsafe log file {}", path.display());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(&path)
            .map_err(|e| anyhow::anyhow!("failed to open log file {}: {e}", path.display()))?;
        if let Some(parent) = path.parent() {
            validate_log_directory(parent)?;
        }
        validate_open_file(&path, &file)?;
        if file.metadata()?.len() >= max_bytes {
            file.set_len(0)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
            max_bytes,
        })
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?;

        // Check size before writing; truncate if needed
        if let Ok(meta) = guard.metadata() {
            if meta.len() >= self.max_bytes {
                guard.set_len(0)?;
            }
        }

        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?
            .flush()
    }
}

fn validate_open_file(path: &std::path::Path, file: &fs::File) -> anyhow::Result<()> {
    let path_metadata = fs::symlink_metadata(path)?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        anyhow::bail!("refusing unsafe log file {}", path.display());
    }
    let file_metadata = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if path_metadata.dev() != file_metadata.dev() || path_metadata.ino() != file_metadata.ino()
        {
            anyhow::bail!("log file changed while opening {}", path.display());
        }
    }
    Ok(())
}

fn validate_log_directory(path: &std::path::Path) -> anyhow::Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() {
            anyhow::bail!("refusing symlinked log directory {}", current.display());
        }
    }
    if !fs::symlink_metadata(path)?.is_dir() {
        anyhow::bail!("refusing unsafe log directory {}", path.display());
    }
    Ok(())
}

// tracing-subscriber requires `MakeWriter` — implement it so we can use this as
// a writer factory.
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RollingFileWriter {
    type Writer = RollingFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn refuses_symlinked_log_file_without_touching_target() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        let log = directory.path().join("logs").join("apprise.log");
        fs::create_dir(log.parent().unwrap()).unwrap();
        fs::write(&target, b"preserve").unwrap();
        symlink(&target, &log).unwrap();

        assert!(RollingFileWriter::open(log, 4).is_err());
        assert_eq!(fs::read(target).unwrap(), b"preserve");
    }

    #[test]
    fn refuses_symlinked_log_directory() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target-logs");
        let logs = directory.path().join("logs");
        fs::create_dir(&target).unwrap();
        symlink(&target, &logs).unwrap();

        assert!(RollingFileWriter::open(logs.join("apprise.log"), 4).is_err());
        assert!(!target.join("apprise.log").exists());
    }

    #[test]
    fn rotation_uses_open_descriptor_after_path_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        let log = directory.path().join("logs").join("apprise.log");
        fs::create_dir(log.parent().unwrap()).unwrap();
        fs::write(&target, b"preserve").unwrap();
        let mut writer = RollingFileWriter::open(log.clone(), 4).unwrap();
        writer.write_all(b"first").unwrap();

        let old_log = directory.path().join("old-log");
        fs::rename(&log, &old_log).unwrap();
        symlink(&target, &log).unwrap();
        writer.write_all(b"next").unwrap();

        assert_eq!(fs::read(target).unwrap(), b"preserve");
        assert_eq!(fs::read(old_log).unwrap(), b"next");
    }
}
