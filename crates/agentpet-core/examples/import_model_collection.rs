//! Validated, hash-deduplicated local model collection import through the Rust core.
use agentpet_core::{AvatarSource, Core};
use agentpet_pet_pack::vrm::{validate_and_extract_vrm, VrmGuardrails};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Collection {
    schema_version: u32,
    style: String,
    characters: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    id: String,
    name: String,
    body: String,
    model: String,
    source: String,
    sha256: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Imported {
    catalog_id: String,
    avatar_id: String,
    sha256: String,
}

fn relative_model(path: &str) -> Result<&Path, String> {
    if path.contains(['\\', ':']) {
        return Err("model path contains forbidden separators".into());
    }
    let p = Path::new(path);
    if p.components().count() != 2
        || !p.components().all(|c| matches!(c, Component::Normal(_)))
        || p.extension().and_then(|s| s.to_str()) != Some("vrm")
    {
        return Err("model must be a relative character/model.vrm path".into());
    }
    Ok(p)
}

fn run() -> Result<(), String> {
    let mut root = None;
    let mut data = None;
    let mut select = None;
    let mut dry_run = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--models" => {
                root = Some(PathBuf::from(
                    args.next().ok_or("--models needs a directory")?,
                ))
            }
            "--data-dir" => {
                data = Some(PathBuf::from(
                    args.next().ok_or("--data-dir needs a directory")?,
                ))
            }
            "--select" => select = Some(args.next().ok_or("--select needs a catalog ID")?),
            "--dry-run" => dry_run = true,
            "--help" | "-h" => {
                println!("import_model_collection --models DIR [--dry-run | --data-dir DIR] [--select CATALOG_ID]");
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let root = root
        .ok_or("--models is required")?
        .canonicalize()
        .map_err(|_| "model directory unavailable")?;
    let manifest = root.join("collection.json");
    if fs::metadata(&manifest)
        .map_err(|_| "collection manifest unavailable")?
        .len()
        > 1024 * 1024
    {
        return Err("collection manifest exceeds 1 MiB".into());
    }
    let collection: Collection =
        serde_json::from_slice(&fs::read(manifest).map_err(|_| "collection manifest unavailable")?)
            .map_err(|_| "invalid collection manifest")?;
    if collection.schema_version != 1
        || collection.style != "full-anime"
        || collection.characters.is_empty()
        || collection.characters.len() > 512
    {
        return Err("unsupported or empty model collection".into());
    }
    if let Some(id) = &select {
        if !collection.characters.iter().any(|e| &e.id == id) {
            return Err("selected character absent from collection".into());
        }
    }
    // Validate the complete collection before changing the application library.
    let mut ids = HashSet::new();
    for entry in &collection.characters {
        if entry.name.is_empty()
            || entry.name.chars().count() > 32
            || entry.name.chars().any(char::is_control)
            || entry.id.len() > 64
            || !ids.insert(&entry.id)
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || !matches!(
                entry.body.as_str(),
                "humanoid" | "beast" | "bird" | "robot" | "slime"
            )
            || !entry.source.ends_with("source.blend")
        {
            return Err("invalid character manifest fields".into());
        }
        let model = root
            .join(relative_model(&entry.model)?)
            .canonicalize()
            .map_err(|_| format!("missing model: {}", entry.id))?;
        if !model.starts_with(&root) {
            return Err("model escapes collection directory".into());
        }
        validate_and_extract_vrm(&model, VrmGuardrails::default())
            .map_err(|_| format!("native validation failed: {}", entry.id))?;
        let hash = format!(
            "{:x}",
            Sha256::digest(fs::read(&model).map_err(|_| "model read failed")?)
        );
        if hash != entry.sha256 {
            return Err(format!("model hash mismatch: {}", entry.id));
        }
    }
    println!(
        "Validated {} models with the native Rust guardrails",
        collection.characters.len()
    );
    if dry_run {
        return Ok(());
    }
    let data = data.ok_or("--data-dir is required unless --dry-run is used")?;
    let core = Core::open(&data).map_err(|e| e.to_string())?;
    let mut by_hash: HashMap<String, String> = core
        .list_custom_avatars()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|a| a.model_sha256.map(|hash| (hash, a.avatar_id)))
        .collect();
    let mut imported = Vec::new();
    let mut selected = None;
    for entry in &collection.characters {
        let avatar_id = if let Some(id) = by_hash.get(&entry.sha256) {
            id.clone()
        } else {
            let model = root.join(relative_model(&entry.model)?);
            let result = core
                .import_vrm_avatar(&model, &data.join("avatars"))
                .map_err(|e| format!("import failed for {}: {e}", entry.id))?;
            if result.model_sha256.as_deref() != Some(entry.sha256.as_str()) {
                core.delete_custom_avatar(&result.avatar_id, &data.join("avatars"))
                    .map_err(|_| "changed source model could not be rolled back")?;
                return Err(format!("model changed during import: {}", entry.id));
            }
            by_hash.insert(entry.sha256.clone(), result.avatar_id.clone());
            result.avatar_id
        };
        if select.as_ref() == Some(&entry.id) {
            selected = Some(avatar_id.clone());
        }
        imported.push(Imported {
            catalog_id: entry.id.clone(),
            avatar_id,
            sha256: entry.sha256.clone(),
        });
    }
    if let Some(id) = selected {
        core.select_avatar(AvatarSource::Custom, &id)
            .map_err(|e| e.to_string())?;
        if let Some(catalog_id) = &select {
            let pack_id = format!("agentpet.{}", catalog_id.rsplit('.').next().unwrap_or(""));
            if let Some(pack) = core
                .list_installed_packs()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|pack| pack.id == pack_id)
            {
                core.set_selected_pet(Some(&pack.selection_id))
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    fs::write(
        root.join("app-import-index.json"),
        serde_json::to_vec_pretty(&imported).map_err(|e| e.to_string())?,
    )
    .map_err(|_| "could not save import index")?;
    println!("Available in the local library: {} models", imported.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Collection import failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_paths_are_confined_to_one_character_directory() {
        assert!(relative_model("nezuko/model.vrm").is_ok());
        for p in [
            "../model.vrm",
            "/nezuko/model.vrm",
            "C:/model.vrm",
            "nezuko\\model.vrm",
            "nezuko/nested/model.vrm",
            "nezuko/model.glb",
        ] {
            assert!(relative_model(p).is_err(), "accepted {p}");
        }
    }
}
