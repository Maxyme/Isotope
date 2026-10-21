//! Focused standalone runtime regressions; no consumer or installed game required.
use crate::{
    build::{Build, marker_run},
    common::{Environment, command, p, repo, run, set, sha, write_json},
    process::{OwnedChild, TERMINATION_GRACE},
};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn environment(build: &Build, runtime: &Path, prefix: &Path) -> Environment {
    let mut env = build.env.clone();
    for (name, value) in [
        ("WINEPREFIX", p(prefix)),
        ("WINELOADER", p(runtime.join("bin/wine"))),
        ("WINESERVER", p(runtime.join("bin/wineserver"))),
        (
            "WINEDLLPATH",
            format!(
                "{}:{}",
                runtime.join("lib/wine/aarch64-windows").display(),
                runtime.join("lib/wine/i386-windows").display()
            ),
        ),
        (
            "DYLD_LIBRARY_PATH",
            p(runtime.join("lib/wine/aarch64-unix")),
        ),
        ("FEX_TSOENABLED", "1".into()),
        ("NUCLEON_EMULATE_KUSD", "1".into()),
        ("ISOTOPE_SUPERVISOR_PID", std::process::id().to_string()),
        ("ISOTOPE_FAULT_DIAGNOSTICS", "1".into()),
        ("WINEDEBUG", "-all,warn+all,err+all,+loaddll".into()),
        (
            "WINEDLLOVERRIDES",
            "winedbg.exe=d;winemenubuilder.exe=d;mscoree=d;mshtml=d".into(),
        ),
    ] {
        set(&mut env, name, value);
    }
    env
}
fn runtime(build: &Build, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = explicit {
        return root.canonicalize().context("Resolve probe runtime");
    }
    build.runtime_environment("runtime-tools")?;
    build
        .work
        .join("runtime")
        .canonicalize()
        .context("Resolve built runtime")
}
fn compile(
    build: &Build,
    architecture: &str,
    source: &str,
    output: &Path,
    libraries: &[&str],
) -> Result<()> {
    let compiler = if architecture == "x86" {
        "i686"
    } else {
        "x86_64"
    };
    let mut args = vec![
        "-O2".into(),
        "-Wall".into(),
        "-Wextra".into(),
        p(repo().join("tools/runtime-probes").join(source)),
        "-o".into(),
        p(output),
    ];
    args.extend(libraries.iter().map(|s| (*s).to_owned()));
    run(
        build
            .compiler
            .join(format!("bin/{compiler}-w64-mingw32-gcc")),
        &args,
        Some(&build.env),
        None,
        Some(&output.with_extension("build.log")),
        120,
        0,
    )
}

pub fn graphics(build: &Build, explicit: Option<&Path>) -> Result<()> {
    let runtime = runtime(build, explicit)?;
    let output = build.work.join("graphics-regression");
    fs::create_dir_all(&output)?;
    let mut tests = Vec::new();
    for architecture in ["x86", "x64"] {
        let env = environment(
            build,
            &runtime,
            &output.join(format!("prefix-{architecture}")),
        );
        for (api, source, libraries, marker) in [
            (
                "wgl",
                "wgl.c",
                vec!["-lopengl32", "-lgdi32"],
                "ISOTOPE_WGL_OK",
            ),
            (
                "d3d11",
                "d3d11-present.c",
                vec!["-ld3d11", "-ldxgi", "-ldxguid"],
                "ISOTOPE_D3D11_OK",
            ),
        ] {
            let exe = output.join(format!("{api}-{architecture}.exe"));
            compile(build, architecture, source, &exe, &libraries)?;
            let mut env = env.clone();
            set(
                &mut env,
                "NUCLEON_EXPECT_GRAPHICS_ABI",
                if architecture == "x86" {
                    "x86"
                } else {
                    "arm64ec"
                },
            );
            let result = marker_run(
                &runtime.join("bin/wine"),
                &[p(&exe)],
                &env,
                Some(&output),
                &output.join(format!("{api}-{architecture}.log")),
                60,
                0,
                marker,
            );
            tests.push(
                json!({"architecture":architecture,"api":api,"probe_sha256":sha(&exe)?,
                "status":if result.is_ok() { "PASSED" } else { "FAILED" },
                "error":result.err().map(|e| format!("{e:#}"))}),
            );
        }
        crate::common::cleanup_server(
            &runtime.join("bin/wineserver"),
            &env,
            &output.join(format!("cleanup-{architecture}.log")),
        )?;
    }
    let passed = tests.iter().all(|t| t["status"] == "PASSED");
    write_json(
        &output.join("result.json"),
        &json!({"schema":1,"runtime":runtime,"tests":tests,"games_tested":false}),
    )?;
    ensure!(
        passed,
        "Graphics regression failed; see {}",
        output.join("result.json").display()
    );
    Ok(())
}

