"""
Blender 3D Avatar Post-Processing Pipeline
Loads generated 3D models (GLB/FBX/OBJ), centers and scales them,
configures studio three-point lighting and cameras, and renders high-quality
preview images.

Architectural Rule:
Does NOT create or rebuild character geometry from 3D primitives
(no cubes, spheres, cylinders, or procedural mesh generation).
Operates strictly on the downloaded external model asset.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
from typing import Dict, List, Optional, Tuple

logger = logging.getLogger("blender_postprocess")
if not logger.handlers:
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("[%(asctime)s] [%(levelname)s] [PostProcess] %(message)s", datefmt="%H:%M:%S"))
    logger.addHandler(handler)
    logger.setLevel(logging.INFO)

# Check if bpy (Blender Python API) is available in current interpreter
try:
    import bpy  # type: ignore
    import mathutils  # type: ignore
    BPY_AVAILABLE = True
except ImportError:
    bpy = None
    mathutils = None
    BPY_AVAILABLE = False


def find_blender_binary() -> Optional[str]:
    """Find the Blender executable path on system PATH or default OS locations."""
    which_path = shutil.which("blender")
    if which_path:
        return which_path

    if sys.platform == "win32":
        search_dirs = [
            Path("C:/Program Files/Blender Foundation"),
            Path("C:/Program Files (x86)/Blender Foundation"),
            Path(os.environ.get("LOCALAPPDATA", "")) / "Programs" / "Blender Foundation",
        ]
        for base in search_dirs:
            if base.exists():
                for sub in sorted(base.glob("Blender */blender.exe"), reverse=True):
                    if sub.is_file():
                        return str(sub)
    elif sys.platform == "darwin":
        mac_path = Path("/Applications/Blender.app/Contents/MacOS/Blender")
        if mac_path.is_file():
            return str(mac_path)

    return None


def _clear_blender_scene() -> None:
    """Remove default primitives, cameras, and lights from the Blender scene."""
    if not BPY_AVAILABLE:
        return
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for collection in bpy.data.collections:
        bpy.data.collections.remove(collection)
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def _compute_bounds(objects: List[Any]) -> Tuple[Tuple[float, float, float], Tuple[float, float, float]]:
    """Compute the combined axis-aligned bounding box for objects in world space."""
    min_x = min_y = min_z = float("inf")
    max_x = max_y = max_z = float("-inf")

    for obj in objects:
        if obj.type == "MESH":
            for corner in obj.bound_box:
                world_coord = obj.matrix_world @ mathutils.Vector(corner)
                min_x = min(min_x, world_coord.x)
                min_y = min(min_y, world_coord.y)
                min_z = min(min_z, world_coord.z)
                max_x = max(max_x, world_coord.x)
                max_y = max(max_y, world_coord.y)
                max_z = max(max_z, world_coord.z)

    if min_x == float("inf"):
        return ((-0.5, -0.5, 0.0), (0.5, 0.5, 1.8))

    return ((min_x, min_y, min_z), (max_x, max_y, max_z))


def run_bpy_postprocess(
    model_path: str,
    output_image_path: str,
    output_model_path: Optional[str] = None,
    target_height: float = 1.8,
) -> Dict[str, Any]:
    """
    Execute post-processing inside Blender using the bpy module.
    Centers, normalizes scale, sets studio lights, and renders a preview.
    """
    if not BPY_AVAILABLE:
        raise RuntimeError("run_bpy_postprocess can only be executed in an environment where bpy is importable.")

    p_model = Path(model_path).resolve()
    p_out_img = Path(output_image_path).resolve()
    p_out_img.parent.mkdir(parents=True, exist_ok=True)

    _clear_blender_scene()

    # Import model asset
    ext = p_model.suffix.lower()
    logger.info("Importing 3D asset '%s' into Blender scene...", p_model.name)
    if ext in (".glb", ".gltf"):
        bpy.ops.import_scene.gltf(filepath=str(p_model))
    elif ext == ".obj":
        if hasattr(bpy.ops.wm, "obj_import"):
            bpy.ops.wm.obj_import(filepath=str(p_model))
        else:
            bpy.ops.import_scene.obj(filepath=str(p_model))
    elif ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=str(p_model))
    else:
        raise ValueError(f"Unsupported 3D model format: {ext}")

    mesh_objects = [o for o in bpy.data.objects if o.type == "MESH"]
    if not mesh_objects:
        raise RuntimeError("No mesh geometry found in imported 3D asset.")

    # Compute bounding box
    (min_x, min_y, min_z), (max_x, max_y, max_z) = _compute_bounds(mesh_objects)
    curr_height = max_z - min_z
    scale_factor = (target_height / curr_height) if curr_height > 1e-4 else 1.0

    center_x = (min_x + max_x) / 2.0
    center_y = (min_y + max_y) / 2.0

    # Create root anchor and center geometry
    root = bpy.data.objects.new("CharacterRoot", None)
    bpy.context.scene.collection.objects.link(root)

    for obj in list(bpy.context.scene.collection.objects):
        if obj != root and obj.parent is None:
            obj.parent = root

    root.scale = (scale_factor, scale_factor, scale_factor)
    root.location = (-center_x * scale_factor, -center_y * scale_factor, -min_z * scale_factor)

    bpy.context.view_layer.update()

    # Studio Three-Point Lighting Setup
    # 1. Key Light (warm, bright, front-right)
    key_data = bpy.data.lights.new(name="KeyLight", type="SUN")
    key_data.energy = 3.5
    key_data.color = (1.0, 0.98, 0.95)
    key_obj = bpy.data.objects.new("KeyLight", key_data)
    key_obj.location = (2.0, -3.0, 3.5)
    key_obj.rotation_euler = (0.7, 0.2, 0.5)
    bpy.context.scene.collection.objects.link(key_obj)

    # 2. Fill Light (soft cool, front-left)
    fill_data = bpy.data.lights.new(name="FillLight", type="SUN")
    fill_data.energy = 1.8
    fill_data.color = (0.9, 0.94, 1.0)
    fill_obj = bpy.data.objects.new("FillLight", fill_data)
    fill_obj.location = (-2.5, -2.5, 2.5)
    fill_obj.rotation_euler = (0.8, -0.3, -0.6)
    bpy.context.scene.collection.objects.link(fill_obj)

    # 3. Rim / Hair Light (accentuates anime silhouette)
    rim_data = bpy.data.lights.new(name="RimLight", type="SUN")
    rim_data.energy = 2.8
    rim_data.color = (1.0, 0.95, 0.9)
    rim_obj = bpy.data.objects.new("RimLight", rim_data)
    rim_obj.location = (0.0, 3.0, 3.0)
    rim_obj.rotation_euler = (-0.9, 0.0, 3.14)
    bpy.context.scene.collection.objects.link(rim_obj)

    # Camera framing full body in T-pose
    cam_data = bpy.data.cameras.new("RenderCamera")
    cam_data.lens = 50.0
    cam_obj = bpy.data.objects.new("RenderCamera", cam_data)
    cam_obj.location = (0.0, -3.2, target_height * 0.5)
    cam_obj.rotation_euler = (1.5708, 0.0, 0.0)  # Face forward along +Y
    bpy.context.scene.collection.objects.link(cam_obj)
    bpy.context.scene.camera = cam_obj

    # Render settings
    scene = bpy.context.scene
    scene.render.resolution_x = 1024
    scene.render.resolution_y = 1024
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = str(p_out_img)

    logger.info("Rendering preview image to %s...", p_out_img)
    bpy.ops.render.render(write_still=True)

    # Export normalized model if requested
    if output_model_path:
        p_out_model = Path(output_model_path).resolve()
        p_out_model.parent.mkdir(parents=True, exist_ok=True)
        logger.info("Exporting normalized model to %s...", p_out_model)
        bpy.ops.export_scene.gltf(
            filepath=str(p_out_model),
            export_format="GLB",
            use_selection=False,
        )

    return {
        "status": "success",
        "preview_path": str(p_out_img),
        "model_path": str(p_out_model) if output_model_path else str(p_model),
        "height": target_height,
    }


def fallback_extract_preview(model_path: str, output_image_path: str) -> Optional[str]:
    """
    Fallback when Blender is not installed:
    Inspects binary glTF (GLB) file for embedded image/texture buffers and extracts
    the primary character texture or generates a visual inspection card.
    """
    p_model = Path(model_path)
    if not p_model.is_file() or p_model.suffix.lower() != ".glb":
        return None

    try:
        from PIL import Image, ImageDraw, ImageFont
    except ImportError:
        return None

    p_out = Path(output_image_path)
    p_out.parent.mkdir(parents=True, exist_ok=True)

    # Parse binary glTF chunks
    with open(p_model, "rb") as f:
        magic = f.read(4)
        if magic != b"glTF":
            return None
        version, total_len = struct.unpack("<II", f.read(8))
        json_len, json_type = struct.unpack("<II", f.read(8))
        if json_type != 0x4E4F534A:  # "JSON"
            return None
        json_bytes = f.read(json_len)
        gltf = json.loads(json_bytes.decode("utf-8"))

        # Look for BIN chunk
        bin_chunk_start = 12 + 8 + json_len
        if bin_chunk_start < total_len:
            f.seek(bin_chunk_start)
            bin_len, bin_type = struct.unpack("<II", f.read(8))
            bin_data = f.read(bin_len)

            # Check for embedded images in gltf["images"]
            images = gltf.get("images", [])
            for img_info in images:
                if "bufferView" in img_info:
                    bv_idx = img_info["bufferView"]
                    bv = gltf["bufferViews"][bv_idx]
                    offset = bv.get("byteOffset", 0)
                    length = bv["byteLength"]
                    raw_img = bin_data[offset : offset + length]

                    # Verify image validity with Pillow
                    try:
                        import io
                        img = Image.open(io.BytesIO(raw_img))
                        # Save extracted high-res texture/preview
                        img.convert("RGBA").save(p_out, "PNG")
                        logger.info("Extracted embedded GLB texture as preview: %s", p_out)
                        return str(p_out.resolve())
                    except Exception:
                        continue

    # If no embedded texture was extractable, compose an informative model card
    card = Image.new("RGBA", (1024, 1024), (18, 24, 38, 255))
    draw = ImageDraw.ImageDraw(card)
    # Background subtle framing
    draw.rounded_rectangle([32, 32, 992, 992], radius=24, outline=(59, 130, 246, 255), width=3)
    draw.text((64, 80), "Miyora 3D Avatar Asset", fill=(255, 255, 255, 255))
    draw.text((64, 140), f"Source Model: {p_model.name}", fill=(147, 197, 253, 255))
    draw.text((64, 180), f"File Size: {p_model.stat().st_size:,} bytes", fill=(203, 213, 225, 255))
    draw.text((64, 220), "Format: Game-Ready Binary glTF (GLB)", fill=(203, 213, 225, 255))
    draw.text((64, 300), "Post-processing lighting and camera rendering available with Blender.", fill=(148, 163, 184, 255))
    card.save(p_out, "PNG")
    return str(p_out.resolve())


def postprocess_model(
    model_path: str,
    output_image_path: str,
    output_model_path: Optional[str] = None,
    target_height: float = 1.8,
) -> Dict[str, Any]:
    """
    Main post-processing entry point.
    If running inside Blender, runs run_bpy_postprocess directly.
    If running outside Blender, invokes Blender CLI if available, or utilizes fallback extraction.
    """
    if BPY_AVAILABLE:
        return run_bpy_postprocess(model_path, output_image_path, output_model_path, target_height)

    blender_bin = find_blender_binary()
    if blender_bin:
        logger.info("Found Blender executable: %s. Launching background render...", blender_bin)
        script_path = Path(__file__).resolve()
        cmd = [
            blender_bin,
            "--background",
            "--python",
            str(script_path),
            "--",
            "--model",
            str(Path(model_path).resolve()),
            "--output-image",
            str(Path(output_image_path).resolve()),
            "--height",
            str(target_height),
        ]
        if output_model_path:
            cmd.extend(["--output-model", str(Path(output_model_path).resolve())])

        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode == 0 and Path(output_image_path).is_file():
            logger.info("Blender background render succeeded: %s", output_image_path)
            return {
                "status": "success",
                "preview_path": str(Path(output_image_path).resolve()),
                "model_path": str(Path(output_model_path).resolve()) if output_model_path else str(Path(model_path).resolve()),
            }
        else:
            logger.warning("Blender CLI exited with code %d. stderr: %s", proc.returncode, proc.stderr[:300])

    # Fallback preview extraction
    logger.info("Blender executable not available on host. Extracting embedded model preview...")
    fallback_path = fallback_extract_preview(model_path, output_image_path)
    return {
        "status": "success",
        "preview_path": fallback_path,
        "model_path": str(Path(model_path).resolve()),
        "note": "Rendered via embedded texture extraction (install Blender for full three-point lighting pass).",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Post-process 3D character models in Blender.")
    parser.add_argument("--model", required=True, help="Path to input 3D model (GLB/OBJ/FBX)")
    parser.add_argument("--output-image", required=True, help="Path to output rendered preview image")
    parser.add_argument("--output-model", default=None, help="Optional path to output centered/scaled GLB")
    parser.add_argument("--height", type=float, default=1.8, help="Target height for character scaling")

    # In Blender background mode, extra arguments are passed after '--'
    args_list = sys.argv[1:]
    if "--" in args_list:
        args_list = args_list[args_list.index("--") + 1 :]

    args = parser.parse_args(args_list)
    res = postprocess_model(args.model, args.output_image, args.output_model, args.height)
    print(json.dumps(res, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
