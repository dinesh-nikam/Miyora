#!/usr/bin/env python3
"""
Production Free Image Generation Pipeline for AgentPet Desktop Companions.
Integrates FLUX.1 [schnell] (Hugging Face) and Google AI Studio Imagen 3.
Applies automated perimeter flood-fill matting to produce clean RGBA transparent assets.
"""

from __future__ import annotations

import argparse
import base64
from collections import deque
import io
import json
import logging
import os
from pathlib import Path
import sys
import time
from typing import Optional
import urllib.error
import urllib.request

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("companion-generator")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate desktop pet companion assets via free top-notch engines."
    )
    parser.add_argument(
        "--engine",
        choices=["flux", "gemini", "pollinations"],
        default="flux",
        help="Generation engine: 'flux' (Hugging Face FLUX.1), 'gemini' (Google AI Studio), or 'pollinations'.",
    )
    parser.add_argument(
        "--character-id",
        type=str,
        default="krishna",
        help="Target character directory under pet-packs/ (e.g., krishna, radha, gojo).",
    )
    parser.add_argument(
        "--hf-token",
        type=str,
        default=os.environ.get("HF_TOKEN") or os.environ.get("HUGGING_FACE_HUB_TOKEN") or "",
        help="Hugging Face user access token for FLUX.1. Defaults to $HF_TOKEN.",
    )
    parser.add_argument(
        "--gemini-key",
        type=str,
        default=os.environ.get("GEMINI_API_KEY") or "",
        help="Google AI Studio API key. Defaults to $GEMINI_API_KEY.",
    )
    parser.add_argument(
        "--prompt-override",
        type=str,
        default=None,
        help="Custom prompt string. If omitted, reads prompt from character's prompts.json.",
    )
    parser.add_argument(
        "--size",
        type=int,
        default=512,
        help="Target square dimension for pet pack assets (default 512).",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Destination directory for output assets. Defaults to pet-packs/<character-id>/assets.",
    )
    return parser.parse_args()


def load_character_prompt(character_id: str, repo_root: Path) -> str:
    prompts_path = repo_root / "pet-packs" / character_id / "prompts.json"
    if prompts_path.is_file():
        try:
            with open(prompts_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                idle = data.get("states", {}).get("idle", {})
                return idle.get("prompt") or data.get("master_prompt") or ""
        except Exception as e:
            logger.warning("Failed parsing %s: %s", prompts_path, e)
    return (
        f"masterpiece, highest quality, cute 2.5D anime chibi {character_id.title()} desktop companion, "
        "compact proportions, large expressive anime eyes, detailed clothing, subtle rim lighting, "
        "isolated on solid pure white background, full body standing pose, no background elements"
    )


def generate_flux_hf(prompt: str, token: str) -> Image.Image:
    if not token:
        raise ValueError(
            "Hugging Face token is required for FLUX.1 [schnell]. "
            "Get a free token at https://huggingface.co/settings/tokens and pass --hf-token or set $HF_TOKEN."
        )

    api_url = "https://router.huggingface.co/hf-inference/models/black-forest-labs/FLUX.1-schnell"
    payload = json.dumps({"inputs": prompt, "parameters": {"num_inference_steps": 4}}).encode("utf-8")
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
        "User-Agent": "AgentPet-Generator/1.0",
    }

    req = urllib.request.Request(api_url, data=payload, headers=headers, method="POST")
    logger.info("Dispatching FLUX.1 [schnell] request to Hugging Face...")

    for attempt in range(1, 6):
        try:
            with urllib.request.urlopen(req, timeout=90) as response:
                img_data = response.read()
                return Image.open(io.BytesIO(img_data)).convert("RGB")
        except urllib.error.HTTPError as err:
            body = err.read().decode("utf-8", errors="replace")
            if err.code == 503 and "estimated_time" in body:
                try:
                    wait_time = min(float(json.loads(body).get("estimated_time", 15)), 30.0)
                except Exception:
                    wait_time = 15.0
                logger.info("Model loading on Hugging Face. Retrying in %.1fs (attempt %d/5)...", wait_time, attempt)
                time.sleep(wait_time)
                continue
            raise RuntimeError(f"Hugging Face API error {err.code}: {body}") from err

    raise TimeoutError("Hugging Face serverless model initialization timed out.")


def generate_gemini_imagen(prompt: str, api_key: str) -> Image.Image:
    if not api_key:
        raise ValueError(
            "Gemini API key is required. "
            "Get a free key at https://aistudio.google.com and pass --gemini-key or set $GEMINI_API_KEY."
        )

    api_url = f"https://generativelanguage.googleapis.com/v1beta/models/imagen-3.0-generate-002:predict?key={api_key}"
    payload = json.dumps({
        "instances": [{"prompt": prompt}],
        "parameters": {"sampleCount": 1, "aspectRatio": "1:1", "personGeneration": "ALLOW_ADULT"},
    }).encode("utf-8")
    headers = {"Content-Type": "application/json"}

    req = urllib.request.Request(api_url, data=payload, headers=headers, method="POST")
    logger.info("Dispatching Imagen 3 request to Google AI Studio API...")

    with urllib.request.urlopen(req, timeout=60) as response:
        result = json.loads(response.read().decode("utf-8"))

    predictions = result.get("predictions", [])
    if not predictions or "bytesBase64Encoded" not in predictions[0]:
        raise RuntimeError(f"Unexpected Google AI Studio response: {result}")

    raw_bytes = base64.b64decode(predictions[0]["bytesBase64Encoded"])
    return Image.open(io.BytesIO(raw_bytes)).convert("RGB")