pub fn supervision(build: &Build, explicit: Option<&Path>) -> Result<()> {
    let runtime = runtime(build, explicit)?;
    let output = build.work.join("supervision-regression");
    fs::create_dir_all(&output)?;
    let exe = output.join("supervisor-guest.exe");
    compile(build, "x86", "supervisor-guest.c", &exe, &[])?;
    let mut env = environment(build, &runtime, &output.join("prefix"));
    // A dedicated owner makes SIGKILL testable without killing this test runner.
    let mut control = OwnedChild::spawn(Command::new("/bin/sleep").arg("90"))?;
    let mut owner = OwnedChild::spawn(Command::new("/bin/sleep").arg("90"))?;
    // Obtain the spawned owner's PID without exposing it as a public lifecycle API.
    let owner_pid = owner.id();
    set(&mut env, "ISOTOPE_SUPERVISOR_PID", owner_pid.to_string());
    let log = output.join("guests.log");
    let stream = fs::File::create(&log)?;
    let mut cmd = command(
        runtime.join("bin/wine"),
        &[p(&exe)],
        Some(&env),
        Some(&output),
    );
    cmd.stdout(Stdio::from(stream.try_clone()?))
        .stderr(Stdio::from(stream));
    let mut guest = OwnedChild::spawn(&mut cmd)?;
    let result = (|| {
        let start = Instant::now();
        loop {
            let text = fs::read_to_string(&log)?;
            if text.contains("role=parent") && text.contains("role=child") {
                break;
            }
            ensure!(
                guest.try_wait()?.is_none(),
                "Guest exited before both processes were ready"
            );
            ensure!(
                start.elapsed() < Duration::from_secs(45),
                "Guest tree did not become ready"
            );
            thread::sleep(Duration::from_millis(100));
        }
        // SIGKILL skips destructors and all normal launcher cleanup.
        owner.kill()?;
        let start = Instant::now();
        loop {
            if let Some(status) = guest.try_wait()? {
                ensure!(
                    !status.success(),
                    "Guest unexpectedly returned success after losing owner"
                );
                break;
            }
            ensure!(
                start.elapsed() < Duration::from_secs(10),
                "Guest survived launcher SIGKILL"
            );
            thread::sleep(Duration::from_millis(50));
        }
        ensure!(
            control.try_wait()?.is_none(),
            "Unrelated control process was terminated"
        );
        // The server's watcher must terminate the prefix as well, including services.
        let mut wait_env = env.clone();
        wait_env.remove(std::ffi::OsStr::new("ISOTOPE_SUPERVISOR_PID"));
        run(
            runtime.join("bin/wineserver"),
            &["-w".into()],
            Some(&wait_env),
            None,
            Some(&output.join("server-wait.log")),
            10,
            0,
        )?;
        Ok(())
    })();
    let cleanup = guest.terminate(TERMINATION_GRACE);
    let _ = owner.terminate(TERMINATION_GRACE);
    let _ = control.terminate(TERMINATION_GRACE);
    let server_cleanup = crate::common::cleanup_server(
        &runtime.join("bin/wineserver"),
        &environment(build, &runtime, &output.join("prefix")),
        &output.join("cleanup.log"),
    );
    let isolated = if result.is_ok() {
        guest_only_supervision(build, &runtime, &output, &exe)
    } else {
        Ok(())
    };
    write_json(
        &output.join("result.json"),
        &json!({"schema":1,"runtime":runtime,"owner_signal":"SIGKILL","unrelated_control_checked":true,
        "guest_without_server_watcher": if isolated.is_ok() && result.is_ok() { "PASSED" } else { "FAILED" },
        "isolated_error": isolated.as_ref().err().map(|e:&anyhow::Error|format!("{e:#}")),
        "guest_tree":if result.is_ok(){"PASSED"}else{"FAILED"},"error":result.as_ref().err().map(|e:&anyhow::Error|format!("{e:#}"))}),
    )?;
    result?;
    cleanup?;
    server_cleanup?;
    isolated?;
    Ok(())
}

