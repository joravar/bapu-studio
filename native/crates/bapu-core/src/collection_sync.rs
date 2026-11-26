//! Git-friendly collection folder sync: mirrors a `Collection` to plain JSON files on disk
//! (`collection.json` manifest + `requests/<id>.json`, one file per request), matching the
//! Electron app's format in `electron-main.cjs`.

use crate::types::{ApiRequest, Collection, KeyValuePair};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct CollectionManifest {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    variables: Vec<KeyValuePair>,
    #[serde(rename = "requestOrder", default)]
    request_order: Vec<String>,
}

fn safe_file_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn write_collection_folder(folder: &Path, collection: &Collection) -> Result<()> {
    let requests_dir = folder.join("requests");
    fs::create_dir_all(&requests_dir)
        .with_context(|| format!("creating {}", requests_dir.display()))?;

    let request_order: Vec<String> = collection.requests.iter().map(|r| r.id.clone()).collect();
    let manifest = CollectionManifest {
        name: collection.name.clone(),
        description: collection.description.clone(),
        variables: collection.variables.clone(),
        request_order: request_order.clone(),
    };
    let manifest_json = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
    fs::write(folder.join("collection.json"), manifest_json)?;

    let mut keep_files = HashSet::new();
    for req in &collection.requests {
        let filename = format!("{}.json", safe_file_id(&req.id));
        let body = format!("{}\n", serde_json::to_string_pretty(req)?);
        fs::write(requests_dir.join(&filename), body)?;
        keep_files.insert(filename);
    }

    if let Ok(entries) = fs::read_dir(&requests_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") && !keep_files.contains(&name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    Ok(())
}

pub fn read_collection_folder(folder: &Path) -> Result<Collection> {
    let manifest_raw = fs::read_to_string(folder.join("collection.json"))
        .with_context(|| format!("reading collection.json in {}", folder.display()))?;
    let manifest: CollectionManifest = serde_json::from_str(&manifest_raw)?;
    let requests_dir = folder.join("requests");

    let mut requests = Vec::new();
    for id in manifest.request_order {
        let file = requests_dir.join(format!("{}.json", safe_file_id(&id)));
        if let Ok(raw) = fs::read_to_string(&file) {
            if let Ok(req) = serde_json::from_str::<ApiRequest>(&raw) {
                requests.push(req);
            }
        }
    }

    let name = if manifest.name.is_empty() {
        folder
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Collection")
            .to_string()
    } else {
        manifest.name
    };

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description: manifest.description,
        requests,
        variables: manifest.variables,
        folder_path: Some(folder.to_string_lossy().into_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HttpMethod;

    #[test]
    fn round_trips_collection_folder_and_prunes_deleted_requests() {
        let dir = std::env::temp_dir().join(format!("bapu_sync_test_{}", uuid::Uuid::new_v4()));
        let req1 = ApiRequest {
            id: "req-1".into(),
            name: "List Users".into(),
            method: HttpMethod::GET,
            url: "https://example.com/users".into(),
            ..Default::default()
        };
        let mut col = Collection {
            id: "col-1".into(),
            name: "Core API".into(),
            description: None,
            requests: vec![req1.clone()],
            variables: vec![],
            folder_path: Some(dir.to_string_lossy().into_owned()),
        };

        write_collection_folder(&dir, &col).unwrap();
        let loaded = read_collection_folder(&dir).unwrap();
        assert_eq!(loaded.name, "Core API");
        assert_eq!(loaded.requests.len(), 1);
        assert_eq!(loaded.requests[0].url, "https://example.com/users");

        col.requests.clear();
        write_collection_folder(&dir, &col).unwrap();
        assert!(!dir.join("requests").join("req-1.json").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
