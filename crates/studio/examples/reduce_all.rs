//! Runs local retopology on every generated model in a studio library (no mocks).
//!
//!     cargo run --release -p agentpet-studio --example reduce_all -- <data-dir>

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use agentpet_studio::credentials::KeyringStore;
use agentpet_studio::local::{LocalEngine, LocalEngineConfig};
use agentpet_studio::service::{
    provider_factory, RetopologyRequest, StudioService, RETOPOLOGY_OPERATION, STUDIO_DIR,
};
use agentpet_studio::transport::{HttpConfig, ReqwestTransport};

const BUDGET: u64 = 5_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: reduce_all <data-dir>")?,
    );
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
                if generation.operation == RETOPOLOGY_OPERATION {
                    continue;
                }
                let Some(triangles) = generation
                    .outputs
                    .iter()
                    .find(|output| output.kind == "model")
                    .and_then(|output| output.report.as_ref()?.get("triangles")?.as_u64())
                else {
                    continue;
                };
                for target in [triangles / 4, BUDGET.min(triangles - 1)] {
                    let started = Instant::now();
                    let request = RetopologyRequest {
                        generation_id: generation.id.clone(),
                        target_triangles: target as usize,
                        preserve_borders: true,
                    };
                    match studio.retopologize(request) {
                        Ok(task) => {
                            let child = studio
                                .asset_detail(&task.asset_id)?
                                .generations
                                .into_iter()
                                .find(|entry| entry.id == task.generation_id)
                                .ok_or("missing child")?;
                            println!(
                                "ok   {:<14} {triangles:>7} -> target {target:>6} -> {} tris in {:.2}s (v{})",
                                generation.provider,
                                child.settings["resultTriangles"],
                                started.elapsed().as_secs_f32(),
                                child.version_number.unwrap_or(0)
                            );
                        }
                        Err(error) => {
                            println!("fail {:<14} target {target}: {error}", generation.provider)
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
