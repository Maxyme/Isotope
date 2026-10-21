//! Producer checks for Isotope's fixed core package, independent of any consumer.
use crate::common::{sha, write_json};
use anyhow::{Context, Result, ensure};
use object::{Architecture, Object, read::macho::LoadCommandVariant};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Entry {
    File { sha256: String, mode: u32 },
    Link { target: PathBuf },
}
fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, Entry>> {
    crate::release::files(root)?
        .into_iter()
        .map(|relative| {
            let path = root.join(&relative);
            let metadata = fs::symlink_metadata(&path)?;
            let entry = if metadata.is_symlink() {
                Entry::Link {
                    target: crate::release::checked_link(root, &path)?,
                }
            } else {
                ensure!(
                    metadata.is_file(),
                    "Unsupported package entry: {}",
                    path.display()
                );
                Entry::File {
                    sha256: sha(&path)?,
                    mode: metadata.permissions().mode() & 0o777,
                }
            };
            Ok((relative, entry))
        })
        .collect()
}

#[derive(Deserialize)]
struct CorePaths {
    loader: PathBuf,
    server: PathBuf,
    unix_dir: PathBuf,
    windows_dirs: Vec<PathBuf>,
    environment: BTreeMap<String, String>,
    emulators: Vec<Emulator>,
}
#[derive(Deserialize)]
struct Emulator {
    path: PathBuf,
    unix_library: PathBuf,
}
pub struct Runtime {
    pub root: PathBuf,
    pub loader: PathBuf,
    pub server: PathBuf,
    pub unix_dir: PathBuf,
    pub windows_dirs: Vec<PathBuf>,
    pub environment: BTreeMap<String, String>,
}
fn inspect_core(root: &Path) -> Result<Runtime> {
    let root = root.canonicalize()?;
    let text = fs::read_to_string(root.join("nucleon-runtime.toml"))?;
    // This is the producer's own core layout, not a general consumer schema validator.
    ensure!(
        toml::from_str::<toml::Value>(&text)?
            == toml::from_str::<toml::Value>(&crate::release::core_descriptor())?,
        "Packaged descriptor differs from Isotope's core layout"
    );
    let paths: CorePaths = toml::from_str(&text)?;
    let resolve = |path: &Path| -> Result<PathBuf> {
        ensure!(
            !path.as_os_str().is_empty()
                && path
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "Invalid core path"
        );
        let resolved = root.join(path).canonicalize()?;
        ensure!(resolved.starts_with(&root), "Core path escapes package");
        Ok(resolved)
    };
    let loader = resolve(&paths.loader)?;
    let server = resolve(&paths.server)?;
    let unix_dir = resolve(&paths.unix_dir)?;
    for file in [&loader, &server] {
        ensure!(
            fs::metadata(file)?.permissions().mode() & 0o111 != 0,
            "Core executable is not executable: {}",
            file.display()
        );
        require_arm64(file)?;
    }
    require_arm64(&unix_dir.join("ntdll.so"))?;
    require_arm64(&unix_dir.join("win32u.so"))?;
    let windows_dirs = paths
        .windows_dirs
        .iter()
        .map(|p| resolve(p))
        .collect::<Result<Vec<_>>>()?;
    for (dir, machine) in windows_dirs.iter().zip([
        object::pe::IMAGE_FILE_MACHINE_ARM64,
        object::pe::IMAGE_FILE_MACHINE_I386,
    ]) {
        require_pe_machine(&dir.join("ntdll.dll"), machine)?;
    }
    for (emulator, machine) in paths.emulators.iter().zip([
        object::pe::IMAGE_FILE_MACHINE_AMD64,
        object::pe::IMAGE_FILE_MACHINE_ARM64,
    ]) {
        require_pe_machine(&resolve(&emulator.path)?, machine)?;
        require_arm64(&resolve(&emulator.unix_library)?)?;
    }
    crate::fonts::validate_assets(&root)?;
    Ok(Runtime {
        root,
        loader,
        server,
        unix_dir,
        windows_dirs,
        environment: paths.environment,
    })
}
fn require_pe_machine(path: &Path, expected: u16) -> Result<()> {
    use object::{LittleEndian, read::pe::ImageNtHeaders};
    let bytes = fs::read(path)?;
    let machine = match object::File::parse(bytes.as_slice())? {
        object::File::Pe32(pe) => pe.nt_headers().file_header().machine.get(LittleEndian),
        object::File::Pe64(pe) => pe.nt_headers().file_header().machine.get(LittleEndian),
        _ => anyhow::bail!("Expected PE image: {}", path.display()),
    };
    ensure!(
        machine == expected,
        "Wrong core PE machine in {}: {machine:#x}",
        path.display()
    );
    Ok(())
}
fn require_arm64(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    ensure!(
        matches!(object::File::parse(bytes.as_slice())?, object::File::MachO64(ref image) if image.architecture() == Architecture::Aarch64),
        "Expected native ARM64 Mach-O: {}",
        path.display()
    );
    Ok(())
}
fn expand(path: &str, loader: &Path, executable: &Path) -> Option<PathBuf> {
    if let Some(suffix) = path.strip_prefix("@loader_path/") {
        Some(loader.join(suffix))
    } else if let Some(suffix) = path.strip_prefix("@executable_path/") {
        Some(executable.join(suffix))
    } else if Path::new(path).is_absolute() {
        Some(PathBuf::from(path))
    } else {
        None
    }
}
fn audit_native(
    runtime: &Runtime,
    entries: &BTreeMap<PathBuf, Entry>,
) -> Result<BTreeMap<PathBuf, Vec<String>>> {
    let mut result = BTreeMap::new();
    for (relative, entry) in entries {
        if !matches!(entry, Entry::File { .. }) {
            continue;
        }
        let path = runtime.root.join(relative);
        let mut magic = [0; 4];
        if fs::File::open(&path)?.read(&mut magic)? != 4
            || !matches!(
                u32::from_be_bytes(magic),
                0xfeedfacf | 0xcffaedfe | 0xcafebabe | 0xbebafeca | 0xcafebabf | 0xbfbafeca
            )
        {
            continue;
        }
        require_arm64(&path)?;
        let bytes = fs::read(&path)?;
        let object::File::MachO64(image) = object::File::parse(bytes.as_slice())? else {
            unreachable!()
        };
        let loader = path.parent().unwrap();
        let executable = runtime.loader.parent().unwrap();
        let mut search = vec![runtime.unix_dir.clone(), runtime.root.join("lib")];
        let mut dependencies = Vec::new();
        let mut commands = image.macho_load_commands()?;
        while let Some(command) = commands.next()? {
            if let LoadCommandVariant::Rpath(rpath) = command.variant()? {
                let text = std::str::from_utf8(command.string(image.endian(), rpath.path)?)?;
                let path = expand(text, loader, executable).context("Unsupported native rpath")?;
                // A build-tree path is never part of a relocatable core package.
                ensure!(
                    path.canonicalize()
                        .is_ok_and(|p| p.starts_with(&runtime.root)),
                    "External or missing native rpath: {text}"
                );
                search.push(path);
            }
            if command.cmd() != object::macho::LC_ID_DYLIB
                && let Some(dylib) = command.dylib()?
            {
                dependencies.push(
                    std::str::from_utf8(command.string(image.endian(), dylib.dylib.name)?)?
                        .to_owned(),
                );
            }
        }
        for name in &dependencies {
            if name.starts_with("/usr/lib/") || name.starts_with("/System/Library/") {
                continue;
            }
            let candidates = if let Some(suffix) = name.strip_prefix("@rpath/") {
                search.iter().map(|p| p.join(suffix)).collect::<Vec<_>>()
            } else {
                expand(name, loader, executable).into_iter().collect()
            };
            ensure!(
                candidates.iter().any(|p| p
                    .canonicalize()
                    .is_ok_and(|p| p.starts_with(&runtime.root) && require_arm64(&p).is_ok())),
                "Missing or external native dependency: {} needs {name}",
                relative.display()
            );
        }
        result.insert(relative.clone(), dependencies);
    }
    Ok(result)
}

