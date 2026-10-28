//! Optional Nucleon inspection for experimental graphics assembly.
use crate::common::{clean_environment, output, p};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::path::{Path, PathBuf};
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
    pub graphics: Graphics,
}
impl Runtime {
    fn inspect(root: &Path, executable: &Path) -> Result<Self> {
        let value = output(executable, &["wine".into(),"inspect".into(),p(root),"--json".into()], Some(&clean_environment()), None)
            .context("Consumer validation requires Nucleon with 'wine inspect --json'; pass --nucleon /path/to/nucleon")?;
        let runtime = parse_inspection(&value)?;
        ensure!(
            runtime.root == root.canonicalize()?,
            "Nucleon inspected a different runtime root"
        );
        Ok(runtime)
    }
}
/// Explicit post-assembly consumer validation; ordinary assembly never calls Nucleon.
pub fn check(root: &Path, executable: &Path, report: &Path) -> Result<()> {
    ensure!(
        !report.exists(),
        "Consumer validation report already exists"
    );
    let local = crate::runtime::Runtime::inspect(root)?;
    let consumer = Runtime::inspect(&local.root, executable)?;
    ensure!(
        consumer.loader == local.loader
            && consumer.server == local.server
            && consumer.unix_dir == local.unix_dir
            && consumer.windows_dirs == local.windows_dirs,
        "Consumer resolved a different runtime layout"
    );
    // Keep the consumer result separate from immutable runtime contents.
    crate::common::write_json(
        report,
        &serde_json::json!({
            "schema":1,"status":"PASSED","runtime":consumer.root,"consumer":executable,
            "graphics":{"dxmt":consumer.graphics.dxmt,"dxvk":consumer.graphics.dxvk,"vkd3d":consumer.graphics.vkd3d,
              "vulkan_icd":consumer.graphics.vulkan_icd,"vulkan_loader":consumer.graphics.vulkan_loader},
            "execution_validation":"NOT_RUN","steam_validation":"NOT_RUN","gameplay_validation":"NOT_RUN"
        }),
    )
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
