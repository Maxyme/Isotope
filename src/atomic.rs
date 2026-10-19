use anyhow::{Context, Result};
use std::{io::Write, path::Path};
pub fn atomic_write_file<P: AsRef<Path>, C: AsRef<[u8]>>(target_path: P, data: C) -> Result<()> {
    let target = target_path.as_ref();
    let parent = target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    #[cfg(unix)]
    let parent_directory = std::fs::File::open(parent)
        .with_context(|| format!("Failed to open parent directory {}", parent.display()))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temporary file in {}", parent.display()))?;

    temp.write_all(data.as_ref()).with_context(|| {
        format!(
            "Failed to write data to temporary file for {}",
            target.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Inherit permissions of existing file if present, or set 0o644 default
        let mode = match std::fs::metadata(target) {
            Ok(meta) => meta.permissions().mode(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0o644,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "Failed to read existing permissions for {}",
                        target.display()
                    )
                });
            }
        };
        temp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .with_context(|| {
                format!(
                    "Failed to set replacement permissions for {}",
                    target.display()
                )
            })?;
    }

    temp.as_file()
        .sync_all()
        .with_context(|| format!("Failed to sync replacement file for {}", target.display()))?;

    temp.persist(target).map_err(|e| {
        anyhow::anyhow!(
            "Failed to atomically persist to {}: {}",
            target.display(),
            e.error
        )
    })?;

    #[cfg(unix)]
    parent_directory.sync_all().with_context(|| {
        format!(
            "Replacement for {} was published, but syncing parent directory {} failed",
            target.display(),
            parent.display()
        )
    })?;

    Ok(())
}
