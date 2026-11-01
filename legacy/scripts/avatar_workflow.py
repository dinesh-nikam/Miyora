"""
Main Avatar Generation Agent Workflow
Orchestrates prompt building, Tripo 3D API submission, task polling,
asset downloading to /output/models/, and optional Blender post-processing.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
from pathlib import Path
import sys
from typing import Any, Dict, Optional

from anime_character_prompt import build_anime_prompt
from blender_postprocess import postprocess_model
from tripo_client import (
    TripoAuthError,
    TripoCreditError,
    TripoError,
    check_task_status,
    download_model,
    generate_image_to_3d,
    generate_text_to_3d,
    poll_task_until_complete,
)

logger = logging.getLogger("avatar_workflow")
if not logger.handlers:
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("[%(asctime)s] [%(levelname)s] [AvatarAgent] %(message)s", datefmt="%H:%M:%S"))
    logger.addHandler(handler)
    logger.setLevel(logging.INFO)


def run_avatar_generation(
    prompt: Optional[str] = None,
    image_path_or_url: Optional[str] = None,
    output_dir: str = "output/models",
    timeout: float = 600.0,
    poll_interval: float = 3.0,
    apply_postprocessing: bool = True,
) -> Dict[str, Any]:
    """
    Execute the complete avatar generation pipeline:
    1. Accept user description or uploaded reference image.
    2. Build the final generation prompt (strictly original, no copyrighted characters).
    3. Call Tripo 3D API (Text-to-3D or Image-to-3D).
    4. Poll the task until completed.
    5. Download the resulting GLB/FBX/OBJ model to output_dir.
    6. Run post-processing (center, scale, light, render preview).
    7. Return model path, preview image path, and task metadata.

    Returns:
        Dict containing model_path, preview_image_path, task_id, status, and error details (if any).
    """
    if not prompt and not image_path_or_url:
        raise ValueError("Must provide either a text description (--prompt) or a reference image (--image).")

    out_dir_path = Path(output_dir).resolve()
    out_dir_path.mkdir(parents=True, exist_ok=True)

    result: Dict[str, Any] = {
        "status": "pending",
        "task_id": None,
        "model_path": None,
        "preview_image_path": None,
        "final_prompt": None,
        "error": None,
    }

    try:
        # Step 1 & 2: Build generation prompt / prepare inputs
        if image_path_or_url:
            logger.info("Executing Image-to-3D pipeline with reference: %s", image_path_or_url)
            task_resp = generate_image_to_3d(image_path_or_url)
        else:
            assert prompt is not None
            logger.info("Raw input prompt: %s", prompt)
            final_prompt = build_anime_prompt(prompt)
            result["final_prompt"] = final_prompt
            logger.info("Engineered anime 3D prompt: %s", final_prompt)

            # Step 3: Call Tripo Text-to-3D endpoint
            task_resp = generate_text_to_3d(final_prompt)

        task_id = task_resp.get("task_id")
        result["task_id"] = task_id
        logger.info("Tripo task initiated. ID: %s", task_id)

        # Step 4: Poll task until completed
        logger.info("Polling Tripo task %s until completion (timeout: %.0fs)...", task_id, timeout)
        poll_task_until_complete(task_id, timeout=timeout, poll_interval=poll_interval)

        # Step 5: Download the resulting 3D model
        logger.info("Downloading generated 3D model into %s...", out_dir_path)
        download_info = download_model(task_id, output_dir=str(out_dir_path))
        result["model_path"] = download_info.get("model_path")
        result["preview_image_path"] = download_info.get("preview_path")

        # Step 6: Post-processing (load model, center, scale, studio light, render preview)
        if apply_postprocessing and result["model_path"]:
            logger.info("Running post-processing pipeline on %s...", result["model_path"])
            preview_target = out_dir_path / f"{task_id}_rendered.png"
            post_res = postprocess_model(
                model_path=result["model_path"],
                output_image_path=str(preview_target),
                target_height=1.8,
            )
            if post_res.get("preview_path"):
                result["preview_image_path"] = post_res["preview_path"]

        result["status"] = "success"
        logger.info("Avatar generation workflow completed successfully!")
        return result

    except TripoAuthError as err:
        logger.error("Authentication error: %s", err)
        result["status"] = "failed"
        result["error"] = str(err)
        return result
    except TripoCreditError as err:
        logger.error("Credits error: %s", err)
        result["status"] = "failed"
        result["error"] = str(err)
        return result
    except TripoError as err:
        logger.error("Tripo pipeline error: %s", err)
        result["status"] = "failed"
        result["error"] = str(err)
        return result
    except Exception as err:
        logger.error("Unexpected workflow error: %s", err, exc_info=True)
        result["status"] = "failed"
        result["error"] = str(err)
        return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Miyora 3D Avatar Generation Workflow (powered by Tripo 3D and Blender)."
    )
    parser.add_argument(
        "--prompt",
        type=str,
        default=None,
        help="Text prompt describing the desired anime character.",
    )
    parser.add_argument(
        "--image",
        type=str,
        default=None,
        help="Path or URL to a reference character image for Image-to-3D generation.",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="output/models",
        help="Directory where generated 3D models and previews are saved. Default: output/models",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=600.0,
        help="Maximum polling timeout in seconds. Default: 600",
    )
    parser.add_argument(
        "--no-postprocess",
        action="store_true",
        help="Skip post-processing pass (centering, scaling, lighting).",
    )

    args = parser.parse_args()

    if not args.prompt and not args.image:
        parser.print_help()
        return 1

    res = run_avatar_generation(
        prompt=args.prompt,
        image_path_or_url=args.image,
        output_dir=args.output_dir,
        timeout=args.timeout,
        apply_postprocessing=not args.no_postprocess,
    )

    print("\n" + "=" * 60)
    print("AVATAR GENERATION WORKFLOW RESULT:")
    print("=" * 60)
    print(f"Status:             {res.get('status')}")
    print(f"Task ID:            {res.get('task_id')}")
    print(f"Model Path:         {res.get('model_path')}")
    print(f"Preview Image Path: {res.get('preview_image_path')}")
    if res.get("error"):
        print(f"Errors:             {res.get('error')}")
    print("=" * 60 + "\n")

    return 0 if res.get("status") == "success" else 1


if __name__ == "__main__":
    sys.exit(main())
