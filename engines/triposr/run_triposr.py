"""Miyora local engine runner for TripoSR (MIT, Tripo AI & Stability AI).

Invoked only by the Miyora desktop app with fixed arguments:

    python -I run_triposr.py --job-dir <dir> [--device auto|cuda|cpu] ...

Reads   <job-dir>/input.png
Writes  <job-dir>/status.json   {"state","stage","device","error","outputs","seconds"}
        <job-dir>/model.glb      generated mesh (y-up, glTF)
        <job-dir>/processed_input.png   background-removed input actually used

The process never contacts the network: weights must already be cached by
setup_engine.py (HF_HUB_OFFLINE is forced on).
"""

import argparse
import json
import os
import sys
import time
import traceback
from pathlib import Path

ENGINE_ROOT = Path(__file__).resolve().parent
os.environ["HF_HUB_OFFLINE"] = "1"
os.environ.setdefault("HF_HOME", str(ENGINE_ROOT / "hf"))
os.environ.setdefault("U2NET_HOME", str(ENGINE_ROOT / "u2net"))
sys.path[:0] = [str(ENGINE_ROOT / "shim"), str(ENGINE_ROOT / "src")]

MODEL_ID = "stabilityai/TripoSR"


def write_status(job_dir: Path, payload: dict) -> None:
    partial = job_dir / "status.json.part"
    partial.write_text(json.dumps(payload), encoding="utf-8")
    os.replace(partial, job_dir / "status.json")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--job-dir", required=True)
    parser.add_argument("--device", choices=["auto", "cuda", "cpu"], default="auto")
    parser.add_argument("--chunk-size", type=int, default=4096)
    parser.add_argument("--mc-resolution", type=int, default=256)
    parser.add_argument("--foreground-ratio", type=float, default=0.85)
    parser.add_argument("--no-remove-bg", action="store_true")
    parser.add_argument("--bake-texture", action="store_true")
    parser.add_argument("--texture-resolution", type=int, default=1024)
    args = parser.parse_args()
    if not 1024 <= args.chunk_size <= 65536:
        parser.error("chunk size out of range")
    if not 64 <= args.mc_resolution <= 512:
        parser.error("marching cubes resolution out of range")
    if not 0.5 <= args.foreground_ratio <= 1.0:
        parser.error("foreground ratio out of range")
    if not 256 <= args.texture_resolution <= 4096:
        parser.error("texture resolution out of range")
    return args


def main() -> int:
    args = parse_args()
    job_dir = Path(args.job_dir).resolve()
    started = time.monotonic()
    status = {"state": "running", "stage": "loading_model", "device": None, "error": None, "outputs": {}}
    write_status(job_dir, status)

    def stage(name: str) -> None:
        status["stage"] = name
        write_status(job_dir, status)

    try:
        import numpy as np
        import torch
        import trimesh
        from PIL import Image

        from tsr.system import TSR
        from tsr.utils import remove_background, resize_foreground

        device = "cuda" if args.device != "cpu" and torch.cuda.is_available() else "cpu"
        if args.device == "cuda" and device != "cuda":
            raise RuntimeError("CUDA was requested but no CUDA GPU is available.")
        status["device"] = device

        model = TSR.from_pretrained(MODEL_ID, config_name="config.yaml", weight_name="model.ckpt")
        model.renderer.set_chunk_size(args.chunk_size)
        model.to(device)

        stage("preparing_image")
        image = Image.open(job_dir / "input.png")
        if args.no_remove_bg:
            image = image.convert("RGB")
        else:
            import rembg

            image = remove_background(image, rembg.new_session())
            image = resize_foreground(image, args.foreground_ratio)
            rgba = np.array(image).astype(np.float32) / 255.0
            rgb = rgba[:, :, :3] * rgba[:, :, 3:4] + (1 - rgba[:, :, 3:4]) * 0.5
            image = Image.fromarray((rgb * 255.0).astype(np.uint8))
        image.save(job_dir / "processed_input.png")

        stage("inference")
        with torch.no_grad():
            scene_codes = model([image], device=device)

        stage("extracting_mesh")
        mesh = model.extract_mesh(scene_codes, not args.bake_texture, resolution=args.mc_resolution)[0]
        if len(mesh.faces) == 0:
            raise RuntimeError("The model produced an empty mesh; try another image.")

        if args.bake_texture:
            stage("baking_texture")
            from tsr.bake_texture import bake_texture

            baked = bake_texture(mesh, model, scene_codes[0], args.texture_resolution)
            texture = Image.fromarray((baked["colors"] * 255.0).astype(np.uint8)).transpose(Image.FLIP_TOP_BOTTOM)
            mesh = trimesh.Trimesh(
                vertices=mesh.vertices[baked["vmapping"]],
                faces=baked["indices"].astype(np.int64),
                visual=trimesh.visual.TextureVisuals(uv=baked["uvs"], image=texture),
                process=False,
            )

        stage("exporting")
        # Outward-facing normals regardless of marching-cubes winding.
        if mesh.is_volume and mesh.volume < 0:
            mesh.invert()
        # TripoSR is z-up; glTF is y-up (same transform as TripoSR's own viewer).
        mesh.apply_transform(trimesh.transformations.rotation_matrix(-np.pi / 2, [1, 0, 0]))
        mesh.apply_transform(trimesh.transformations.rotation_matrix(np.pi / 2, [0, 1, 0]))
        mesh.export(job_dir / "model.glb")

        status.update(
            state="succeeded",
            stage=None,
            outputs={"model": "model.glb", "preview": "processed_input.png"},
            seconds=round(time.monotonic() - started, 2),
        )
        write_status(job_dir, status)
        return 0
    except Exception as error:  # noqa: BLE001 - reported to the app, never swallowed
        message = str(error) or error.__class__.__name__
        if "out of memory" in message.lower():
            message = "GPU ran out of memory. Lower the chunk size or use the CPU device."
        status.update(state="failed", error=message[:500], seconds=round(time.monotonic() - started, 2))
        write_status(job_dir, status)
        traceback.print_exc(file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
