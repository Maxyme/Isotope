//! Self-contained native font dependency and runtime font inventory.
use crate::{
    build::Build,
    common::{copy, fetch_archive, hash_files, key, output, p, repo, run, sha, write_json},
};
use anyhow::{Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const LIBRARY: &str = "lib/wine/aarch64-unix/libfreetype.6.dylib";
pub const FONT: &str = "share/wine/fonts/tahoma.ttf";

pub fn prepare(build: &Build) -> Result<PathBuf> {
    let pin = &build.recipe.freetype;
    let archive = fetch_archive(&build.workspace, pin)?;
    let source_root = build.workspace.join("dependencies").join(&pin.sha256);
    let source = source_root.join(
        pin.directory
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing FreeType archive directory"))?,
    );
    if !source.exists() {
        crate::artifacts::unpack(&archive, &source_root, 256 * 1024 * 1024)?;
    }
    let identity = key(&(
        pin,
        output(
            "/usr/bin/clang",
            &["--version".into()],
            Some(&build.env),
            None,
        )?,
        output(
            "xcrun",
            &["--show-sdk-version".into()],
            Some(&build.env),
            None,
        )?,
    ))?;
    let prefix = build
        .workspace
        .join("dependencies")
        .join(format!("freetype-{identity}"));
    let args = vec![
        "-S".into(),
        p(&source),
        "-B".into(),
        p(prefix.join("build")),
        "-G".into(),
        "Ninja".into(),
        "-DCMAKE_BUILD_TYPE=Release".into(),
        "-DBUILD_SHARED_LIBS=ON".into(),
        "-DCMAKE_C_COMPILER=/usr/bin/clang".into(),
        "-DCMAKE_OSX_ARCHITECTURES=arm64".into(),
        "-DCMAKE_OSX_DEPLOYMENT_TARGET=11.0".into(),
        "-DCMAKE_INSTALL_NAME_DIR=@rpath".into(),
        format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()),
        "-DFT_DISABLE_ZLIB=ON".into(),
        "-DFT_DISABLE_BZIP2=ON".into(),
        "-DFT_DISABLE_PNG=ON".into(),
        "-DFT_DISABLE_HARFBUZZ=ON".into(),
        "-DFT_DISABLE_BROTLI=ON".into(),
        "-DFT_DISABLE_HVF=ON".into(),
    ];
    run(
        "cmake",
        &args,
        Some(&build.env),
        None,
        Some(&build.logs.join("freetype-configure.log")),
        120,
        0,
    )?;
    run(
        "cmake",
        &[
            "--build".into(),
            p(prefix.join("build")),
            "--parallel".into(),
            build.jobs.to_string(),
        ],
        Some(&build.env),
        None,
        Some(&build.logs.join("freetype-build.log")),
        600,
        0,
    )?;
    run(
        "cmake",
        &["--install".into(), p(prefix.join("build"))],
        Some(&build.env),
        None,
        Some(&build.logs.join("freetype-install.log")),
        60,
        0,
    )?;
    Ok(prefix)
}

pub fn stage(build: &Build, prefix: &Path) -> Result<()> {
    let runtime = build.work.join("runtime");
    let library = runtime.join(LIBRARY);
    copy(&prefix.join("lib/libfreetype.6.dylib"), &library)?;
    run(
        "install_name_tool",
        &[
            "-id".into(),
            "@rpath/libfreetype.6.dylib".into(),
            p(&library),
        ],
        None,
        None,
        None,
        30,
        0,
    )?;
    run(
        "codesign",
        &["--force".into(), "--sign".into(), "-".into(), p(&library)],
        None,
        None,
        None,
        30,
        0,
    )?;
    ensure!(
        output("lipo", &["-archs".into(), p(&library)], None, None)? == "arm64",
        "FreeType must be native ARM64"
    );
    let dependencies = output("otool", &["-L".into(), p(&library)], None, None)?;
    ensure!(
        dependencies.lines().skip(1).all(|l| {
            let name = l.split_whitespace().next().unwrap_or_default();
            name == "@rpath/libfreetype.6.dylib"
                || name.starts_with("/usr/lib/")
                || name.starts_with("/System/Library/")
        }),
        "FreeType depends on an unbundled library: {dependencies}"
    );
    for directory in [
        build.wine.join("fonts"),
        build.work.join("wine-build/fonts"),
    ] {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.extension().is_some_and(|s| s == "ttf" || s == "fon") && path.is_file() {
                copy(
                    &path,
                    &runtime
                        .join("share/wine/fonts")
                        .join(path.file_name().unwrap()),
                )?;
            }
        }
    }
    let archive = fetch_archive(&build.workspace, &build.recipe.freetype)?;
    let source = build
        .workspace
        .join("dependencies")
        .join(&build.recipe.freetype.sha256)
        .join(build.recipe.freetype.directory.as_ref().unwrap());
    for name in ["LICENSE.TXT", "docs/FTL.TXT", "docs/GPLv2.TXT"] {
        copy(
            &source.join(name),
            &runtime
                .join("share/licenses/freetype")
                .join(Path::new(name).file_name().unwrap()),
        )?;
    }
    write_json(
        &runtime.join("share/nucleon-build/freetype.json"),
        &serde_json::json!({"source":build.recipe.freetype,"archive_sha256":sha(&archive)?,"files":hash_files(&runtime,&[LIBRARY.into(),FONT.into()])?,"builder":sha(&repo().join("src/fonts.rs"))?}),
    )?;
    validate_assets(&runtime)
}

pub fn validate_assets(runtime: &Path) -> Result<()> {
    for name in [
        LIBRARY,
        FONT,
        "share/wine/fonts/tahomabd.ttf",
        "share/licenses/freetype/FTL.TXT",
    ] {
        ensure!(
            runtime.join(name).is_file(),
            "Missing runtime font support: {name}; rebuild Wine"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_inventory_requires_library_fonts_and_license() {
        let temp = tempfile::tempdir().unwrap();
        assert!(validate_assets(temp.path()).is_err());
        for name in [
            LIBRARY,
            FONT,
            "share/wine/fonts/tahomabd.ttf",
            "share/licenses/freetype/FTL.TXT",
        ] {
            let p = temp.path().join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, b"fixture").unwrap();
        }
        validate_assets(temp.path()).unwrap();
        fs::remove_file(temp.path().join(FONT)).unwrap();
        assert!(validate_assets(temp.path()).is_err());
    }
}