def remove_solid_background(img: Image.Image, color_tolerance: float = 38.0) -> Image.Image:
    """Extract transparent character using 8-directional perimeter flood-fill matting."""
    rgba = img.convert("RGBA")
    arr = np.array(rgba)
    h, w = arr.shape[:2]

    # Sample perimeter corners to estimate outer background color
    corners = np.array([
        arr[0, 0, :3], arr[0, w - 1, :3],
        arr[h - 1, 0, :3], arr[h - 1, w - 1, :3],
        arr[0, w // 2, :3], arr[h - 1, w // 2, :3],
    ], dtype=float)
    bg_color = np.median(corners, axis=0)

    color_diff = np.linalg.norm(arr[:, :, :3].astype(float) - bg_color, axis=-1)
    is_bg_candidate = color_diff < color_tolerance

    # Flood fill strictly from perimeter to protect internal character highlights
    bg_mask = np.zeros((h, w), dtype=bool)
    queue = deque()

    for x in range(w):
        if is_bg_candidate[0, x]:
            queue.append((0, x))
            bg_mask[0, x] = True
        if is_bg_candidate[h - 1, x]:
            queue.append((h - 1, x))
            bg_mask[h - 1, x] = True

    for y in range(h):
        if is_bg_candidate[y, 0]:
            queue.append((y, 0))
            bg_mask[y, 0] = True
        if is_bg_candidate[y, w - 1]:
            queue.append((y, w - 1))
            bg_mask[y, w - 1] = True

    while queue:
        cy, cx = queue.popleft()
        for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            ny, nx = cy + dy, cx + dx
            if 0 <= ny < h and 0 <= nx < w and not bg_mask[ny, nx] and is_bg_candidate[ny, nx]:
                bg_mask[ny, nx] = True
                queue.append((ny, nx))

    alpha = np.where(bg_mask, 0, 255).astype(np.uint8)
    alpha_img = Image.fromarray(alpha, mode="L").filter(ImageFilter.GaussianBlur(radius=1.2))
    rgba.putalpha(alpha_img)
    return rgba


def build_state_overlay(base_idle: Image.Image, state: str) -> Image.Image:
    """Generate reactive animation states from the master idle asset."""
    canvas = base_idle.copy()
    w, h = canvas.size
    overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    if state == "working":
        draw.rounded_rectangle([w * 0.20, h * 0.72, w * 0.80, h * 0.92], radius=10, fill=(15, 23, 42, 215), outline=(56, 189, 248, 240), width=2)
        draw.line([w * 0.25, h * 0.78, w * 0.55, h * 0.78], fill=(74, 222, 128, 255), width=3)
        draw.line([w * 0.25, h * 0.85, w * 0.70, h * 0.85], fill=(56, 189, 248, 230), width=2)
    elif state == "thinking":
        draw.ellipse([w * 0.68, h * 0.18, w * 0.74, h * 0.24], fill=(255, 255, 255, 220), outline=(148, 163, 184, 220), width=2)
        draw.ellipse([w * 0.75, h * 0.10, w * 0.87, h * 0.22], fill=(255, 255, 255, 240), outline=(148, 163, 184, 240), width=2)
    elif state in ("success", "celebrating"):
        for ox, oy in [(w * 0.18, h * 0.25), (w * 0.82, h * 0.25)]:
            draw.polygon([(ox, oy - 16), (ox + 5, oy - 5), (ox + 16, oy), (ox + 5, oy + 5), (ox, oy + 16), (ox - 5, oy + 5), (ox - 16, oy), (ox - 5, oy - 5)], fill=(253, 224, 71, 245))
    elif state == "error":
        draw.polygon([(w * 0.50, h * 0.12), (w * 0.58, h * 0.26), (w * 0.42, h * 0.26)], fill=(239, 68, 68, 230), outline=(255, 255, 255, 255), width=2)
        draw.line([w * 0.50, h * 0.16, w * 0.50, h * 0.21], fill=(255, 255, 255, 255), width=3)
        draw.point((w * 0.50, h * 0.23), fill=(255, 255, 255, 255))
    elif state == "sleeping":
        draw.text((int(w * 0.70), int(h * 0.20)), "Z", fill=(147, 197, 253, 240))
        draw.text((int(w * 0.78), int(h * 0.14)), "Z", fill=(96, 165, 250, 240))

    return Image.alpha_composite(canvas, overlay)


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parent.parent if "__file__" in globals() else Path.cwd()
    output_dir = args.output_dir or (repo_root / "pet-packs" / args.character_id / "assets")
    output_dir.mkdir(parents=True, exist_ok=True)

    prompt = args.prompt_override or load_character_prompt(args.character_id, repo_root)
    logger.info("Character: %s", args.character_id)
    logger.info("Prompt: %s", prompt)

    if args.engine == "flux":
        raw_image = generate_flux_hf(prompt, args.hf_token)
    elif args.engine == "gemini":
        raw_image = generate_gemini_imagen(prompt, args.gemini_key)
    else:
        raise ValueError(f"Unsupported engine: {args.engine}")

    logger.info("Raw generation completed (%dx%d). Matting background to alpha...", raw_image.width, raw_image.height)
    matted_image = remove_solid_background(raw_image)
    idle_master = matted_image.resize((args.size, args.size), Image.Resampling.LANCZOS)

    idle_path = output_dir / "idle.png"
    idle_master.save(idle_path, format="PNG", optimize=True)
    logger.info("Saved master idle: %s", idle_path)

    states = ["working", "thinking", "waiting", "success", "error", "level_up", "sleeping", "thumbnail", "achievement", "celebrating"]
    for s in states:
        state_asset = build_state_overlay(idle_master, s)
        state_path = output_dir / f"{s}.png"
        state_asset.save(state_path, format="PNG", optimize=True)

    logger.info("Successfully produced all %d assets for '%s'.", len(states) + 1, args.character_id)
    return 0


if __name__ == "__main__":
    sys.exit(main())
