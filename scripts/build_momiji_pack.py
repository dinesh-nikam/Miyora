#!/usr/bin/env python3
"""Generate the Momiji pixel sprite pack (pet-packs/momiji).

Momiji (`miyora.momiji` in the avatar catalog) is an original character:
long dark hair with rose-pink tips, a pink petal-pattern kimono, bare feet,
and a rose worn in her hair. This script renders one 512x512 RGBA sprite per
pet state with Pillow and writes the `pet.json` manifest that
`crates/pet-pack` validates.

The output is deterministic: running the script twice produces identical
assets and an identical manifest.

Usage:
    python scripts/build_momiji_pack.py [--pack-root PATH]
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

# Momiji palette — mirrors crates/avatar/catalog/characters.json (miyora.momiji)
SKIN = (242, 201, 168, 255)       # #f2c9a8
HAND = (255, 238, 224, 255)
HAIR = (12, 10, 9, 255)           # #0c0a09
HAIR_TIP = (251, 113, 133, 255)   # #fb7185
EYE = (219, 39, 119, 255)         # #db2777
KIMONO = (190, 24, 87, 255)       # #be185d
KIMONO_DEEP = (157, 23, 77, 255)
KIMONO_DARK = (17, 24, 39, 255)   # #111827 base robe
ACCENT = (249, 168, 212, 255)     # #f9a8d4 petal motifs
BLUSH = (244, 114, 146, 190)
WHITE = (255, 255, 255, 255)
BLACK = (20, 20, 25, 255)
SMOKE = (90, 90, 100, 210)
WOOD = (146, 92, 40, 255)

STATES = [
    "idle", "working", "thinking", "waiting",
    "success", "celebrating", "error", "level_up",
    "achievement", "sleeping", "thumbnail",
]

MANIFEST_ASSETS = [
    "idle", "working", "thinking", "waiting",
    "success", "error", "level_up", "sleeping", "thumbnail",
]


def build_manifest() -> dict:
    """Assemble the pet.json content, byte-for-byte identical on reruns."""
    assets = {state: f"assets/{state}.png" for state in MANIFEST_ASSETS}

    def animation(asset: str) -> dict:
        return {
            "asset": asset,
            "frames": [{"x": 0, "y": 0, "width": 512, "height": 512}],
            "fps": 1,
            "loop": True,
            "anchor": {"x": 0.5, "y": 1.0},
        }

    animations = {
        state: animation(state)
        for state in ("idle", "working", "thinking", "waiting", "success",
                      "error", "level_up", "sleeping")
    }
    animations["celebrating"] = animation("success")
    animations["achievement"] = animation("level_up")

    return {
        "schemaVersion": 1,
        "id": "agentpet.momiji",
        "name": "Momiji",
        "version": "1.0.0",
        "description": (
            "Quiet kimono companion with long dark hair that fades to rose "
            "tips, a pink petal-pattern kimono, and bare feet that never make "
            "a sound about failing tests. Rose (#be185d) layered over midnight "
            "plum (#111827). Drifts petal-wise between tasks: patient through "
            "slow builds, precise at merge time."
        ),
        "creator": "Miyora contributors",
        "license": "CC0-1.0",
        "source": "https://github.com/agentpet/pet-packs/tree/main/momiji",
        "attribution": (
            "Original character design and artwork by Miyora contributors. "
            "CC0-1.0 — released to the public domain."
        ),
        "commercialUse": True,
        "redistribution": True,
        "derivativeWorks": True,
        "assets": assets,
        "animations": animations,
        "stateMappings": {
            "idle": "idle",
            "working": "working",
            "thinking": "thinking",
            "waiting": "waiting",
            "success": "success",
            "celebrating": "celebrating",
            "error": "error",
            "level_up": "level_up",
            "achievement": "achievement",
            "sleeping": "sleeping",
            "offline": "sleeping",
            "attention_required": "waiting",
        },
        "tags": ["anime", "kimono", "calm", "petals", "momiji", "clean-room"],
    }


def canvas() -> tuple[Image.Image, ImageDraw.ImageDraw]:
    img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img)


def petal(draw: ImageDraw.ImageDraw, x: int, y: int, r: int, fill=ACCENT,
          outline=None) -> None:
    """A small round petal dot used for drifting petals and motifs."""
    draw.ellipse([x - r, y - r, x + r, y + r], fill=fill, outline=outline)


def plum_blossom(draw: ImageDraw.ImageDraw, cx: int, cy: int, r: int) -> None:
    """Five-petal rose worn in Momiji's hair."""
    for angle in range(0, 360, 72):
        a = angle * math.pi / 180.0
        px = cx + r * 0.82 * math.cos(a)
        py = cy + r * 0.82 * math.sin(a)
        pr = r * 0.55
        draw.ellipse([px - pr, py - pr, px + pr, py + pr], fill=ACCENT,
                     outline=KIMONO_DEEP, width=2)
    cr = r * 0.38
    draw.ellipse([cx - cr, cy - cr, cx + cr, cy + cr], fill=EYE)


