//! Producer-side layout checks. Consumer policy is checked explicitly, after assembly.
use anyhow::{Context, Result, ensure};
use object::{
    Architecture, FileKind, Object,
    read::macho::{FatArch, MachOFatFile32, MachOFatFile64},
};
use serde::Deserialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Default, Deserialize)]
pub struct Graphics {
    pub dxmt: Option<PathBuf>,
    pub dxvk: Option<PathBuf>,
    pub vkd3d: Option<PathBuf>,
    pub vulkan_icd: Option<PathBuf>,
    pub vulkan_loader: Option<PathBuf>,
    pub frontend_manifest: Option<PathBuf>,
}
#[derive(Deserialize)]
struct Emulator {
    guest: String,
    path: PathBuf,
    unix_library: PathBuf,
}
#[derive(Deserialize)]
struct Layout {
    schema: u32,
    mode: String,
    guests: Vec<String>,
    loader: PathBuf,
    server: PathBuf,
    unix_dir: PathBuf,
    windows_dirs: Vec<PathBuf>,
    emulators: Vec<Emulator>,
    #[serde(default)]
    graphics: Graphics,
}
pub struct Runtime {
    pub root: PathBuf,
    pub loader: PathBuf,
    pub server: PathBuf,
    pub unix_dir: PathBuf,
    pub windows_dirs: Vec<PathBuf>,
    pub graphics: Graphics,
}
fn resolve(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure!(
        !relative.as_os_str().is_empty()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "Invalid runtime-relative path: {}",
        relative.display()
    );
    let path = root
        .join(relative)
        .canonicalize()
        .with_context(|| format!("Missing runtime component {}", relative.display()))?;
    ensure!(
        path.starts_with(root),
        "Runtime component escapes root: {}",
        relative.display()
    );
    Ok(path)
}
fn native_image(data: &[u8]) -> Result<()> {
    let valid = |image: &[u8]| {
        object::File::parse(image).is_ok_and(|f| {
            f.format() == object::BinaryFormat::MachO && f.architecture() == Architecture::Aarch64
        })
    };
    let found = match FileKind::parse(data)? {
        FileKind::MachOFat32 => MachOFatFile32::parse(data)?
            .arches()
            .iter()
            .any(|a| a.data(data).is_ok_and(valid)),
        FileKind::MachOFat64 => MachOFatFile64::parse(data)?
            .arches()
            .iter()
            .any(|a| a.data(data).is_ok_and(valid)),
        _ => valid(data),
    };
    ensure!(found, "Expected an ARM64 Mach-O image");
    Ok(())
}
fn native(path: &Path) -> Result<()> {
    native_image(&fs::read(path)?)
        .with_context(|| format!("Native ABI mismatch: {}", path.display()))
}
fn pe(path: &Path, allowed: &[u16]) -> Result<()> {
    use object::{LittleEndian, read::pe::ImageNtHeaders};
    let data = fs::read(path)?;
    let machine = match object::File::parse(data.as_slice())? {
        object::File::Pe32(f) => f.nt_headers().file_header().machine.get(LittleEndian),
        object::File::Pe64(f) => f.nt_headers().file_header().machine.get(LittleEndian),
        _ => anyhow::bail!("Expected a Windows image: {}", path.display()),
    };
    ensure!(
        allowed.contains(&machine),
        "Guest ABI mismatch: {}",
        path.display()
    );
    Ok(())
}
impl Runtime {
    pub fn inspect(root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        let d: Layout = toml::from_str(&fs::read_to_string(root.join("nucleon-runtime.toml"))?)?;
        ensure!(
            d.schema == 1 && d.mode == "arm64-fex",
            "Unsupported Isotope runtime layout"
        );
        ensure!(
            !d.guests.is_empty() && d.guests.iter().all(|g| matches!(g.as_str(), "x64" | "x86")),
            "Unsupported guest ABI"
        );
        let loader = resolve(&root, &d.loader)?;
        let server = resolve(&root, &d.server)?;
        for path in [&loader, &server] {
            use std::os::unix::fs::PermissionsExt;
            ensure!(
                path.metadata()?.permissions().mode() & 0o111 != 0,
                "Runtime executable is not executable"
            );
            native(path)?;
        }
        let unix_dir = resolve(&root, &d.unix_dir)?;
        ensure!(unix_dir.is_dir(), "Unix library path must be a directory");
        native(&unix_dir.join("ntdll.so"))?;
        let windows_dirs = d
            .windows_dirs
            .iter()
            .map(|p| resolve(&root, p))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            !windows_dirs.is_empty() && windows_dirs.iter().all(|p| p.is_dir()),
            "Windows library paths must be directories"
        );
        for guest in &d.guests {
            let allowed: &[u16] = if guest == "x86" {
                &[0x14c]
            } else {
                &[0xaa64, 0xa641, 0xa64e]
            };
            ensure!(
                windows_dirs
                    .iter()
                    .any(|p| pe(&p.join("ntdll.dll"), allowed).is_ok()),
                "Missing Windows ntdll for {guest}"
            );
            let module = d
                .emulators
                .iter()
                .find(|e| &e.guest == guest)
                .context("Missing guest emulator pair")?;
            pe(
                &resolve(&root, &module.path)?,
                if guest == "x86" {
                    &[0xaa64]
                } else {
                    &[0x8664, 0xa641]
                },
            )?;
            native(&resolve(&root, &module.unix_library)?)?;
        }
        let mut graphics = d.graphics;
        for path in [&mut graphics.dxmt, &mut graphics.dxvk, &mut graphics.vkd3d]
            .into_iter()
            .flatten()
        {
            *path = resolve(&root, path)?;
            ensure!(path.is_dir(), "Graphics bundle must be a directory");
        }
        if let Some(path) = &mut graphics.vulkan_loader {
            *path = resolve(&root, path)?;
            native(path)?;
        }
        if let Some(path) = &mut graphics.vulkan_icd {
            *path = resolve(&root, path)?;
            let icd: serde_json::Value = serde_json::from_slice(&fs::read(&*path)?)?;
            let library = icd["ICD"]["library_path"]
                .as_str()
                .context("Vulkan ICD has no library path")?;
            ensure!(
                !Path::new(library).is_absolute(),
                "Vulkan library path must be relative"
            );
            let library = path.parent().unwrap().join(library).canonicalize()?;
            ensure!(library.starts_with(&root), "Vulkan driver escapes runtime");
            native(&library)?;
        }
        if let Some(path) = &mut graphics.frontend_manifest {
            *path = resolve(&root, path)?;
            let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&*path)?)?;
            ensure!(
                manifest["schema"] == 1 && manifest["frontend_abi"] == "arm64ec",
                "Unsupported frontend pairing manifest"
            );
            for (relative, digest) in manifest["files"]
                .as_object()
                .context("Frontend manifest has no files")?
            {
                ensure!(
                    crate::common::sha(&resolve(&root, Path::new(relative))?)?
                        == digest.as_str().context("Invalid pairing digest")?,
                    "Frontend file changed: {relative}"
                );
            }
        }
        Ok(Self {
            root,
            loader,
            server,
            unix_dir,
            windows_dirs,
            graphics,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn producer_paths_cannot_escape_through_traversal_or_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("runtime");
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let outside = dir.path().join("outside");
        fs::write(&outside, b"keep").unwrap();
        assert!(resolve(&root, Path::new("../outside")).is_err());
        std::os::unix::fs::symlink(&outside, root.join("escaped")).unwrap();
        assert!(resolve(&root, Path::new("escaped")).is_err());
        fs::write(root.join("inside"), b"ok").unwrap();
        assert!(resolve(&root, Path::new("inside")).is_ok());
    }
    #[test]
    fn native_validation_rejects_an_intel_only_image() {
        let mut bytes = vec![0u8; 32];
        bytes[..4].copy_from_slice(&0xfeedfacfu32.to_le_bytes());
        bytes[4..8].copy_from_slice(&0x01000007u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&6u32.to_le_bytes());
        assert!(native_image(&bytes).is_err());
        bytes[4..8].copy_from_slice(&0x0100000cu32.to_le_bytes());
        assert!(native_image(&bytes).is_ok());
    }
}
