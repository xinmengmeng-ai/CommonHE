use std::path::{Path, PathBuf};

pub fn resolve_status_path(
    project_root: Option<&Path>,
    session_root: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(session_root) = session_root {
        return Ok(session_root.join("status.json"));
    }

    if let Some(project_root) = project_root {
        return Ok(project_root
            .join(".commonhe")
            .join("session")
            .join("status.json"));
    }

    Err("projectRoot or sessionRoot is required to read status.".to_string())
}

pub fn read_status_json(
    project_root: Option<&Path>,
    session_root: Option<&Path>,
) -> Result<String, String> {
    let status_path = resolve_status_path(project_root, session_root)?;
    if !status_path.is_file() {
        return Ok(format!(
            "{{\"exists\":false,\"path\":{}}}",
            super::json::json_string(&status_path.display().to_string())
        ));
    }

    let content = std::fs::read_to_string(&status_path)
        .map_err(|error| format!("Failed to read status file: {error}"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("Invalid status JSON: {error}"))?;

    Ok(format!(
        "{{\"exists\":true,\"path\":{},\"status\":{}}}",
        super::json::json_string(&status_path.display().to_string()),
        parsed
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_project_status_file() {
        let project_root = Path::new("E:\\WorkSoft\\Demo");

        assert_eq!(
            resolve_status_path(Some(project_root), None).unwrap(),
            project_root
                .join(".commonhe")
                .join("session")
                .join("status.json")
        );
    }

    #[test]
    fn reads_existing_status_as_json_payload() {
        let temp =
            std::env::temp_dir().join(format!("commonhe-status-test-{}", std::process::id()));
        let session_root = temp.join(".commonhe").join("session");
        fs::create_dir_all(&session_root).unwrap();
        fs::write(session_root.join("status.json"), "{\"stage\":\"doctor\"}").unwrap();

        let json = read_status_json(None, Some(&session_root)).unwrap();

        assert!(json.contains("\"exists\":true"));
        assert!(json.contains("\"stage\":\"doctor\""));
        let _ = fs::remove_dir_all(&temp);
    }
}