def hair_back(draw: ImageDraw.ImageDraw, cx: int, cy: int, reach: int,
              tip: bool = True) -> None:
    """Long flowing hair mass behind the head, fading to rose tips."""
    draw.polygon(
        [(cx - 62, cy - 20), (cx + 62, cy - 20), (cx + 70, reach - 24),
         (cx - 70, reach - 24)],
        fill=HAIR,
    )
    draw.ellipse([cx - 62, cy - 40, cx + 62, cy + 40], fill=HAIR)
    draw.ellipse([cx - 70, reach - 46, cx + 70, reach + 6], fill=HAIR)
    if tip:
        # Rose fade: strand tips catching the color at the bottom.
        for sx in (cx - 46, cx - 16, cx + 16, cx + 46):
            petal(draw, sx, reach - 4, 13, HAIR_TIP)
            petal(draw, sx, reach + 8, 9, HAIR_TIP)


def face(draw: ImageDraw.ImageDraw, cx: int, hy: int, eyes: str) -> None:
    """Round face with clean bangs; the eyes vary per state."""
    draw.ellipse([cx - 34, hy - 16, cx + 34, hy + 34], fill=SKIN)
    # Bangs: a dark curl over the brow.
    draw.arc([cx - 34, hy - 24, cx + 34, hy + 8], 200, 340, fill=HAIR, width=10)
    draw.ellipse([cx - 26, hy + 12, cx - 16, hy + 20], fill=BLUSH)
    draw.ellipse([cx + 16, hy + 12, cx + 26, hy + 20], fill=BLUSH)
    if eyes == "open":
        draw.ellipse([cx - 20, hy + 2, cx - 8, hy + 16], fill=WHITE)
        draw.ellipse([cx + 8, hy + 2, cx + 20, hy + 16], fill=WHITE)
        draw.ellipse([cx - 18, hy + 4, cx - 8, hy + 14], fill=EYE)
        draw.ellipse([cx + 8, hy + 4, cx + 18, hy + 14], fill=EYE)
        draw.ellipse([cx - 16, hy + 6, cx - 13, hy + 9], fill=WHITE)
        draw.ellipse([cx + 13, hy + 6, cx + 16, hy + 9], fill=WHITE)
    elif eyes == "happy":
        draw.arc([cx - 22, hy + 2, cx - 8, hy + 16], 180, 360, fill=BLACK, width=3)
        draw.arc([cx + 8, hy + 2, cx + 22, hy + 16], 180, 360, fill=BLACK, width=3)
        draw.arc([cx - 6, hy + 22, cx + 6, hy + 30], 0, 180, fill=BLACK, width=2)
    elif eyes == "downcast":
        draw.line([(cx - 20, hy + 10), (cx - 9, hy + 10)], fill=BLACK, width=3)
        draw.line([(cx + 9, hy + 10), (cx + 20, hy + 10)], fill=BLACK, width=3)
        draw.arc([cx - 6, hy + 26, cx + 6, hy + 34], 180, 360, fill=BLACK, width=2)


