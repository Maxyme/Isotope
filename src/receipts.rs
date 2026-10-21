//! Typed contracts for persisted build and publisher state.
use crate::common::{ArchivePin, Pin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type FileManifest = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Schema2;
impl TryFrom<u32> for Schema2 {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 2 {
            Ok(Self)
        } else {
            Err(format!("Unsupported receipt schema {value}; expected 2"))
        }
    }
}
impl From<Schema2> for u32 {
    fn from(_: Schema2) -> Self {
        2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildStatus {
    Building,
    BuiltNotExecutionValidated,
    BuildFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayReceipt {
    pub diff_sha256: String,
    pub patches: FileManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnixBuildInputs {
    pub revision: String,
    pub source: String,
    pub header: String,
    pub compiler: String,
    pub patches: FileManifest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnixBuildReceipt {
    pub schema: Schema2,
    pub inputs: UnixBuildInputs,
    pub files: FileManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WineBuildReceipt {
    pub schema: Schema2,
    pub status: BuildStatus,
    pub variant: String,
    pub recipe_sha256: String,
    pub sources: BTreeMap<String, Pin>,
    pub patches: FileManifest,
    pub toolchain_sha256: String,
    pub sdk: String,
    pub native_compiler: Option<String>,
    pub host_files: FileManifest,
    pub features: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphics_boundary_files: Option<FileManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dx12_recipe_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulkan_sdk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulkan_loader_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendBuildReceipt {
    pub schema: Schema2,
    pub status: BuildStatus,
    pub architecture: String,
    pub source: Pin,
    pub submodules: String,
    pub recipe_sha256: String,
    pub cross_sha256: String,
    pub toolchain: ArchivePin,
    pub log: std::path::PathBuf,
    pub options: Vec<String>,
    pub prerequisites: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crt_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emutls_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teb_header_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mingw_source: Option<Pin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<OverlayReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<FileManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    Testing,
    Passed,
    Failed,
    FailedOptional,
    PassedWithOptionalFailure,
    #[default]
    NotRun,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuestOutcome {
    pub status: ValidationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
impl GuestOutcome {
    pub fn new(status: ValidationStatus) -> Self {
        Self {
            status,
            error: None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub schema: Schema2,
    #[serde(default)]
    pub archive_relocation: ValidationStatus,
    pub managed_archive_import: ValidationStatus,
    pub integrity_and_native_dependencies: ValidationStatus,
    pub guest_smoke: BTreeMap<String, GuestOutcome>,
    pub x86_required: bool,
    pub steam: ValidationStatus,
    pub gameplay: ValidationStatus,
    pub status: ValidationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix_cleanup: Option<ValidationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn wine_and_frontend_receipts_keep_required_file_fields_typed() {
        // Required fields cannot silently become null and bypass verification.
        assert!(
            serde_json::from_value::<WineBuildReceipt>(
                json!({"schema":2,"status":"BUILT_NOT_EXECUTION_VALIDATED"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<FrontendBuildReceipt>(json!({"schema":2,"status":"BUILDING"}))
                .is_err()
        );
        let report = ValidationReport {
            schema: Schema2,
            archive_relocation: ValidationStatus::Passed,
            managed_archive_import: ValidationStatus::Passed,
            integrity_and_native_dependencies: ValidationStatus::Passed,
            guest_smoke: BTreeMap::from([(
                "x86".into(),
                GuestOutcome::new(ValidationStatus::FailedOptional),
            )]),
            x86_required: false,
            steam: ValidationStatus::NotRun,
            gameplay: ValidationStatus::NotRun,
            status: ValidationStatus::PassedWithOptionalFailure,
            prefix_cleanup: Some(ValidationStatus::Passed),
            error: None,
        };
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["status"], "PASSED_WITH_OPTIONAL_FAILURE");
        assert_eq!(value["guest_smoke"]["x86"]["status"], "FAILED_OPTIONAL");
        assert!(serde_json::from_value::<ValidationReport>(value.clone()).is_ok());
        let mut previous = value;
        previous
            .as_object_mut()
            .unwrap()
            .remove("archive_relocation");
        assert_eq!(
            serde_json::from_value::<ValidationReport>(previous)
                .unwrap()
                .archive_relocation,
            ValidationStatus::NotRun
        );
    }
    #[test]
    fn malformed_receipts_and_unknown_statuses_fail_at_the_boundary() {
        let valid = json!({"schema":2,"inputs":{"revision":"pin","source":"source","header":"header","compiler":"clang","patches":{}},"files":{"library.so":"hash"}});
        assert!(serde_json::from_value::<UnixBuildReceipt>(valid.clone()).is_ok());
        for field in ["schema", "inputs", "files"] {
            let mut value = valid.clone();
            value.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<UnixBuildReceipt>(value).is_err());
        }
        for (field, replacement) in [
            ("schema", json!(1)),
            ("files", json!(["wrong type"])),
            ("extra", json!(true)),
        ] {
            let mut value = valid.clone();
            value[field] = replacement;
            assert!(serde_json::from_value::<UnixBuildReceipt>(value).is_err());
        }
        assert!(serde_json::from_value::<BuildStatus>(json!("BUILD_SUCCEEDED")).is_err());
        assert!(serde_json::from_value::<GuestOutcome>(json!({"status":"PASSED_TYPO"})).is_err());
        assert!(
            serde_json::from_value::<GuestOutcome>(json!({"status":"FAILED_OPTIONAL","error":42}))
                .is_err()
        );
    }
    #[test]
    fn serialized_status_and_optional_failure_contract_is_preserved() {
        assert_eq!(
            serde_json::to_value(BuildStatus::BuiltNotExecutionValidated).unwrap(),
            "BUILT_NOT_EXECUTION_VALIDATED"
        );
        assert_eq!(
            serde_json::to_value(GuestOutcome::new(ValidationStatus::Passed)).unwrap(),
            json!({"status":"PASSED"})
        );
        let value = GuestOutcome {
            status: ValidationStatus::FailedOptional,
            error: Some("error 998".into()),
        };
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            json!({"status":"FAILED_OPTIONAL","error":"error 998"})
        );
    }
}
