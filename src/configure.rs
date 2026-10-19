//! Configuration reuse is opt-in: old or changed build trees are never cleaned implicitly.
use crate::common::{key, write_json};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const RECEIPT: &str = "isotope-configure.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigureInputs {
    pub build_root: PathBuf,
    pub source_root: PathBuf,
    pub source_revision: String,
    pub source_diff_sha256: String,
    pub patches: BTreeMap<String, String>,
    pub toolchain_sha256: String,
    pub tools: BTreeMap<String, ToolIdentity>,
    pub native_sdk: SdkIdentity,
    pub graphics_sdk: Option<SdkIdentity>,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    pub path: PathBuf,
    pub sha256: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SdkIdentity {
    pub root: PathBuf,
    pub version: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u32,
    inputs_sha256: String,
    inputs: ConfigureInputs,
    outputs: BTreeMap<String, String>,
}

/// Record success only after configure produced its Makefile. A failed or interrupted
/// configure leaves an untrusted tree, even when it managed to write a Makefile.
pub fn configure_once(
    directory: &Path,
    inputs: &ConfigureInputs,
    configure: impl FnOnce() -> Result<()>,
) -> Result<()> {
    fs::create_dir_all(directory)?;
    let receipt = directory.join(RECEIPT);
    let guidance = || {
        format!(
            "Preserve Wine build directory {}; use a fresh --work directory (configuration inputs changed or reuse cannot be verified)",
            directory.display()
        )
    };
    if fs::read_dir(directory)?.next().transpose()?.is_some() {
        let previous: Receipt = serde_json::from_slice(&fs::read(&receipt).with_context(guidance)?)
            .with_context(guidance)?;
        ensure!(
            previous.schema == 1
                && previous.inputs_sha256 == key(&previous.inputs)?
                && previous.inputs == *inputs
                && directory.join("Makefile").is_file()
                && directory.join("config.status").is_file()
                && previous.outputs == configuration_outputs(directory)?,
            "{}",
            guidance()
        );
        return Ok(());
    }
    configure()?;
    ensure!(
        directory.join("Makefile").is_file() && directory.join("config.status").is_file(),
        "Wine configure did not produce Makefile and config.status; {}",
        guidance()
    );
    write_json(
        &receipt,
        &Receipt {
            schema: 1,
            inputs_sha256: key(inputs)?,
            inputs: inputs.clone(),
            outputs: configuration_outputs(directory)?,
        },
    )
}

fn configuration_outputs(directory: &Path) -> Result<BTreeMap<String, String>> {
    crate::common::hash_files(directory, &["Makefile".into(), "config.status".into()])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> ConfigureInputs {
        ConfigureInputs {
            build_root: "/build".into(),
            source_root: "/source".into(),
            source_revision: "revision".into(),
            source_diff_sha256: "diff".into(),
            patches: BTreeMap::from([("fix.patch".into(), "patch".into())]),
            toolchain_sha256: "toolchain".into(),
            tools: BTreeMap::from([(
                "cc".into(),
                ToolIdentity {
                    path: "/compiler".into(),
                    sha256: "binary".into(),
                    version: "clang version".into(),
                },
            )]),
            native_sdk: SdkIdentity {
                root: "/sdk".into(),
                version: "1".into(),
                files: BTreeMap::new(),
            },
            graphics_sdk: None,
            arguments: vec!["--without-vulkan".into()],
            environment: BTreeMap::from([("CFLAGS".into(), "-O2".into())]),
        }
    }
    fn configure(directory: &Path, inputs: &ConfigureInputs) {
        configure_once(directory, inputs, || {
            fs::write(directory.join("Makefile"), "original Makefile")?;
            fs::write(directory.join("config.status"), "original configuration")?;
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn identical_inputs_reuse_configuration_without_running_configure() {
        let dir = tempfile::tempdir().unwrap();
        configure(dir.path(), &inputs());
        configure_once(dir.path(), &inputs(), || panic!("must reuse")).unwrap();
    }
    #[test]
    fn each_changed_input_rejects_reuse_and_preserves_existing_files() {
        let base = inputs();
        let mut changes = vec![];
        let mut change = |edit: fn(&mut ConfigureInputs)| {
            let mut value = base.clone();
            edit(&mut value);
            changes.push(value);
        };
        change(|i| i.build_root = "/moved-build".into());
        change(|i| i.source_root = "/moved-source".into());
        change(|i| i.source_revision.push('2'));
        change(|i| i.source_diff_sha256.push('2'));
        change(|i| {
            i.patches.insert("fix.patch".into(), "changed".into());
        });
        change(|i| i.toolchain_sha256.push('2'));
        change(|i| i.tools.get_mut("cc").unwrap().sha256.push('2'));
        change(|i| i.tools.get_mut("cc").unwrap().version.push('2'));
        change(|i| i.native_sdk.version.push('2'));
        change(|i| {
            i.graphics_sdk = Some(i.native_sdk.clone());
        });
        change(|i| i.arguments.push("--with-vulkan".into()));
        change(|i| {
            i.environment.insert("CFLAGS".into(), "-O0".into());
        });
        for updated in changes {
            let dir = tempfile::tempdir().unwrap();
            configure(dir.path(), &base);
            let receipt = fs::read(dir.path().join(RECEIPT)).unwrap();
            let err = configure_once(dir.path(), &updated, || panic!("must reject")).unwrap_err();
            assert!(format!("{err:#}").contains("fresh --work"));
            assert_eq!(fs::read(dir.path().join(RECEIPT)).unwrap(), receipt);
            assert_eq!(
                fs::read_to_string(dir.path().join("Makefile")).unwrap(),
                "original Makefile"
            );
        }
    }
    #[test]
    fn legacy_corrupt_and_incomplete_trees_are_not_adopted() {
        for mode in [
            "legacy",
            "corrupt",
            "incomplete",
            "wrong-schema",
            "tampered",
            "reconfigured",
        ] {
            let dir = tempfile::tempdir().unwrap();
            configure(dir.path(), &inputs());
            match mode {
                "legacy" => fs::remove_file(dir.path().join(RECEIPT)).unwrap(),
                "reconfigured" => {
                    fs::write(dir.path().join("config.status"), "manually reconfigured").unwrap()
                }
                "corrupt" => fs::write(dir.path().join(RECEIPT), "{").unwrap(),
                "incomplete" => fs::remove_file(dir.path().join("Makefile")).unwrap(),
                _ => {
                    let mut value: Receipt =
                        serde_json::from_slice(&fs::read(dir.path().join(RECEIPT)).unwrap())
                            .unwrap();
                    if mode == "wrong-schema" {
                        value.schema = 2;
                    } else {
                        value.inputs_sha256 = "wrong".into();
                    }
                    write_json(&dir.path().join(RECEIPT), &value).unwrap();
                }
            }
            assert!(configure_once(dir.path(), &inputs(), || panic!("must reject")).is_err());
            assert!(dir.path().join("config.status").is_file());
        }
    }
    #[test]
    fn failed_configure_cannot_publish_a_reusable_receipt() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            configure_once(dir.path(), &inputs(), || {
                fs::write(dir.path().join("Makefile"), "partial")?;
                anyhow::bail!("configure failed")
            })
            .is_err()
        );
        assert!(!dir.path().join(RECEIPT).exists());
        assert!(
            configure_once(dir.path(), &inputs(), || panic!("must reject partial tree")).is_err()
        );
    }
}