/// Wine's native-DLL rejection stays intact. Use identifiable copies of real Wine
/// builtins to verify precedence without depending on a particular frontend build.
pub fn dll_search(build: &Build, explicit: Option<&Path>) -> Result<()> {
    let runtime = runtime(build, explicit)?;
    let output = build.work.join("dll-search-regression");
    fs::create_dir_all(&output)?;
    let mut tests = Vec::new();
    for architecture in ["x86", "x64"] {
        let dir = if architecture == "x86" {
            "i386-windows"
        } else {
            "aarch64-windows"
        };
        let original = fs::read(runtime.join("lib/wine").join(dir).join("d3d11.dll"))?;
        ensure!(
            original.get(64..80) == Some(b"Wine builtin DLL"),
            "Fixture must be a Wine builtin"
        );
        let pe = u32::from_le_bytes(
            original
                .get(60..64)
                .context("Truncated DOS header")?
                .try_into()?,
        ) as usize;
        let timestamp = u32::from_le_bytes(
            original
                .get(pe + 8..pe + 12)
                .context("Truncated PE header")?
                .try_into()?,
        );
        let alternate = timestamp.wrapping_add(12345);
        let path = output.join(architecture);
        fs::create_dir_all(&path)?;
        let mut modified = original.clone();
        modified[pe + 8..pe + 12].copy_from_slice(&alternate.to_le_bytes());
        fs::write(path.join("d3d11.dll"), &modified)?;
        let exe = output.join(format!("search-{architecture}.exe"));
        compile(build, architecture, "dll-search.c", &exe, &[])?;
        let base = environment(
            build,
            &runtime,
            &output.join(format!("prefix-{architecture}")),
        );
        for (case, expected) in [
            ("appended", timestamp),
            ("prepended", alternate),
            ("native-rejected", timestamp),
        ] {
            let mut env = base.clone();
            set(
                &mut env,
                "WINEDLLOVERRIDES",
                "d3d11=b;winemenubuilder.exe=d;mscoree=d;mshtml=d;winedbg.exe=d",
            );
            if case == "appended" {
                let original_path = env[std::ffi::OsStr::new("WINEDLLPATH")].to_string_lossy();
                let paths = format!("{}:{original_path}", path.display());
                set(&mut env, "WINEDLLPATH", paths);
            } else {
                // Empty path entries must never introduce the current directory.
                set(
                    &mut env,
                    "WINEDLLPATH_PREPEND",
                    format!(":{}::", path.display()),
                );
            }
            if case == "native-rejected" {
                modified[64..80].fill(0);
                fs::write(path.join("d3d11.dll"), &modified)?;
            }
            let result = marker_run(
                &runtime.join("bin/wine"),
                &[p(&exe), expected.to_string()],
                &env,
                Some(&output),
                &output.join(format!("{case}-{architecture}.log")),
                60,
                0,
                "ISOTOPE_DLL_SEARCH_OK",
            );
            tests.push(json!({"architecture":architecture,"case":case,"status":if result.is_ok(){"PASSED"}else{"FAILED"},
                "error":result.err().map(|e|format!("{e:#}"))}));
        }
        crate::common::cleanup_server(
            &runtime.join("bin/wineserver"),
            &base,
            &output.join(format!("cleanup-{architecture}.log")),
        )?;
    }
    let passed = tests.iter().all(|t| t["status"] == "PASSED");
    write_json(
        &output.join("result.json"),
        &json!({"schema":1,"runtime":runtime,"tests":tests}),
    )?;
    ensure!(
        passed,
        "DLL precedence regression failed; see {}",
        output.display()
    );
    Ok(())
}