pub fn relocate(archive: &Path, expected: &Path, target: &Path, logs: &Path) -> Result<Runtime> {
    let expected = inventory(expected)?;
    write_json(
        &logs.join("archive-inventory.json"),
        &serde_json::json!({"schema":1,"archive_sha256":sha(archive)?,"files":expected}),
    )?;
    ensure!(!target.exists(), "Relocation output must be new");
    crate::artifacts::unpack(archive, target, 8 * 1024 * 1024 * 1024)?;
    let actual = inventory(target)?;
    ensure!(
        actual == expected,
        "Relocated archive differs from staged files, permissions or links"
    );
    let runtime = inspect_core(target)?;
    let native = audit_native(&runtime, &actual)?;
    write_json(&logs.join("native-dependencies.json"), &native)?;
    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_inventory_detects_corruption_permissions_and_escaping_links() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let file = root.join("file");
        fs::write(&file, b"original").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        let expected = inventory(&root).unwrap();
        let archive = temp.path().join("runtime.tar.gz");
        crate::release::archive(
            &archive,
            &root,
            &crate::release::files(&root).unwrap(),
            "",
            0,
        )
        .unwrap();
        let relocated = temp.path().join("relocated");
        crate::artifacts::unpack(&archive, &relocated, 1024).unwrap();
        assert_eq!(expected, inventory(&relocated).unwrap());
        fs::write(&file, b"corrupted").unwrap();
        assert_ne!(expected, inventory(&root).unwrap());
        fs::write(&file, b"original").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        assert_ne!(expected, inventory(&root).unwrap());
        std::os::unix::fs::symlink("../../runtime.tar.gz", root.join("escape")).unwrap();
        assert!(inventory(&root).is_err());
    }
    fn macho(dependency: &str) -> Vec<u8> {
        let len = (24 + dependency.len() + 1 + 7) & !7;
        let mut bytes = vec![0; 32 + len];
        for (offset, value) in [
            (0, 0xfeedfacfu32),
            (4, 0x0100000c),
            (12, 6),
            (16, 1),
            (20, len as u32),
            (32, 0xc),
            (36, len as u32),
            (40, 24),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[56..56 + dependency.len()].copy_from_slice(dependency.as_bytes());
        bytes
    }
    #[test]
    fn dependency_audit_rejects_external_missing_and_wrong_architecture_libraries() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join("bin")).unwrap();
        fs::create_dir(root.join("lib")).unwrap();
        let loader = root.join("bin/wine");
        let runtime = Runtime {
            root: root.clone(),
            loader: loader.clone(),
            server: loader.clone(),
            unix_dir: root.join("lib"),
            windows_dirs: vec![],
            environment: BTreeMap::new(),
        };
        for dependency in [
            "/opt/homebrew/lib/libexternal.dylib",
            "@rpath/missing.dylib",
            "@loader_path/../../outside.dylib",
        ] {
            fs::write(&loader, macho(dependency)).unwrap();
            assert!(audit_native(&runtime, &inventory(&root).unwrap()).is_err());
        }
        fs::write(&loader, macho("/usr/lib/libSystem.B.dylib")).unwrap();
        assert!(audit_native(&runtime, &inventory(&root).unwrap()).is_ok());
        let library = root.join("lib/bundled.dylib");
        fs::write(&library, macho("/usr/lib/libSystem.B.dylib")).unwrap();
        fs::write(&loader, macho("@rpath/bundled.dylib")).unwrap();
        assert!(audit_native(&runtime, &inventory(&root).unwrap()).is_ok());
        let mut wrong = macho("/usr/lib/libSystem.B.dylib");
        wrong[4..8].copy_from_slice(&0x01000007u32.to_le_bytes());
        fs::write(&library, wrong).unwrap();
        assert!(audit_native(&runtime, &inventory(&root).unwrap()).is_err());
    }
}
