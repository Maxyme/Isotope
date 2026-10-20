//! Execute the real consumer; this repository has no source dependency on Nucleon.
use crate::common::{clean_environment, command, execute, output, p, set, sha};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
pub const RECEIPT: &str = ".nucleon-install.json";
#[derive(Debug, Deserialize)]
pub struct Graphics {
    pub dxmt: Option<PathBuf>,
    pub dxvk: Option<PathBuf>,
    pub vkd3d: Option<PathBuf>,
    pub vulkan_icd: Option<PathBuf>,
    pub vulkan_loader: Option<PathBuf>,
}
#[derive(Debug, Deserialize)]
pub struct Runtime {
    pub root: PathBuf,
    pub loader: PathBuf,
    pub server: PathBuf,
    pub unix_dir: PathBuf,
    pub windows_dirs: Vec<PathBuf>,
    pub environment: BTreeMap<String, String>,
    pub graphics: Graphics,
}
fn executable() -> PathBuf {
    crate::NUCLEON
        .get()
        .cloned()
        .unwrap_or_else(|| "nucleon".into())
}
impl Runtime {
    pub fn inspect(root: &Path) -> Result<Self> {
        let value = output(executable(), &["wine".into(),"inspect".into(),p(root),"--json".into()], Some(&clean_environment()), None)
            .context("Consumer validation requires Nucleon with 'wine inspect --json'; pass --nucleon /path/to/nucleon")?;
        parse_inspection(&value)
    }
}
fn parse_inspection(value: &str) -> Result<Runtime> {
    // The producer is independently versioned: accept additional response fields.
    #[derive(Deserialize)]
    struct Inspection {
        schema: u32,
        runtime: Runtime,
    }
    let inspection: Inspection =
        serde_json::from_str(value).context("Invalid Nucleon inspection response")?;
    ensure!(
        inspection.schema == 1,
        "Unsupported consumer inspection schema"
    );
    Ok(inspection.runtime)
}

pub fn install_archive(archive: &Path, support: &Path) -> Result<Runtime> {
    fs::create_dir_all(support)?;
    let mut env = clean_environment();
    set(&mut env, "NUCLEON_SUPPORT_DIR", support);
    let mut cmd = command(
        executable(),
        &[
            "wine".into(),
            "install".into(),
            p(archive),
            "--sha256".into(),
            sha(archive)?,
        ],
        Some(&env),
        None,
    );
    execute(
        &mut cmd,
        Some(&support.join("consumer-install.log")),
        Duration::from_secs(600),
        0,
        Duration::from_millis(300),
    )?;
    let store = support.join("runtimes/arm64-fex");
    let roots: Vec<_> = fs::read_dir(&store)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|p| p.join(RECEIPT).is_file())
        .collect();
    ensure!(
        roots.len() == 1,
        "Expected exactly one installed generation in isolated storage"
    );
    let root = &roots[0];
    let mut cmd = command(
        executable(),
        &["wine".into(), "fex".into(), "verify".into(), p(root)],
        Some(&env),
        None,
    );
    execute(
        &mut cmd,
        Some(&support.join("consumer-verify.log")),
        Duration::from_secs(600),
        0,
        Duration::from_millis(300),
    )?;
    Runtime::inspect(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_accepts_extensions_but_rejects_wrong_shapes_and_schemas() {
        let valid = serde_json::json!({"schema":1,"future_envelope_field":true,"runtime":{
            "root":"/runtime","loader":"/runtime/wine","server":"/runtime/wineserver","unix_dir":"/runtime/unix",
            "windows_dirs":["/runtime/windows"],"environment":{},"graphics":{"future_backend":{}},"future_runtime_field":[]
        }});
        assert_eq!(
            parse_inspection(&valid.to_string()).unwrap().root,
            PathBuf::from("/runtime")
        );
        let mut bad = valid.clone();
        bad["schema"] = 2.into();
        assert!(parse_inspection(&bad.to_string()).is_err());
        let mut bad = valid.clone();
        bad["runtime"]["windows_dirs"] = false.into();
        assert!(parse_inspection(&bad.to_string()).is_err());
        let mut bad = valid;
        bad["runtime"].as_object_mut().unwrap().remove("loader");
        assert!(parse_inspection(&bad.to_string()).is_err());
    }
}