pub fn vulkan(build: &Build, explicit: Option<&Path>, sdk: Option<&Path>) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Settings {
        environment: std::collections::BTreeMap<String, String>,
        graphics: Vulkan,
    }
    #[derive(serde::Deserialize)]
    struct Vulkan {
        vulkan_icd: PathBuf,
        vulkan_loader: PathBuf,
    }
    let runtime = runtime(build, explicit)?;
    let settings: Settings =
        toml::from_str(&fs::read_to_string(runtime.join("nucleon-runtime.toml"))?)?;
    let sdk = sdk.map(Path::to_path_buf).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join("VulkanSDK/1.4.363.0/macOS")
    });
    let include = format!("-I{}", sdk.join("include").display());
    let output = build.work.join("vulkan-regression");
    fs::create_dir_all(&output)?;
    let mut tests = Vec::new();
    for architecture in ["x86", "x64"] {
        let exe = output.join(format!("vulkan-{architecture}.exe"));
        compile(build, architecture, "vulkan.c", &exe, &[&include])?;
        let mut env = environment(
            build,
            &runtime,
            &output.join(format!("prefix-{architecture}")),
        );
        for (k, v) in &settings.environment {
            set(&mut env, k, v);
        }
        set(
            &mut env,
            "VK_DRIVER_FILES",
            runtime.join(&settings.graphics.vulkan_icd),
        );
        let lib = format!(
            "{}:{}",
            runtime
                .join(&settings.graphics.vulkan_loader)
                .parent()
                .context("Missing loader parent")?
                .display(),
            runtime.join("lib/wine/aarch64-unix").display()
        );
        set(&mut env, "DYLD_LIBRARY_PATH", lib);
        set(
            &mut env,
            "WINEDLLOVERRIDES",
            "vulkan-1,winevulkan=b;winedbg.exe=d;winemenubuilder.exe=d;mscoree=d;mshtml=d",
        );
        let result = marker_run(
            &runtime.join("bin/wine"),
            &[p(&exe)],
            &env,
            Some(&output),
            &output.join(format!("{architecture}.log")),
            90,
            0,
            "ISOTOPE_VULKAN_PRESENT_OK",
        );
        tests.push(json!({"architecture":architecture,"status":if result.is_ok(){"PASSED"}else{"FAILED"},"probe_sha256":sha(&exe)?,"error":result.err().map(|e|format!("{e:#}"))}));
        crate::common::cleanup_server(
            &runtime.join("bin/wineserver"),
            &env,
            &output.join(format!("cleanup-{architecture}.log")),
        )?;
    }
    let passed = tests.iter().all(|t| t["status"] == "PASSED");
    write_json(
        &output.join("result.json"),
        &json!({"schema":1,"runtime":runtime,"tests":tests}),
    )?;
    ensure!(
        passed,
        "Vulkan presentation regression failed; see {}",
        output.display()
    );
    Ok(())
}

