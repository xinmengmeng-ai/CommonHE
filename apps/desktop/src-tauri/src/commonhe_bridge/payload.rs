use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PayloadSource {
    BundledResource,
    RepoFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadLocation {
    pub source: PayloadSource,
    pub payload_root: PathBuf,
    pub orchestrator_path: PathBuf,
    pub checked_roots: Vec<PathBuf>,
}

pub fn locate_payload(
    resource_dir: Option<&Path>,
    manifest_dir: Option<&Path>,
    current_dir: Option<&Path>,
) -> Result<PayloadLocation, String> {
    let mut checked_roots = Vec::new();

    if let Some(resource_dir) = resource_dir {
        for candidate in resource_candidates(resource_dir) {
            checked_roots.push(candidate.clone());
            if is_payload_root(&candidate) {
                return Ok(PayloadLocation {
                    source: PayloadSource::BundledResource,
                    orchestrator_path: candidate
                        .join("tools")
                        .join("common-he-init-orchestrator.ps1"),
                    payload_root: candidate,
                    checked_roots,
                });
            }
        }
    }

    for start in [manifest_dir, current_dir].into_iter().flatten() {
        for candidate in ancestor_candidates(start) {
            checked_roots.push(candidate.clone());
            if is_payload_root(&candidate) {
                return Ok(PayloadLocation {
                    source: PayloadSource::RepoFallback,
                    orchestrator_path: candidate
                        .join("tools")
                        .join("common-he-init-orchestrator.ps1"),
                    payload_root: candidate,
                    checked_roots,
                });
            }
        }
    }

    Err(format!(
        "CommonHE payload was not found. Checked {} candidate roots.",
        checked_roots.len()
    ))
}

impl PayloadLocation {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("payload location serialization cannot fail")
    }
}

fn resource_candidates(resource_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![resource_dir.to_path_buf()];
    candidates.push(resource_dir.join("commonhe"));
    candidates.push(resource_dir.join("CommonHE"));
    candidates.push(resource_dir.join("resources").join("commonhe"));
    candidates.push(resource_dir.join("resources").join("CommonHE"));
    candidates
}

fn ancestor_candidates(start: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for ancestor in start.ancestors() {
        candidates.push(ancestor.to_path_buf());
    }
    candidates
}

fn is_payload_root(root: &Path) -> bool {
    root.join("tools")
        .join("common-he-init-orchestrator.ps1")
        .is_file()
        && ["config", "core", "init", "templates", "tools"]
            .iter()
            .all(|directory| root.join(directory).is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_payload(root: &Path) {
        fs::create_dir_all(root.join("tools")).unwrap();
        fs::write(
            root.join("tools").join("common-he-init-orchestrator.ps1"),
            "# test",
        )
        .unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("core")).unwrap();
        fs::create_dir_all(root.join("init")).unwrap();
    }

    #[test]
    fn prefers_bundled_commonhe_resource() {
        let temp =
            std::env::temp_dir().join(format!("commonhe-payload-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let bundled_root = temp.join("resources").join("commonhe");
        let repo_root = temp.join("repo");
        make_payload(&bundled_root);
        make_payload(&repo_root);

        let location =
            locate_payload(Some(&temp.join("resources")), Some(&repo_root), None).unwrap();

        assert_eq!(location.source, PayloadSource::BundledResource);
        assert_eq!(location.payload_root, bundled_root);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn falls_back_to_repo_root_from_manifest_dir() {
        let temp = std::env::temp_dir().join(format!(
            "commonhe-payload-fallback-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        let manifest_dir = temp.join("apps").join("desktop").join("src-tauri");
        fs::create_dir_all(&manifest_dir).unwrap();
        make_payload(&temp);

        let location = locate_payload(None, Some(&manifest_dir), None).unwrap();

        assert_eq!(location.source, PayloadSource::RepoFallback);
        assert_eq!(location.payload_root, temp);
        let _ = fs::remove_dir_all(&temp);
    }
}
