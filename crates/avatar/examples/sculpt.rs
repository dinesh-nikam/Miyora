//! Developer tool: sculpt an image into a GLB and print the derived spec.
//!
//! Usage:
//!   cargo run -p agentpet-avatar --example sculpt -- <image> <output.glb> [name]
//!
//! The GLB can be inspected in any glTF viewer (for example Blender or
//! https://gltf-viewer.donmccurdy.com). Exit status is 2 for usage errors and 1
//! when sculpting fails.

use std::{path::PathBuf, process::ExitCode};

use agentpet_avatar::{sculpt_image, ImageSculptLimits};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        eprintln!("usage: sculpt <image> <output.glb> [name]");
        return ExitCode::from(2);
    };
    let name = args.get(2).map_or("Sculpt", String::as_str);
    let result = sculpt_image(
        &PathBuf::from(input),
        "user.style-example",
        name,
        ImageSculptLimits::default(),
    );
    let output_data = match result {
        Ok(data) => data,
        Err(error) => {
            eprintln!("sculpt failed: {error}");
            return ExitCode::from(1);
        }
    };
    if let Err(error) = std::fs::write(output, &output_data.glb) {
        eprintln!("could not write {output}: {error}");
        return ExitCode::from(1);
    }
    match serde_json::to_string_pretty(&serde_json::json!({
        "stats": output_data.stats,
        "glbBytes": output_data.glb.len(),
        "spec": output_data.spec,
    })) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("could not format report: {error}");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}