fn guest_only_supervision(build: &Build, runtime: &Path, output: &Path, exe: &Path) -> Result<()> {
    let mut env = environment(build, runtime, &output.join("guest-only-prefix"));
    env.remove(std::ffi::OsStr::new("ISOTOPE_SUPERVISOR_PID"));
    set(&mut env, "WINEDEBUG", "-all");
    // This server and its services predate the supervised guests. They must stay
    // alive for an independent observer to prove each guest exits on its own.
    run(
        runtime.join("bin/wine"),
        &["wineboot".into(), "-u".into()],
        Some(&env),
        None,
        Some(&output.join("guest-only-bootstrap.log")),
        60,
        0,
    )?;
    let mut owner = OwnedChild::spawn(Command::new("/bin/sleep").arg("90"))?;
    let mut watched = env.clone();
    set(
        &mut watched,
        "ISOTOPE_SUPERVISOR_PID",
        owner.id().to_string(),
    );
    let log = output.join("guest-only.log");
    let stream = fs::File::create(&log)?;
    let mut cmd = command(
        runtime.join("bin/wine"),
        &[p(exe)],
        Some(&watched),
        Some(output),
    );
    cmd.stdout(Stdio::from(stream.try_clone()?))
        .stderr(Stdio::from(stream));
    let mut guest = OwnedChild::spawn(&mut cmd)?;
    let result = (|| {
        let start = Instant::now();
        let (parent, child) = loop {
            let text = fs::read_to_string(&log)?;
            let pid = |role: &str| -> Option<u32> {
                text.split(&format!("role={role} pid="))
                    .nth(1)?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()
            };
            if let (Some(parent), Some(child)) = (pid("parent"), pid("child")) {
                break (parent, child);
            }
            ensure!(
                guest.try_wait()?.is_none(),
                "Isolated guest exited before readiness"
            );
            ensure!(
                start.elapsed() < Duration::from_secs(30),
                "Isolated guest readiness timed out"
            );
            thread::sleep(Duration::from_millis(50));
        };
        let log = output.join("observer.log");
        let stream = fs::File::create(&log)?;
        let mut cmd = command(
            runtime.join("bin/wine"),
            &[
                p(exe),
                "--observe".into(),
                parent.to_string(),
                child.to_string(),
            ],
            Some(&env),
            Some(output),
        );
        cmd.stdout(Stdio::from(stream.try_clone()?))
            .stderr(Stdio::from(stream));
        let mut observer = OwnedChild::spawn(&mut cmd)?;
        let observed = (|| {
            let start = Instant::now();
            loop {
                if fs::read_to_string(&log)?.contains("ISOTOPE_OBSERVER_READY") {
                    break;
                }
                ensure!(
                    observer.try_wait()?.is_none(),
                    "Independent observer exited before readiness"
                );
                ensure!(
                    start.elapsed() < Duration::from_secs(15),
                    "Independent observer readiness timed out"
                );
                thread::sleep(Duration::from_millis(50));
            }
            owner.kill()?;
            let start = Instant::now();
            loop {
                if let Some(status) = observer.try_wait()? {
                    ensure!(
                        status.success()
                            && fs::read_to_string(&log)?.contains("ISOTOPE_GUEST_TREE_EXIT_OK"),
                        "Guest or Windows child survived owner loss with an unsupervised server"
                    );
                    break;
                }
                ensure!(
                    start.elapsed() < Duration::from_secs(15),
                    "Independent observer timed out"
                );
                thread::sleep(Duration::from_millis(50));
            }
            Ok(())
        })();
        let cleanup = observer.terminate(TERMINATION_GRACE);
        observed?;
        cleanup?;
        Ok(())
    })();
    let cleanup = guest.terminate(TERMINATION_GRACE);
    let _ = owner.terminate(TERMINATION_GRACE);
    let server = crate::common::cleanup_server(
        &runtime.join("bin/wineserver"),
        &env,
        &output.join("guest-only-cleanup.log"),
    );
    result?;
    cleanup?;
    server?;
    Ok(())
}
