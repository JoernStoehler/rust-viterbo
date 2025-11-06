use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs;
use std::panic::Location;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Metadata used to generate a provenance sidecar.
pub struct Payload {
    pub params: Value,
    pub th: Vec<String>,
}

impl Payload {
    pub fn new(params: Value) -> Self {
        Self {
            params,
            th: Vec::new(),
        }
    }
}

/// Write `<artifact>.provenance.json` containing the git commit, callsite, params, and outputs.
#[track_caller]
pub fn write_sidecar<P: AsRef<Path>>(artifact: P, payload: Payload) -> Result<PathBuf> {
    let artifact = artifact.as_ref();
    let provenance_path = provenance_path(artifact);
    if let Some(parent) = provenance_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating provenance dir {}", parent.display()))?;
        }
    }

    let callsite = Location::caller();
    let doc = json!({
        "code_rev": current_git_rev(),
        "callsite": {
            "file": callsite.file(),
            "line": callsite.line()
        },
        "th": payload.th,
        "params": payload.params,
        "outputs": [artifact.to_string_lossy()]
    });
    fs::write(&provenance_path, serde_json::to_vec_pretty(&doc)?)
        .with_context(|| format!("writing {}", provenance_path.display()))?;
    Ok(provenance_path)
}

fn provenance_path(artifact: &Path) -> PathBuf {
    let stem = artifact
        .file_stem()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| OsString::from("artifact"));
    let mut name = stem;
    name.push(".provenance.json");
    artifact.with_file_name(name)
}

pub fn current_git_rev() -> String {
    if let Some(from_env) = option_env!("GIT_COMMIT") {
        if !from_env.is_empty() {
            return from_env.to_string();
        }
    }
    if let Ok(env_override) = std::env::var("GIT_COMMIT") {
        if !env_override.is_empty() {
            return env_override;
        }
    }
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn provenance_path_rewrites_extension() {
        let base = Path::new("/tmp/output/foo.csv");
        let derived = provenance_path(base);
        assert_eq!(derived, Path::new("/tmp/output/foo.provenance.json"));
    }

    #[test]
    fn write_sidecar_creates_file() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("a.json");
        fs::write(&artifact, "{}").unwrap();
        let payload = Payload::new(json!({"algo": "demo"}));
        let prov_path = write_sidecar(&artifact, payload).unwrap();
        assert!(prov_path.exists());
        let parsed: Value = serde_json::from_slice(&fs::read(prov_path).unwrap()).unwrap();
        assert_eq!(parsed["outputs"][0], artifact.to_string_lossy().as_ref());
    }
}
