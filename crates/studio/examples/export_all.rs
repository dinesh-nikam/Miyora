//! Exports every model in a studio library to Original/OBJ/STL/PLY (no mocks).
//!
//!     cargo run -p agentpet-studio --example export_all -- <data-dir> <out-dir>

use std::path::PathBuf;
use std::sync::Arc;

use agentpet_studio::credentials::KeyringStore;
use agentpet_studio::export::{ExportOptions, LocalExportFormat};
use agentpet_studio::local::{LocalEngine, LocalEngineConfig};
use agentpet_studio::service::{provider_factory, StudioService, STUDIO_DIR};
use agentpet_studio::transport::{HttpConfig, ReqwestTransport};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let data_dir = PathBuf::from(
        args.next()
            .ok_or("usage: export_all <data-dir> <out-dir>")?,
    );
    let out_dir = PathBuf::from(args.next().ok_or("missing out dir")?);
    std::fs::create_dir_all(&out_dir)?;
    let transport = Arc::new(
        ReqwestTransport::new(HttpConfig::default()).map_err(|error| format!("{error:?}"))?,
    );
    let local = LocalEngine::new(
        LocalEngine::default_engine_root(),
        data_dir.join(STUDIO_DIR).join("local-jobs"),
        LocalEngineConfig::default(),
    );
    let studio = StudioService::open(
        &data_dir,
        Arc::new(KeyringStore),
        provider_factory(transport, local, data_dir.join(STUDIO_DIR).join("hf-jobs")),
    )?;
    for project in studio.list_projects()? {
        for asset in studio.list_assets(&project.id)? {
            for generation in studio.asset_detail(&asset.id)?.generations {
                let Some(model) = generation
                    .outputs
                    .iter()
                    .find(|output| output.kind == "model")
                else {
                    continue;
                };
                for format in [
                    LocalExportFormat::Original,
                    LocalExportFormat::Obj,
                    LocalExportFormat::Stl,
                    LocalExportFormat::Ply,
                ] {
                    let stem = format!("{}-{}", generation.provider, &generation.id[..8]);
                    let result = studio.export_output(
                        model.id,
                        format,
                        ExportOptions::default(),
                        &out_dir.join(stem),
                    );
                    match result {
                        Ok(view) => println!(
                            "ok   {:<14} {:<8} {:>9} bytes tris={:?} files={}",
                            generation.provider,
                            format.as_str(),
                            view.byte_size,
                            view.triangles,
                            view.files.len()
                        ),
                        Err(error) => println!(
                            "fail {:<14} {:<8} {error}",
                            generation.provider,
                            format.as_str()
                        ),
                    }
                }
            }
        }
    }
    Ok(())
}