def kimono(draw: ImageDraw.ImageDraw, cx: int, cy: int,
           feet: str | None = "both") -> None:
    """Pink-floral kimono over a dark base robe."""
    draw.polygon(
        [(cx - 44, cy + 2), (cx + 44, cy + 2), (cx + 48, cy + 112),
         (cx - 48, cy + 112)],
        fill=KIMONO_DARK,
    )
    draw.polygon(
        [(cx - 44, cy + 2), (cx + 6, cy + 2), (cx - 22, cy + 62),
         (cx - 48, cy + 62), (cx - 48, cy + 2)],
        fill=KIMONO,
    )
    draw.polygon(
        [(cx + 44, cy + 2), (cx + 6, cy + 2), (cx - 22, cy + 62), (cx + 44, cy + 62)],
        fill=KIMONO,
    )
    draw.line([(cx + 6, cy + 2), (cx - 22, cy + 62)], fill=KIMONO_DEEP, width=3)
    draw.rectangle([cx - 48, cy + 100, cx + 48, cy + 112], fill=KIMONO_DEEP)
    draw.rectangle([cx - 44, cy + 46, cx + 44, cy + 60], fill=EYE)
    draw.rectangle([cx - 10, cy + 44, cx + 10, cy + 62], fill=ACCENT)
    for px, py in [(cx + 14, cy + 26), (cx - 30, cy + 74), (cx + 26, cy + 78),
                   (cx + 2, cy + 90)]:
        petal(draw, px, py, 6, ACCENT)
    if feet == "both":
        draw.ellipse([cx - 26, cy + 108, cx - 6, cy + 122], fill=SKIN)
        draw.ellipse([cx + 6, cy + 108, cx + 26, cy + 122], fill=SKIN)
    elif feet == "single":
        draw.ellipse([cx + 6, cy + 108, cx + 26, cy + 122], fill=SKIN)


def load_master_asset(pack_root: Path) -> Image.Image | None:
    candidates = [
        pack_root / "assets" / "master.png",
        Path("pet-packs") / "momiji" / "assets" / "master.png",
        Path(__file__).resolve().parent.parent / "pet-packs" / "momiji" / "assets" / "master.png",
    ]
    for c in candidates:
        if c.exists():
            return Image.open(c).convert("RGBA").resize((512, 512), Image.Resampling.LANCZOS)
    return None


def create_radial_glow(size: int, inner_rgba: tuple, outer_rgba: tuple) -> Image.Image:
    glow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(glow)
    center = size / 2.0
    radius = size / 2.0
    for r in range(int(radius), 0, -2):
        t = r / radius
        c = tuple(int(inner_rgba[i] * (1 - t) + outer_rgba[i] * t) for i in range(4))
        draw.ellipse([center - r, center - r, center + r, center + r], fill=c)
    return glow


