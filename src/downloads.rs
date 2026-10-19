use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{self, Read, Seek},
    path::Path,
    time::Duration,
};
pub enum ArchiveFormat {
    TarGz,
    TarZstd,
    Zip,
}
pub(crate) fn download(url: &str, limit: u64) -> Result<fs::File> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(60))
        .timeout_write(Duration::from_secs(60))
        .build();
    let response = agent
        .get(url)
        .set("User-Agent", "nucleon-fetcher")
        .call()
        .with_context(|| format!("Failed to download {url}"))?;
    let mut archive = tempfile::tempfile().context("Failed to create download spool")?;
    let size = io::copy(&mut response.into_reader().take(limit + 1), &mut archive)
        .with_context(|| format!("Failed to read download {url}"))?;
    ensure!(size <= limit, "Archive exceeds download size limit");
    archive.rewind()?;
    Ok(archive)
}

pub(crate) fn extract_with_limit(
    reader: impl Read + Seek,
    format: ArchiveFormat,
    directory: &Path,
    limit: u64,
) -> Result<()> {
    match format {
        ArchiveFormat::Zip => {
            let mut archive = zip::ZipArchive::new(reader)?;
            let size = (0..archive.len()).try_fold(0u64, |sum, index| -> Result<u64> {
                sum.checked_add(archive.by_index(index)?.size())
                    .context("Archive size overflow")
            })?;
            ensure!(size <= limit, "Archive exceeds extracted size limit");
            archive.extract(directory)?;
        }
        ArchiveFormat::TarGz => {
            extract_tar(flate2::read::GzDecoder::new(reader), directory, limit)?
        }
        ArchiveFormat::TarZstd => {
            extract_tar(zstd::stream::Decoder::new(reader)?, directory, limit)?
        }
    }
    Ok(())
}

pub(crate) fn extract_tar(reader: impl Read, directory: &Path, limit: u64) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    let mut size = 0u64;
    for entry in archive.entries()? {
        let mut entry = entry?;
        size = size
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        ensure!(size <= limit, "Archive exceeds extracted size limit");
        ensure!(
            entry.unpack_in(directory)?,
            "Archive contains a path outside its extraction directory"
        );
    }
    Ok(())
}

pub(crate) fn publish_new(staged: &Path, target: &Path) -> Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        staged,
        rustix::fs::CWD,
        target,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .with_context(|| format!("Publish runtime without replacing {}", target.display()))?;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    anyhow::bail!("Atomic runtime publication is unavailable on this platform");
    Ok(())
}
