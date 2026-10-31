//! Real end-to-end smoke run through `StudioService` (no mocks).
//!
//!     cargo run -p agentpet-studio --example generate -- <provider-id> <image> [data-dir]
//!
//! provider-id: local-triposr | hf-trellis | tripo. Uses the real engines/network
//! and the OS keyring for keys. Prints the final task and the stored model report.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use agentpet_studio::credentials::KeyringStore;
use agentpet_studio::local::{LocalEngine, LocalEngineConfig};
use agentpet_studio::provider::GenerationSettings;
use agentpet_studio::service::{provider_factory, ImageSubmission, StudioService, STUDIO_DIR};
use agentpet_studio::transport::{HttpConfig, ReqwestTransport};

const WAIT_LIMIT: Duration = Duration::from_secs(40 * 60);
const REFRESH: Duration = Duration::from_secs(3);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let provider = args
        .next()
        .ok_or("usage: generate <provider-id> <image> [data-dir]")?;
    let image = PathBuf::from(args.next().ok_or("missing image path")?);
    let data_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("miyora-smoke"));

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
    let project = studio.list_projects()?.remove(0).id;
    let task = studio.submit_image_to_3d(ImageSubmission {
        project_id: project,
        asset_id: None,
        asset_name: None,
        provider: provider.clone(),
        image_path: image.to_string_lossy().into_owned(),
        settings: GenerationSettings::default(),
        parent_version_id: None,
    })?;
    println!("submitted task {} ({provider})", task.id);

    let started = Instant::now();
    let mut last = String::new();
    let done = loop {
        let view = studio
            .list_tasks()?
            .into_iter()
            .find(|entry| entry.id == task.id)
            .ok_or("task vanished")?;
        let now = format!("{:?} {:?}", view.state, view.stage);
        if now != last {
            println!("[{:>5.0}s] {now}", started.elapsed().as_secs_f32());
            last = now;
        }
        if view.state.is_terminal() {
            break view;
        }
        if started.elapsed() > WAIT_LIMIT {
            return Err("timed out waiting for the task".into());
        }
        std::thread::sleep(REFRESH);
    };
    println!(
        "final: {:?} error={:?}",
        done.state,
        done.error.as_ref().map(|error| &error.message)
    );
    let detail = studio.asset_detail(&done.asset_id)?;
    for output in &detail.generations[0].outputs {
        let report = output
            .report
            .as_ref()
            .map(|report| report.to_string())
            .unwrap_or_default();
        println!(
            "output {} {} {} bytes report={report}",
            output.label, output.format, output.byte_size
        );
    }
    println!("data dir: {}", data_dir.display());
    Ok(())
}