def render_idle(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        return master.copy()
    img, draw = canvas()
    cx, cy = 256, 300
    hair_back(draw, cx, cy - 30, cy + 116)
    kimono(draw, cx, cy)
    draw.line([(cx - 38, cy + 10), (cx - 44, cy + 76)], fill=KIMONO_DARK, width=12)
    draw.line([(cx + 38, cy + 10), (cx + 44, cy + 76)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx - 50, cy + 70, cx - 38, cy + 82], fill=HAND)
    draw.ellipse([cx + 38, cy + 70, cx + 50, cy + 82], fill=HAND)
    face(draw, cx, cy - 62, "open")
    plum_blossom(draw, cx + 52, cy - 88, 22)
    petal(draw, cx - 96, cy + 70, 7, HAIR_TIP)
    petal(draw, cx + 98, cy + 84, 5, ACCENT)
    petal(draw, cx - 74, cy + 96, 5, ACCENT)
    petal(draw, cx + 70, cy + 56, 6, HAIR_TIP)
    return img


def render_working(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        glow = create_radial_glow(480, (255, 180, 200, 70), (0, 0, 0, 0))
        canvas_img.paste(glow, (16, 16), glow)
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        for px, py, r in [(-110, 80, 8), (-80, 110, 6), (100, 95, 7), (125, 60, 5), (-30, 130, 6)]:
            petal(draw, cx + px, cy + py, r, HAIR_TIP)
        return canvas_img
    img, draw = canvas()
    cx, cy = 262, 300
    hair_back(draw, cx, cy - 30, cy + 116)
    kimono(draw, cx, cy, feet="single")
    draw.line([(cx - 64, cy + 104), (cx + 12, cy + 34)], fill=WOOD, width=5)
    draw.polygon([(cx - 76, cy + 118), (cx - 46, cy + 118), (cx - 62, cy + 96)],
                 fill=ACCENT, outline=KIMONO_DEEP)
    draw.ellipse([cx - 84, cy + 112, cx - 62, cy + 126], fill=HAIR_TIP)
    draw.line([(cx + 34, cy + 10), (cx + 16, cy + 44)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx + 4, cy + 34, cx + 20, cy + 50], fill=HAND)
    face(draw, cx, cy - 62, "open")
    plum_blossom(draw, cx + 52, cy - 88, 22)
    petal(draw, cx + 84, cy + 112, 6, HAIR_TIP)
    petal(draw, cx + 98, cy + 106, 5, ACCENT)
    petal(draw, cx + 90, cy + 120, 4, HAIR_TIP)
    return img


def render_thinking(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        for px, py, r in [(70, -110, 5), (90, -140, 7), (115, -175, 10)]:
            petal(draw, cx + px, cy + py, r, ACCENT)
            draw.ellipse([cx + px - r, cy + py - r, cx + px + r, cy + py + r], outline=KIMONO_DEEP, width=2)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 300
    hair_back(draw, cx, cy - 30, cy + 116)
    kimono(draw, cx, cy)
    draw.line([(cx + 36, cy + 12), (cx + 22, cy - 40)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx + 12, cy - 54, cx + 30, cy - 36], fill=HAND)
    face(draw, cx, cy - 62, "open")
    plum_blossom(draw, cx + 52, cy - 88, 22)
    petal(draw, cx + 64, cy - 116, 4, ACCENT)
    petal(draw, cx + 84, cy - 138, 6, HAIR_TIP)
    petal(draw, cx + 112, cy - 166, 9, ACCENT)
    petal(draw, cx + 130, cy - 196, 5, ACCENT)
    return img


def render_waiting(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        draw.rectangle([cx - 5, cy - 200, cx + 5, cy - 170], fill=KIMONO)
        draw.ellipse([cx - 5, cy - 162, cx + 5, cy - 152], fill=KIMONO)
        petal(draw, cx - 4, cy + 130, 6, HAIR_TIP)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 306
    hair_back(draw, cx, cy - 24, cy + 110, tip=False)
    kimono(draw, cx, cy)
    draw.line([(cx - 36, cy + 12), (cx - 16, cy + 44)], fill=KIMONO_DARK, width=11)
    draw.line([(cx + 36, cy + 12), (cx + 16, cy + 44)], fill=KIMONO_DARK, width=11)
    face(draw, cx, cy - 58, "open")
    plum_blossom(draw, cx + 52, cy - 84, 22)
    hy = cy - 58
    draw.rectangle([cx - 6, hy - 96, cx + 6, hy - 66], fill=KIMONO)
    draw.ellipse([cx - 6, hy - 58, cx + 6, hy - 46], fill=KIMONO)
    petal(draw, cx - 4, cy + 130, 6, HAIR_TIP)
    return img


def render_success(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        glow = create_radial_glow(480, (255, 215, 0, 65), (0, 0, 0, 0))
        canvas_img.paste(glow, (16, 16), glow)
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        for bx, by in [(-120, -70), (-140, -10), (120, -70), (140, -10), (0, -120)]:
            draw.polygon([
                (cx + bx, cy + by - 12), (cx + bx + 3, cy + by - 3),
                (cx + bx + 12, cy + by), (cx + bx + 3, cy + by + 3),
                (cx + bx, cy + by + 12), (cx + bx - 3, cy + by + 3),
                (cx + bx - 12, cy + by), (cx + bx - 3, cy + by - 3)
            ], fill=(255, 215, 0, 240))
        for px, py in [(-90, -40), (90, -40), (0, -80)]:
            petal(draw, cx + px, cy + py, 7, HAIR_TIP)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 300
    hair_back(draw, cx, cy - 30, cy + 116)
    kimono(draw, cx, cy)
    draw.line([(cx - 36, cy + 10), (cx - 6, cy + 36)], fill=KIMONO_DARK, width=12)
    draw.line([(cx + 36, cy + 10), (cx + 6, cy + 36)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx - 16, cy + 28, cx + 2, cy + 44], fill=HAND)
    draw.ellipse([cx - 2, cy + 28, cx + 16, cy + 44], fill=HAND)
    face(draw, cx, cy - 62, "happy")
    plum_blossom(draw, cx + 52, cy - 88, 22)
    for bx, by, br in [(-120, -80, 8), (-140, -30, 5), (124, -82, 8),
                       (144, -32, 5), (0, -110, 6), (-84, -108, 5),
                       (86, -106, 5)]:
        petal(draw, cx + bx, cy + by, br, HAIR_TIP)
    return img


def render_celebrating(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        glow = create_radial_glow(480, (255, 215, 0, 80), (0, 0, 0, 0))
        canvas_img.paste(glow, (16, 16), glow)
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        for px, py, pr in [(-110, -60, 6), (-96, 10, 5), (-118, -6, 4),
                           (112, -64, 6), (98, 6, 5), (120, -10, 4)]:
            petal(draw, cx + px, cy + py, pr, ACCENT)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 296
    hair_back(draw, cx, cy - 30, cy + 116)
    kimono(draw, cx, cy)
    draw.line([(cx - 34, cy + 12), (cx - 52, cy - 66)], fill=KIMONO_DARK, width=12)
    draw.line([(cx + 34, cy + 12), (cx + 52, cy - 66)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx - 62, cy - 78, cx - 46, cy - 62], fill=HAND)
    draw.ellipse([cx + 46, cy - 78, cx + 62, cy - 62], fill=HAND)
    plum_blossom(draw, cx, cy - 96, 18)
    face(draw, cx, cy - 62, "happy")
    for px, py, pr in [(-110, -60, 6), (-96, 10, 5), (-118, -6, 4),
                       (112, -64, 6), (98, 6, 5), (120, -10, 4)]:
        petal(draw, cx + px, cy + py, pr, ACCENT)
    draw.ellipse([cx + 28, cy + 118, cx + 52, cy + 132], fill=SKIN)
    return img


def render_error(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        draw.polygon([(cx + 80, cy - 110), (cx + 86, cy - 94), (cx + 74, cy - 94)], fill=(100, 200, 255, 230))
        draw.ellipse([cx + 74, cy - 98, cx + 86, cy - 86], fill=(100, 200, 255, 230))
        petal(draw, cx - 34, cy + 128, 5, KIMONO_DEEP)
        petal(draw, cx + 34, cy + 126, 5, KIMONO_DEEP)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 306
    hair_back(draw, cx, cy - 26, cy + 118)
    kimono(draw, cx, cy)
    draw.line([(cx - 34, cy + 20), (cx + 2, cy + 56)], fill=KIMONO_DARK, width=12)
    draw.line([(cx + 34, cy + 20), (cx - 2, cy + 56)], fill=KIMONO_DARK, width=12)
    face(draw, cx, cy - 52, "downcast")
    plum_blossom(draw, cx + 58, cy - 72, 20)
    draw.ellipse([cx + 74, cy - 96, cx + 118, cy - 58], fill=SMOKE)
    draw.ellipse([cx + 104, cy - 128, cx + 138, cy - 100], fill=SMOKE)
    petal(draw, cx - 34, cy + 128, 5, KIMONO_DEEP)
    petal(draw, cx + 34, cy + 126, 5, KIMONO_DEEP)
    return img


def render_level_up(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        glow = create_radial_glow(480, (255, 215, 0, 95), (0, 0, 0, 0))
        canvas_img.paste(glow, (16, 16), glow)
        canvas_img.paste(master, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        draw.ellipse([cx - 80, cy - 180, cx + 80, cy - 20], outline=(255, 240, 130, 200), width=3)
        draw.ellipse([cx - 100, cy - 200, cx + 100, cy], outline=(255, 180, 80, 160), width=2)
        ring = [
            (cx - 128, cy + 30), (cx - 102, cy - 38), (cx - 56, cy - 96),
            (cx + 8, cy - 132), (cx + 74, cy - 100), (cx + 114, cy - 34),
            (cx + 128, cy + 34), (cx + 96, cy + 82), (cx - 6, cy + 124),
            (cx - 92, cy + 82),
        ]
        for i, (px, py) in enumerate(ring):
            petal(draw, px, py, 7 if i % 2 == 0 else 5,
                  HAIR_TIP if i % 3 else ACCENT)
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 282
    hair_back(draw, cx, cy - 30, cy + 118)
    kimono(draw, cx, cy, feet=None)
    draw.ellipse([cx - 30, cy + 118, cx - 8, cy + 138], fill=SKIN)
    draw.ellipse([cx + 8, cy + 118, cx + 30, cy + 138], fill=SKIN)
    draw.line([(cx - 36, cy + 10), (cx - 62, cy - 24)], fill=KIMONO_DARK, width=12)
    draw.line([(cx + 36, cy + 10), (cx + 62, cy - 24)], fill=KIMONO_DARK, width=12)
    draw.ellipse([cx - 70, cy - 36, cx - 54, cy - 20], fill=HAND)
    draw.ellipse([cx + 54, cy - 36, cx + 70, cy - 20], fill=HAND)
    face(draw, cx, cy - 62, "happy")
    plum_blossom(draw, cx + 52, cy - 88, 22)
    ring = [
        (cx - 128, cy + 30), (cx - 102, cy - 38), (cx - 56, cy - 96),
        (cx + 8, cy - 132), (cx + 74, cy - 100), (cx + 114, cy - 34),
        (cx + 128, cy + 34), (cx + 96, cy + 82), (cx - 6, cy + 124),
        (cx - 92, cy + 82),
    ]
    for i, (px, py) in enumerate(ring):
        petal(draw, px, py, 7 if i % 2 == 0 else 5,
              HAIR_TIP if i % 3 else ACCENT)
    draw.ellipse([cx - 150, cy - 128, cx + 150, cy + 152], outline=ACCENT, width=3)
    return img


def render_sleeping(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        canvas_img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        tinted = master.copy()
        night_tint = Image.new("RGBA", (512, 512), (30, 40, 90, 75))
        tinted = Image.alpha_composite(tinted, night_tint)
        canvas_img.paste(tinted, (0, 0), master)
        draw = ImageDraw.Draw(canvas_img)
        cx, cy = 256, 256
        for txt, ox, oy in [("z", 70, -80), ("Z", 95, -110), ("Z", 125, -145)]:
            draw.text((cx + ox, cy + oy), txt, fill=(180, 210, 255, 230))
        return canvas_img
    img, draw = canvas()
    cx, cy = 256, 330
    draw.ellipse([cx - 78, cy - 30, cx + 78, cy + 88], fill=KIMONO_DEEP)
    draw.ellipse([cx - 64, cy - 16, cx + 64, cy + 76], fill=KIMONO)
    for px, py in [(-40, 10), (8, -6), (46, 26), (-10, 44)]:
        petal(draw, cx + px, cy + py, 6, ACCENT)
    hair_back(draw, cx - 10, cy - 58, cy - 20)
    draw.ellipse([cx - 42, cy - 66, cx + 30, cy - 8], fill=SKIN)
    draw.arc([cx - 26, cy - 40, cx - 8, cy - 26], 0, 180, fill=BLACK, width=3)
    draw.arc([cx + 8, cy - 40, cx + 26, cy - 26], 0, 180, fill=BLACK, width=3)
    draw.ellipse([cx - 24, cy - 18, cx - 14, cy - 10], fill=BLUSH)
    plum_blossom(draw, cx + 66, cy + 40, 16)
    draw.text((cx + 92, cy - 96), "Z", fill=KIMONO, font_size=30)
    draw.text((cx + 118, cy - 126), "z", fill=ACCENT, font_size=22)
    return img


def render_thumbnail(master: Image.Image | None = None) -> Image.Image:
    if master is not None:
        return master.copy()
    img, draw = canvas()
    tcx, tcy = 256, 250
    draw.polygon([(tcx - 132, 512), (tcx + 132, 512), (tcx + 108, tcy + 148),
                  (tcx, tcy + 168), (tcx - 108, tcy + 148)],
                 fill=KIMONO_DARK)
    draw.polygon([(tcx - 132, 512), (tcx - 10, 512), (tcx - 60, tcy + 130),
                  (tcx - 132, tcy + 120)], fill=KIMONO)
    draw.line([(tcx - 12, 512), (tcx - 58, tcy + 132)], fill=KIMONO_DEEP, width=4)
    draw.rectangle([tcx - 96, 488, tcx + 96, 512], fill=EYE)
    draw.rectangle([tcx - 16, 482, tcx + 16, 512], fill=ACCENT)
    hair_back(draw, tcx, tcy - 40, tcy + 150)
    for sx in (tcx - 78, tcx - 44, tcx - 10, tcx + 24, tcx + 58, tcx + 88):
        petal(draw, sx, tcy + 128, 16, HAIR_TIP)
    draw.ellipse([tcx - 56, tcy - 40, tcx + 56, tcy + 62], fill=SKIN)
    draw.arc([tcx - 56, tcy - 52, tcx + 56, tcy + 2], 200, 340, fill=HAIR, width=17)
    draw.ellipse([tcx - 34, tcy + 2, tcx - 6, tcy + 34], fill=WHITE)
    draw.ellipse([tcx + 6, tcy + 2, tcx + 34, tcy + 34], fill=WHITE)
    draw.ellipse([tcx - 31, tcy + 5, tcx - 9, tcy + 31], fill=EYE)
    draw.ellipse([tcx + 9, tcy + 5, tcx + 31, tcy + 31], fill=EYE)
    draw.ellipse([tcx - 27, tcy + 9, tcx - 20, tcy + 16], fill=WHITE)
    draw.ellipse([tcx + 20, tcy + 9, tcx + 27, tcy + 16], fill=WHITE)
    draw.ellipse([tcx - 46, tcy + 32, tcx - 30, tcy + 44], fill=BLUSH)
    draw.ellipse([tcx + 30, tcy + 32, tcx + 46, tcy + 44], fill=BLUSH)
    draw.arc([tcx - 8, tcy + 46, tcx + 8, tcy + 58], 0, 180, fill=BLACK, width=3)
    plum_blossom(draw, tcx + 66, tcy - 78, 26)
    return img


RENDERERS = {
    "idle": render_idle,
    "working": render_working,
    "thinking": render_thinking,
    "waiting": render_waiting,
    "success": render_success,
    "celebrating": render_celebrating,
    "error": render_error,
    "level_up": render_level_up,
    "achievement": render_level_up,
    "sleeping": render_sleeping,
    "thumbnail": render_thumbnail,
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--pack-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "pet-packs" / "momiji",
        help="where the pack is written (default pet-packs/momiji)",
    )
    args = parser.parse_args()
    try:
        assets_dir = args.pack_root / "assets"
        assets_dir.mkdir(parents=True, exist_ok=True)
        master = load_master_asset(args.pack_root)
        sprite_states = [s for s in STATES if s != "achievement"]
        missing = [state for state in sprite_states if state not in RENDERERS]
        if missing:
            raise ValueError(
                f"missing renderer function for states: {', '.join(missing)}"
            )
        sprite_bytes: dict[str, bytes] = {}
        for state in sprite_states:
            renderer = RENDERERS[state]
            try:
                sprite = renderer(master=master)
            except TypeError:
                sprite = renderer()
            target = assets_dir / f"{state}.png"
            sprite.save(target)
            sprite_bytes[state] = target.read_bytes()
        (assets_dir / "achievement.png").write_bytes(sprite_bytes["level_up"])
        manifest = build_manifest()
        (args.pack_root / "pet.json").write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
            newline="\n",
        )
    except (OSError, ValueError) as error:
        print(f"momiji pack error: {error}", file=sys.stderr)
        return 2
    print(f"wrote pack {args.pack_root} ({len(STATES)} states, pet.json)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
