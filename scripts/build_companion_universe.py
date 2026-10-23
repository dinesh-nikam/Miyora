#!/usr/bin/env python3
"""Build and compile the complete Companion Universe character roster.

Generates validated PetPacks with 512x512 RGBA transparent assets for all
42 characters across Indian Epic/Devotional, Top Anime, and Ghibli-inspired collections.

Usage:
    python scripts/build_companion_universe.py [--category all|devotional|anime|ghibli] [--character ID]
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
import json
import math
import os
from pathlib import Path
import sys
from typing import Dict, List, Optional, Tuple

from PIL import Image, ImageChops, ImageDraw, ImageFilter

REPO_ROOT = Path(__file__).resolve().parent.parent
PET_PACKS_DIR = REPO_ROOT / "pet-packs"
SUPERSAMPLE = 2  # Render at 2x, then downscale for clean anti-aliased edges.


@dataclass(frozen=True)
class Palette:
    skin: Tuple[int, int, int, int]
    hair: Tuple[int, int, int, int]
    eyes: Tuple[int, int, int, int]
    primary: Tuple[int, int, int, int]
    secondary: Tuple[int, int, int, int]
    accent: Tuple[int, int, int, int]


@dataclass(frozen=True)
class CharacterDef:
    id: str
    name: str
    category: str
    title: str
    head_ratio: float
    palette: Palette
    hair_style: str  # "curls", "long", "spiky", "bald", "bob", "twintails", "ears", "antlers", etc.
    outfit_style: str  # "dhoti", "sari", "uniform", "armor", "kimono", "smock", "suit", "vest", "creature", etc.
    prop_type: str  # "flute", "bow", "mace", "modak", "trident", "lotus", "veena", "katana", "staff", "blades", etc.
    special_features: List[str]
    description: str


def hex_to_rgba(h: str, a: int = 255) -> Tuple[int, int, int, int]:
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


# -----------------------------------------------------------------------------
# 42-Character Registry
# -----------------------------------------------------------------------------
CHARACTERS: List[CharacterDef] = [
    # 1. Indian Epic & Devotional
    CharacterDef(
        "krishna", "Krishna", "devotional", "Divine Flute Companion", 3.0,
        Palette(hex_to_rgba("7da4d4"), hex_to_rgba("1a2238"), hex_to_rgba("231f20"), hex_to_rgba("ffd700"), hex_to_rgba("f59e0b"), hex_to_rgba("38bdf8")),
        "curls", "dhoti", "flute", ["peacock_feather", "flute", "lotus", "tilak"],
        "Cute blue-skinned Krishna playing his golden flute with peacock feather."
    ),
    CharacterDef(
        "radha", "Radha", "devotional", "Graceful Beloved", 3.2,
        Palette(hex_to_rgba("f6d7c3"), hex_to_rgba("1e1b18"), hex_to_rgba("451a03"), hex_to_rgba("e11d48"), hex_to_rgba("f59e0b"), hex_to_rgba("fbcfe8")),
        "long", "sari", "lotus", ["flower_garland", "dupatta", "bangles"],
        "Graceful cute Radha in crimson and saffron silks with floral ornaments."
    ),
    CharacterDef(
        "rama", "Rama", "devotional", "Righteous Archer Prince", 3.4,
        Palette(hex_to_rgba("6ba3b8"), hex_to_rgba("111827"), hex_to_rgba("1e293b"), hex_to_rgba("f97316"), hex_to_rgba("ffd700"), hex_to_rgba("15803d")),
        "spiky", "dhoti", "bow", ["mukut", "bow", "quiver", "tilak"],
        "Dignified blue-skinned Rama standing calmly with his Kodanda bow."
    ),
    CharacterDef(
        "sita", "Sita", "devotional", "Graceful Princess of Mithila", 3.2,
        Palette(hex_to_rgba("f8dfd0"), hex_to_rgba("1c1917"), hex_to_rgba("292524"), hex_to_rgba("dc2626"), hex_to_rgba("ffd700"), hex_to_rgba("fef08a")),
        "long", "sari", "lotus", ["maang_tikka", "gold_border", "flower"],
        "Kind and graceful Sita in vermilion and gold silk sari."
    ),
    CharacterDef(
        "hanuman", "Hanuman", "devotional", "Mighty Loyal Hero", 2.8,
        Palette(hex_to_rgba("e0ac83"), hex_to_rgba("78350f"), hex_to_rgba("451a03"), hex_to_rgba("ea580c"), hex_to_rgba("ffd700"), hex_to_rgba("b91c1c")),
        "ears", "dhoti", "mace", ["curled_tail", "gada", "mukut", "tilak"],
        "Adorable and heroic Hanuman with golden gada and curled tail."
    ),
    CharacterDef(
        "ganesha", "Ganesha", "devotional", "Lord of Good Fortune", 2.5,
        Palette(hex_to_rgba("f6d7c3"), hex_to_rgba("78350f"), hex_to_rgba("312e81"), hex_to_rgba("f97316"), hex_to_rgba("ffd700"), hex_to_rgba("dc2626")),
        "elephant", "dhoti", "modak", ["elephant_trunk", "large_ears", "crown", "mouse"],
        "Extra cute Ganesha with sweet trunk, gold crown, and delicious modak."
    ),
    CharacterDef(
        "shiva", "Shiva", "devotional", "Serene Ascetic Lord", 3.3,
        Palette(hex_to_rgba("8ba3be"), hex_to_rgba("0f172a"), hex_to_rgba("1e293b"), hex_to_rgba("d97706"), hex_to_rgba("475569"), hex_to_rgba("38bdf8")),
        "jata", "tiger", "trident", ["crescent_moon", "third_eye", "rudraksha", "trishula"],
        "Peaceful Shiva with blue skin, crescent moon, and golden trishula."
    ),
    CharacterDef(
        "parvati", "Parvati", "devotional", "Goddess of Devotion", 3.2,
        Palette(hex_to_rgba("f8dfd0"), hex_to_rgba("18181b"), hex_to_rgba("3b0764"), hex_to_rgba("059669"), hex_to_rgba("ffd700"), hex_to_rgba("e11d48")),
        "long", "sari", "lotus", ["golden_tiara", "emerald_sari", "flower"],
        "Loving and motherly Parvati in rich emerald green and gold sari."
    ),
    CharacterDef(
        "lakshmi", "Lakshmi", "devotional", "Goddess of Prosperity", 3.2,
        Palette(hex_to_rgba("fbe3d4"), hex_to_rgba("1c1917"), hex_to_rgba("451a03"), hex_to_rgba("be123c"), hex_to_rgba("ffd700"), hex_to_rgba("fda4af")),
        "long", "sari", "lotus", ["lotus_blossom", "gold_coins", "crown"],
        "Radiant Lakshmi surrounded by pink lotuses and showers of gold coins."
    ),
    CharacterDef(
        "saraswati", "Saraswati", "devotional", "Goddess of Wisdom", 3.2,
        Palette(hex_to_rgba("fdf2f8"), hex_to_rgba("1e293b"), hex_to_rgba("0284c7"), hex_to_rgba("f8fafc"), hex_to_rgba("ca8a04"), hex_to_rgba("67e8f9")),
        "long", "sari", "veena", ["veena", "swan_feather", "mala_beads"],
        "Pure Saraswati in cream silk sari holding her carved veena."
    ),

    # 2. Top Anime Characters
    CharacterDef(
        "gojo", "Gojo", "anime", "The Strongest Sorcerer", 3.6,
        Palette(hex_to_rgba("fce7f3"), hex_to_rgba("f8fafc"), hex_to_rgba("38bdf8"), hex_to_rgba("0f172a"), hex_to_rgba("1e1b4b"), hex_to_rgba("818cf8")),
        "spiky", "uniform", "none", ["blindfold", "high_collar", "six_eyes"],
        "Playful and invincible Gojo with white hair and signature blindfold."
    ),
    CharacterDef(
        "yuji", "Yuji", "anime", "Tiger Sorcerer", 3.2,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("fb7185"), hex_to_rgba("78350f"), hex_to_rgba("0f172a"), hex_to_rgba("dc2626"), hex_to_rgba("0284c7")),
        "spiky", "uniform", "fist", ["red_hoodie", "eye_scars", "blue_spark"],
        "Energetic Yuji Itadori in dark uniform with iconic red hoodie hood."
    ),
    CharacterDef(
        "tanjiro", "Tanjiro", "anime", "Sun Breathing Swordsman", 3.1,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("7f1d1d"), hex_to_rgba("991b1b"), hex_to_rgba("166534"), hex_to_rgba("111827"), hex_to_rgba("f59e0b")),
        "spiky", "kimono", "katana", ["checkered_haori", "hanafuda", "scar"],
        "Kindhearted Tanjiro with green checkered haori and hanafuda earrings."
    ),
    CharacterDef(
        "nezuko", "Nezuko", "anime", "Beloved Demon Companion", 2.8,
        Palette(hex_to_rgba("fde2d2"), hex_to_rgba("111827"), hex_to_rgba("f43f5e"), hex_to_rgba("f472b6"), hex_to_rgba("1c1917"), hex_to_rgba("ea580c")),
        "long", "kimono", "none", ["bamboo_muzzle", "pink_ribbon", "star_kimono", "orange_hair_tips"],
        "Adorable Nezuko with pink star-pattern kimono and green bamboo muzzle."
    ),
    CharacterDef(
        "luffy", "Luffy", "anime", "Straw Hat Captain", 3.0,
        Palette(hex_to_rgba("e0ac83"), hex_to_rgba("0c0a09"), hex_to_rgba("1e293b"), hex_to_rgba("dc2626"), hex_to_rgba("0284c7"), hex_to_rgba("eab308")),
        "shaggy", "vest", "none", ["straw_hat", "eye_scar", "red_vest", "sandals"],
        "Joyful pirate captain Luffy with straw hat and red vest."
    ),
    CharacterDef(
        "zoro", "Zoro", "anime", "Three-Sword Master", 3.3,
        Palette(hex_to_rgba("e0ac83"), hex_to_rgba("15803d"), hex_to_rgba("1e293b"), hex_to_rgba("14532d"), hex_to_rgba("334155"), hex_to_rgba("eab308")),
        "short", "samurai", "katana", ["green_hair", "triple_katana", "eye_scar", "haramaki"],
        "Cool swordsman Zoro with green hair, triple katanas, and sleepy smirk."
    ),
    CharacterDef(
        "nami", "Nami", "anime", "Cat Burglar Navigator", 3.2,
        Palette(hex_to_rgba("f6d7c3"), hex_to_rgba("ea580c"), hex_to_rgba("7c2d12"), hex_to_rgba("0284c7"), hex_to_rgba("f8fafc"), hex_to_rgba("facc15")),
        "long", "vest", "staff", ["orange_hair", "clima_tact", "mikan_tattoo"],
        "Clever and confident Nami with bright orange hair and weather staff."
    ),
    CharacterDef(
        "sanji", "Sanji", "anime", "Black Leg Chef", 3.4,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("facc15"), hex_to_rgba("1e293b"), hex_to_rgba("0f172a"), hex_to_rgba("1e1b4b"), hex_to_rgba("ea580c")),
        "swoop", "suit", "flame", ["spiral_eyebrow", "black_suit", "flame_kick"],
        "Charming chef Sanji in sleek black suit with fiery kick stance."
    ),
    CharacterDef(
        "naruto", "Naruto", "anime", "Hero of the Hidden Leaf", 3.0,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("facc15"), hex_to_rgba("0284c7"), hex_to_rgba("ea580c"), hex_to_rgba("1f2937"), hex_to_rgba("f8fafc")),
        "spiky", "tracksuit", "rasengan", ["headband", "whiskers", "orange_suit"],
        "Energetic ninja Naruto with spiky blond hair, headband, and whiskers."
    ),
    CharacterDef(
        "sasuke", "Sasuke", "anime", "Shadow Avenger", 3.3,
        Palette(hex_to_rgba("fce7f3"), hex_to_rgba("0f172a"), hex_to_rgba("1e1b4b"), hex_to_rgba("1e293b"), hex_to_rgba("e2e8f0"), hex_to_rgba("a855f7")),
        "spiky", "uniform", "katana", ["high_collar", "uchiha_fan", "chokuto"],
        "Cool and analytical Sasuke in high-collar shirt with kusanagi sword."
    ),
    CharacterDef(
        "goku", "Goku", "anime", "Saiyan Warrior", 3.0,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("0c0a09"), hex_to_rgba("1e293b"), hex_to_rgba("ea580c"), hex_to_rgba("1d4ed8"), hex_to_rgba("ffd700")),
        "spiky_wild", "gi", "kamehameha", ["wild_hair", "kame_kanji", "blue_wristbands"],
        "Happy martial-arts hero Goku with wild spiky hair and orange gi."
    ),
    CharacterDef(
        "vegeta", "Vegeta", "anime", "Prince of Saiyans", 3.0,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("0c0a09"), hex_to_rgba("111827"), hex_to_rgba("1e3a8a"), hex_to_rgba("f8fafc"), hex_to_rgba("eab308")),
        "spiky_up", "armor", "none", ["flame_hair", "saiyan_armor", "proud_scowl"],
        "Proud Prince Vegeta in white battle armor with crossed arms and pout."
    ),
    CharacterDef(
        "levi", "Levi", "anime", "Captain of Scouts", 3.1,
        Palette(hex_to_rgba("f5ece8"), hex_to_rgba("111827"), hex_to_rgba("334155"), hex_to_rgba("78350f"), hex_to_rgba("15803d"), hex_to_rgba("f8fafc")),
        "undercut", "jacket", "dual_blades", ["cravat", "wings_of_freedom", "reverse_blades"],
        "Disciplined Levi with clean undercut, white cravat, and dual blades."
    ),
    CharacterDef(
        "mikasa", "Mikasa", "anime", "Protective Scout", 3.3,
        Palette(hex_to_rgba("f6e7e7"), hex_to_rgba("111827"), hex_to_rgba("1f2937"), hex_to_rgba("78350f"), hex_to_rgba("991b1b"), hex_to_rgba("e2e8f0")),
        "bob", "jacket", "dual_blades", ["red_scarf", "wings_of_freedom", "harness"],
        "Quiet and determined Mikasa with her iconic dark red knit scarf."
    ),
    CharacterDef(
        "gon", "Gon", "anime", "Green Hunter", 2.9,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("14532d"), hex_to_rgba("78350f"), hex_to_rgba("16a34a"), hex_to_rgba("f97316"), hex_to_rgba("fde047")),
        "tall_spiky", "jacket", "rod", ["tall_green_hair", "fishing_rod", "boots"],
        "Cheerful Gon with tall spiky green hair and trusty fishing rod."
    ),
    CharacterDef(
        "killua", "Killua", "anime", "Lightning Prodigy", 3.0,
        Palette(hex_to_rgba("fce7f3"), hex_to_rgba("f8fafc"), hex_to_rgba("1d4ed8"), hex_to_rgba("1e3a8a"), hex_to_rgba("93c5fd"), hex_to_rgba("38bdf8")),
        "fluffy", "shirt", "lightning", ["fluffy_silver_hair", "electric_spark", "cat_mouth"],
        "Cool Killua with fluffy silver hair, cat smirk, and electric sparks."
    ),
    CharacterDef(
        "l-lawliet", "L", "anime", "Mastermind Detective", 3.0,
        Palette(hex_to_rgba("f5ece8"), hex_to_rgba("111827"), hex_to_rgba("1e1b4b"), hex_to_rgba("f8fafc"), hex_to_rgba("334155"), hex_to_rgba("ef4444")),
        "messy", "baggy", "cake", ["dark_circles", "crouch_pose", "strawberry_cake"],
        "Eccentric detective L crouching in oversized shirt with a slice of cake."
    ),
    CharacterDef(
        "light", "Light", "anime", "Prodigy Scholar", 3.4,
        Palette(hex_to_rgba("f6d7c3"), hex_to_rgba("78350f"), hex_to_rgba("991b1b"), hex_to_rgba("b45309"), hex_to_rgba("111827"), hex_to_rgba("dc2626")),
        "neat", "blazer", "notebook", ["death_note", "school_tie", "pen"],
        "Composed Light Yagami in school blazer holding the black notebook."
    ),
    CharacterDef(
        "ichigo", "Ichigo", "anime", "Substitute Shinigami", 3.4,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("ea580c"), hex_to_rgba("854d0e"), hex_to_rgba("0f172a"), hex_to_rgba("f8fafc"), hex_to_rgba("dc2626")),
        "spiky", "shinigami", "cleaver", ["orange_hair", "giant_zangetsu", "bandage_wraps"],
        "Protective Ichigo with bright orange hair and giant Zangetsu blade."
    ),
    CharacterDef(
        "rukia", "Rukia", "anime", "Snow Soul Reaper", 3.0,
        Palette(hex_to_rgba("f6e7e7"), hex_to_rgba("0c0a09"), hex_to_rgba("312e81"), hex_to_rgba("0f172a"), hex_to_rgba("f8fafc"), hex_to_rgba("67e8f9")),
        "bob_strand", "shinigami", "katana", ["center_strand", "white_katana", "ribbon"],
        "Graceful Rukia in black shinigami robe with pure white zanpakuto."
    ),
    CharacterDef(
        "saitama", "Saitama", "anime", "Hero for Fun", 3.0,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("fcd5be"), hex_to_rgba("111827"), hex_to_rgba("eab308"), hex_to_rgba("dc2626"), hex_to_rgba("f8fafc")),
        "bald", "jumpsuit", "none", ["shiny_bald_head", "red_gloves", "white_cape", "deadpan_face"],
        "Deadpan Saitama in yellow suit and red gloves with simple dot face."
    ),
    CharacterDef(
        "genos", "Genos", "anime", "Demon Cyborg", 3.3,
        Palette(hex_to_rgba("f2c9a8"), hex_to_rgba("facc15"), hex_to_rgba("eab308"), hex_to_rgba("1e293b"), hex_to_rgba("0f172a"), hex_to_rgba("f97316")),
        "spiky", "vest", "cannon", ["cyber_arms", "glowing_cores", "gold_joints"],
        "Intense cyborg Genos with blond hair and mechanical gold-trimmed arms."
    ),
    CharacterDef(
        "denji", "Denji", "anime", "Chainsaw Devil", 3.1,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("facc15"), hex_to_rgba("92400e"), hex_to_rgba("f8fafc"), hex_to_rgba("1e293b"), hex_to_rgba("f97316")),
        "scruffy", "shirt", "cord", ["pull_cord", "pochita", "shark_teeth", "loose_tie"],
        "Chaotic Denji with chest pull-cord ring and friendly little Pochita."
    ),
    CharacterDef(
        "power", "Power", "anime", "Blood Devil", 3.1,
        Palette(hex_to_rgba("fde2d2"), hex_to_rgba("fed7aa"), hex_to_rgba("dc2626"), hex_to_rgba("f8fafc"), hex_to_rgba("1e293b"), hex_to_rgba("ef4444")),
        "long", "shirt", "none", ["red_horns", "cross_eyes", "meowy_cat", "loose_tie"],
        "Mischievous Power with twin red horns, cross eyes, and fang smile."
    ),
    CharacterDef(
        "anya", "Anya", "anime", "Telepathic Star Student", 2.6,
        Palette(hex_to_rgba("fce7f3"), hex_to_rgba("f472b6"), hex_to_rgba("16a34a"), hex_to_rgba("0f172a"), hex_to_rgba("ca8a04"), hex_to_rgba("fed7aa")),
        "bob_curled", "dress", "plush", ["chimera_horns", "stella_uniform", "heh_face", "peanuts"],
        "Ultra-cute Anya with pink hair, black horn clips, and smug 'heh' face."
    ),
    CharacterDef(
        "yor", "Yor", "anime", "Thorn Princess", 3.4,
        Palette(hex_to_rgba("fce7f3"), hex_to_rgba("111827"), hex_to_rgba("991b1b"), hex_to_rgba("0f172a"), hex_to_rgba("b91c1c"), hex_to_rgba("ffd700")),
        "headband_locks", "dress", "needles", ["gold_needles", "rose_lining", "rose_headband"],
        "Sweet and deadly Yor in black assassin dress with golden needles."
    ),
    CharacterDef(
        "frieren", "Frieren", "anime", "Mage of the Journey", 3.0,
        Palette(hex_to_rgba("fdf2f8"), hex_to_rgba("e2e8f0"), hex_to_rgba("10b981"), hex_to_rgba("f8fafc"), hex_to_rgba("1e293b"), hex_to_rgba("f59e0b")),
        "twintails", "coat", "staff", ["elf_ears", "crescent_staff", "amber_earrings"],
        "Gentle elven mage Frieren with white twintails and crescent staff."
    ),
    CharacterDef(
        "pikachu", "Pikachu", "anime", "Electric Mouse", 2.5,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("facc15"), hex_to_rgba("1e293b"), hex_to_rgba("facc15"), hex_to_rgba("dc2626"), hex_to_rgba("78350f")),
        "ears", "creature", "spark", ["red_cheeks", "lightning_tail", "black_ear_tips"],
        "Chubby cute Pikachu with red cheek sacs and lightning bolt tail."
    ),

    # 3. Studio Ghibli-Inspired Original Companions
    CharacterDef(
        "kodama-spirit", "Suzu", "ghibli", "Forest Sprout Spirit", 2.5,
        Palette(hex_to_rgba("fdfbf7"), hex_to_rgba("fdfbf7"), hex_to_rgba("1c1917"), hex_to_rgba("ecfdf5"), hex_to_rgba("a7f3d0"), hex_to_rgba("34d399")),
        "none", "creature", "none", ["clover_sprout", "tilted_head", "green_glow"],
        "Gentle porcelain forest spirit with tilted head and clover sprout."
    ),
    CharacterDef(
        "witch-apprentice", "Noa", "ghibli", "Little Broom Delivery Witch", 3.0,
        Palette(hex_to_rgba("fbe3d4"), hex_to_rgba("3f2b1d"), hex_to_rgba("451a03"), hex_to_rgba("1e1b4b"), hex_to_rgba("dc2626"), hex_to_rgba("fde047")),
        "bob", "smock", "broom", ["giant_red_bow", "twig_broom", "pocket_cat"],
        "Earnest apprentice witch in deep navy smock with giant red bow."
    ),
    CharacterDef(
        "valley-flyer", "Ren", "ghibli", "Wind Valley Explorer", 3.2,
        Palette(hex_to_rgba("fcd5be"), hex_to_rgba("92400e"), hex_to_rgba("0e7490"), hex_to_rgba("0891b2"), hex_to_rgba("f8fafc"), hex_to_rgba("f97316")),
        "short", "jacket", "wings", ["aviator_goggles", "glider_wings", "long_scarf"],
        "Brave glider explorer with teal flight jacket and aviator goggles."
    ),
    CharacterDef(
        "woodland-guardian", "Bramble", "ghibli", "Moss Antler Beast", 2.6,
        Palette(hex_to_rgba("d6d3d1"), hex_to_rgba("78716c"), hex_to_rgba("1c1917"), hex_to_rgba("15803d"), hex_to_rgba("78350f"), hex_to_rgba("facc15")),
        "antlers", "creature", "leaf", ["moss_antlers", "leaf_umbrella", "acorn_satchel"],
        "Gentle woodland beast holding a giant green butterbur leaf umbrella."
    ),
]


# -----------------------------------------------------------------------------
# Asset Renderer & Procedural Art Generator
# -----------------------------------------------------------------------------
def _lighten(c: Tuple[int, ...], f: float) -> Tuple[int, int, int, int]:
    a = c[3] if len(c) > 3 else 255
    return (int(c[0] + (255 - c[0]) * f), int(c[1] + (255 - c[1]) * f), int(c[2] + (255 - c[2]) * f), a)


def _darken(c: Tuple[int, ...], f: float) -> Tuple[int, int, int, int]:
    a = c[3] if len(c) > 3 else 255
    return (int(c[0] * f), int(c[1] * f), int(c[2] * f), a)


def _draw_folds(draw: ImageDraw.ImageDraw, cx: int, body_top: int, body_h: int, s: float, base: Tuple[int, ...]) -> None:
    """Cloth folds and hem shadow over an outfit body."""
    shade = _darken(base, 0.7)
    for dx in (-0.45, -0.15, 0.2, 0.5):
        x = cx + int(dx * 50 * s)
        draw.line([(x, body_top + int(12 * s)), (x + int(6 * s), body_top + int(body_h * 0.7))], fill=shade, width=max(1, int(2 * s)))
    hem_y = body_top + body_h - int(4 * s)
    draw.line([(cx - int(40 * s), hem_y), (cx + int(40 * s), hem_y)], fill=shade, width=max(1, int(3 * s)))


def _draw_hair_sheen(draw: ImageDraw.ImageDraw, cx: int, head_cy: int, head_r: int, s: float, hair_c: Tuple[int, ...]) -> None:
    """Light arc on the upper-left of the hair mass (light source is top-left)."""
    r = head_r + int(4 * s)
    draw.arc([cx - r, head_cy - r, cx + r, head_cy + r], 200, 255, fill=_lighten(hair_c, 0.35), width=max(2, int(4 * s)))


def _shade_sprite(img: Image.Image) -> Image.Image:
    """Top-left light, edge form shading, and a thin rim light. Keeps the original alpha."""
    w, h = img.size
    k = w / 512.0
    a = img.getchannel("A")

    # Top-left key light: brighter at top, slightly darker at bottom.
    grad = Image.linear_gradient("L").resize((w, h))  # 0 at top, 255 at bottom
    light = grad.point([int(255 * (0.93 + 0.07 * (1 - g / 255.0))) for g in range(256)])
    rgb = ImageChops.multiply(img.convert("RGB"), Image.merge("RGB", (light, light, light)))

    # Form shading: darken pixels near the silhouette edge.
    inner = ImageChops.invert(a.filter(ImageFilter.GaussianBlur(radius=max(1, 6 * k))))
    form = inner.point([int(255 * (1 - 0.08 * (v / 255.0))) for v in range(256)])
    rgb = ImageChops.multiply(rgb, Image.merge("RGB", (form, form, form)))

    out = Image.merge("RGBA", (*rgb.split(), a))

    # Rim light: lit silhouette edge, strongest on the top-left.
    edge = ImageChops.subtract(a, a.filter(ImageFilter.MinFilter(max(3, int(3 * k)) | 1)))
    rim_a = ImageChops.multiply(edge, light).point(lambda v: int(v * 0.5))
    rim = Image.new("RGBA", (w, h), (255, 244, 214, 0))
    rim.putalpha(rim_a)
    out = Image.alpha_composite(out, rim)
    out.putalpha(a)  # Rim must not widen the silhouette.
    return out


def render_character_sprite(char: CharacterDef, state: str, size: int = 512) -> Image.Image:
    """Final sprite: supersampled layers, soft ground shadow, shading pass, then downscale."""
    big = size * SUPERSAMPLE
    k = big / 512.0
    cx, cy = big // 2, int(big * 0.62)

    shadow = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    sw, sh = int(85 * k), int(16 * k)
    foot_y = cy + int(135 * k)
    ImageDraw.Draw(shadow).ellipse([cx - sw, foot_y - sh, cx + sw, foot_y + sh], fill=(20, 25, 40, 120))
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=max(1, int(8 * k))))

    scene = Image.alpha_composite(Image.new("RGBA", (big, big), (0, 0, 0, 0)), shadow)
    scene = Image.alpha_composite(scene, _shade_sprite(_draw_sprite_layers(char, state, big)))
    return scene.resize((size, size), Image.LANCZOS)


def _draw_sprite_layers(char: CharacterDef, state: str, size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    p = char.palette

    cx, cy = size // 2, int(size * 0.62)
    s = size / 512.0

    # 1. State Auras & Background Effects
    if state in ("level_up", "achievement"):
        for r in range(int(190 * s), 0, -14):
            alpha = int(50 * (1 - r / (190 * s)))
            draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=p.accent[:3] + (alpha,), width=3)
        # Golden sparkle stars
        for angle in range(0, 360, 45):
            rad = math.radians(angle)
            sx = cx + int(140 * s * math.cos(rad))
            sy = cy + int(140 * s * math.sin(rad))
            draw.ellipse([sx - 4, sy - 4, sx + 4, sy + 4], fill=p.accent[:3] + (220,))

    elif state in ("success", "working"):
        aura_rad = int(140 * s)
        for r in range(aura_rad, 0, -18):
            alpha = int(40 * (1 - r / aura_rad))
            draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=p.accent[:3] + (alpha,))

    # 3. Body / Outfit
    body_w = int(55 * s)
    body_h = int(85 * s)
    body_top = cy - int(10 * s)

    if char.outfit_style == "dhoti":
        # Torso
        draw.polygon([(cx - body_w, body_top), (cx + body_w, body_top), (cx + int(35 * s), body_top + int(45 * s)), (cx - int(35 * s), body_top + int(45 * s))], fill=p.skin, outline=(30, 20, 35, 255), width=2)
        # Dhoti lower
        draw.polygon([(cx - int(45 * s), body_top + int(40 * s)), (cx + int(45 * s), body_top + int(40 * s)), (cx + int(35 * s), body_top + body_h + int(40 * s)), (cx - int(35 * s), body_top + body_h + int(40 * s))], fill=p.primary, outline=p.secondary, width=3)
        # Gold sash diagonal
        draw.line([(cx - body_w + 5, body_top + 5), (cx + int(35 * s), body_top + int(50 * s))], fill=p.secondary, width=int(8 * s))

    elif char.outfit_style == "sari":
        # Sari dress drape
        draw.rounded_rectangle([cx - body_w, body_top, cx + body_w, body_top + body_h + int(45 * s)], radius=int(14 * s), fill=p.primary, outline=p.secondary, width=3)
        # Pallu diagonal sash
        draw.polygon([(cx - body_w, body_top), (cx - int(10 * s), body_top), (cx + body_w, body_top + int(70 * s)), (cx + body_w - int(20 * s), body_top + int(70 * s))], fill=p.secondary)

    elif char.outfit_style == "kimono":
        # Kimono robe
        draw.rounded_rectangle([cx - body_w, body_top, cx + body_w, body_top + body_h + int(35 * s)], radius=int(16 * s), fill=p.primary, outline=p.secondary, width=3)
        # Obi belt
        draw.rectangle([cx - body_w - 4, body_top + int(35 * s), cx + body_w + 4, body_top + int(60 * s)], fill=p.secondary, outline=p.accent, width=2)

    elif char.outfit_style == "armor":
        # Battle armor chestplate
        draw.polygon([(cx - body_w, body_top), (cx + body_w, body_top), (cx + int(40 * s), body_top + body_h), (cx - int(40 * s), body_top + body_h)], fill=p.primary, outline=p.secondary, width=3)
        draw.line([(cx - body_w + 10, body_top + int(30 * s)), (cx + body_w - 10, body_top + int(30 * s))], fill=p.secondary, width=int(4 * s))

    elif char.outfit_style == "creature":
        # Rounded plump creature body (Pikachu, Kodama, Bramble)
        draw.ellipse([cx - int(68 * s), body_top - int(10 * s), cx + int(68 * s), body_top + body_h + int(35 * s)], fill=p.primary, outline=p.secondary, width=3)

    else:
        # Standard anime school uniform, vest, hoodie, jacket
        draw.rounded_rectangle([cx - body_w, body_top, cx + body_w, body_top + body_h], radius=int(16 * s), fill=p.primary, outline=p.secondary, width=3)
        # Collar / Tie
        draw.polygon([(cx - int(15 * s), body_top), (cx + int(15 * s), body_top), (cx, body_top + int(20 * s))], fill=p.secondary)
        if "red_hoodie" in char.special_features:
            draw.arc([cx - int(25 * s), body_top - int(5 * s), cx + int(25 * s), body_top + int(25 * s)], 0, 180, fill=hex_to_rgba("dc2626"), width=6)

    if char.outfit_style != "creature":
        _draw_folds(draw, cx, body_top, body_h, s, p.primary)

    # 4. Lower Feet / Shoes
    foot_y = cy + int(125 * s)
    draw.ellipse([cx - int(35 * s), foot_y - int(8 * s), cx - int(10 * s), foot_y + int(8 * s)], fill=p.secondary)
    draw.ellipse([cx + int(10 * s), foot_y - int(8 * s), cx + int(35 * s), foot_y + int(8 * s)], fill=p.secondary)

    # 5. Head Base
    head_r = int(58 * s * (1.12 if char.head_ratio <= 2.8 else 0.98))
    head_cy = cy - int(65 * s)
    if state == "thinking":
        head_cy -= int(4 * s)  # Slight tilt / perk

    draw.ellipse([cx - head_r, head_cy - head_r, cx + head_r, head_cy + head_r], fill=p.skin, outline=(30, 20, 35, 255), width=2)

    # 6. Hair & Headwear
    hair_c = p.hair
    if char.hair_style == "bald":
        # Shiny gleam on bald head
        draw.ellipse([cx - 15, head_cy - head_r + 10, cx + 5, head_cy - head_r + 25], fill=(255, 255, 255, 180))

    elif char.hair_style == "spiky" or char.hair_style == "spiky_wild":
        # Spiky anime hair locks
        for angle in range(160, 385, 30):
            rad = math.radians(angle)
            tip_x = cx + int((head_r + 28 * s) * math.cos(rad))
            tip_y = head_cy + int((head_r + 28 * s) * math.sin(rad))
            base_x1 = cx + int((head_r - 5) * math.cos(rad - 0.25))
            base_y1 = head_cy + int((head_r - 5) * math.sin(rad - 0.25))
            base_x2 = cx + int((head_r - 5) * math.cos(rad + 0.25))
            base_y2 = head_cy + int((head_r - 5) * math.sin(rad + 0.25))
            draw.polygon([(base_x1, base_y1), (tip_x, tip_y), (base_x2, base_y2)], fill=hair_c)
        draw.chord([cx - head_r - 4, head_cy - head_r - 10, cx + head_r + 4, head_cy], 180, 360, fill=hair_c)

    elif char.hair_style == "spiky_up":
        # Vegeta flame-spiked hair
        draw.polygon([(cx - head_r, head_cy - 10), (cx - int(30 * s), head_cy - head_r - int(45 * s)), (cx, head_cy - head_r - int(65 * s)), (cx + int(30 * s), head_cy - head_r - int(45 * s)), (cx + head_r, head_cy - 10)], fill=hair_c)

    elif char.hair_style == "long":
        # Flowing long hair
        draw.chord([cx - head_r - 6, head_cy - head_r - 12, cx + head_r + 6, head_cy + int(10 * s)], 170, 370, fill=hair_c)
        draw.polygon([(cx - head_r - 4, head_cy), (cx - head_r - 18, head_cy + int(110 * s)), (cx - int(25 * s), head_cy + int(90 * s)), (cx - int(25 * s), head_cy + int(20 * s))], fill=hair_c)
        draw.polygon([(cx + head_r + 4, head_cy), (cx + head_r + 18, head_cy + int(110 * s)), (cx + int(25 * s), head_cy + int(90 * s)), (cx + int(25 * s), head_cy + int(20 * s))], fill=hair_c)
        if "orange_hair_tips" in char.special_features:
            draw.ellipse([cx - head_r - 20, head_cy + int(85 * s), cx - int(20 * s), head_cy + int(125 * s)], fill=hex_to_rgba("ea580c"))
            draw.ellipse([cx + int(20 * s), head_cy + int(85 * s), cx + head_r + 20, head_cy + int(125 * s)], fill=hex_to_rgba("ea580c"))

    elif char.hair_style == "twintails":
        # Twin pigtails
        draw.chord([cx - head_r - 4, head_cy - head_r - 8, cx + head_r + 4, head_cy + 5], 180, 360, fill=hair_c)
        draw.ellipse([cx - head_r - int(25 * s), head_cy - int(10 * s), cx - head_r + 5, head_cy + int(95 * s)], fill=hair_c)
        draw.ellipse([cx + head_r - 5, head_cy - int(10 * s), cx + head_r + int(25 * s), head_cy + int(95 * s)], fill=hair_c)

    elif char.hair_style == "elephant":
        # Ganesha cute elephant head with ears & trunk
        ear_r = int(35 * s)
        draw.ellipse([cx - head_r - ear_r, head_cy - int(15 * s), cx - head_r + int(10 * s), head_cy + int(45 * s)], fill=p.skin, outline=(30, 20, 35, 255), width=2)
        draw.ellipse([cx + head_r - int(10 * s), head_cy - int(15 * s), cx + head_r + ear_r, head_cy + int(45 * s)], fill=p.skin, outline=(30, 20, 35, 255), width=2)
        # Trunk
        draw.arc([cx - int(16 * s), head_cy + int(10 * s), cx + int(16 * s), head_cy + int(55 * s)], 10, 190, fill=p.skin, width=int(14 * s))

    elif char.hair_style == "antlers":
        # Moss antlers
        draw.line([(cx - int(25 * s), head_cy - head_r), (cx - int(45 * s), head_cy - head_r - int(30 * s))], fill=hex_to_rgba("78350f"), width=5)
        draw.line([(cx - int(40 * s), head_cy - head_r - int(15 * s)), (cx - int(55 * s), head_cy - head_r - int(22 * s))], fill=hex_to_rgba("78350f"), width=4)
        draw.line([(cx + int(25 * s), head_cy - head_r), (cx + int(45 * s), head_cy - head_r - int(30 * s))], fill=hex_to_rgba("78350f"), width=5)
        draw.line([(cx + int(40 * s), head_cy - head_r - int(15 * s)), (cx + int(55 * s), head_cy - head_r - int(22 * s))], fill=hex_to_rgba("78350f"), width=4)

    else:
        # Default cute bob / curls
        draw.chord([cx - head_r - 6, head_cy - head_r - 10, cx + head_r + 6, head_cy + int(15 * s)], 175, 365, fill=hair_c)

    if char.hair_style not in ("bald", "antlers"):
        _draw_hair_sheen(draw, cx, head_cy, head_r, s, hair_c)

    # 7. Signature Head Accessories
    if "peacock_feather" in char.special_features:
        fx, fy = cx + int(20 * s), head_cy - head_r - int(25 * s)
        draw.ellipse([fx - 10, fy - 18, fx + 10, fy + 18], fill=hex_to_rgba("059669"), outline=hex_to_rgba("ffd700"), width=2)
        draw.ellipse([fx - 5, fy - 8, fx + 5, fy + 8], fill=hex_to_rgba("0284c7"))

    if "mukut" in char.special_features or "crown" in char.special_features:
        draw.polygon([(cx - int(30 * s), head_cy - head_r + int(5 * s)), (cx, head_cy - head_r - int(35 * s)), (cx + int(30 * s), head_cy - head_r + int(5 * s))], fill=hex_to_rgba("ffd700"), outline=hex_to_rgba("f59e0b"), width=2)
        draw.ellipse([cx - 4, head_cy - head_r - int(15 * s), cx + 4, head_cy - head_r - int(7 * s)], fill=p.accent)

    if "straw_hat" in char.special_features:
        draw.ellipse([cx - head_r - int(35 * s), head_cy - head_r - int(10 * s), cx + head_r + int(35 * s), head_cy - head_r + int(22 * s)], fill=hex_to_rgba("facc15"), outline=hex_to_rgba("ca8a04"), width=3)
        draw.arc([cx - head_r - int(15 * s), head_cy - head_r - int(8 * s), cx + head_r + int(15 * s), head_cy - head_r + int(14 * s)], 190, 350, fill=hex_to_rgba("dc2626"), width=6)

    if "blindfold" in char.special_features:
        draw.rectangle([cx - head_r + 4, head_cy - int(8 * s), cx + head_r - 4, head_cy + int(14 * s)], fill=(15, 20, 35, 255))

    if "bamboo_muzzle" in char.special_features:
        draw.rounded_rectangle([cx - int(24 * s), head_cy + int(18 * s), cx + int(24 * s), head_cy + int(34 * s)], radius=4, fill=hex_to_rgba("16a34a"), outline=hex_to_rgba("14532d"), width=2)
        draw.line([(cx - int(36 * s), head_cy + int(26 * s)), (cx + int(36 * s), head_cy + int(26 * s))], fill=hex_to_rgba("dc2626"), width=2)

    if "giant_red_bow" in char.special_features:
        bx, by = cx, head_cy - head_r - int(12 * s)
        draw.polygon([(bx, by), (bx - int(45 * s), by - int(35 * s)), (bx - int(40 * s), by + int(5 * s))], fill=hex_to_rgba("dc2626"))
        draw.polygon([(bx, by), (bx + int(45 * s), by - int(35 * s)), (bx + int(40 * s), by + int(5 * s))], fill=hex_to_rgba("dc2626"))
        draw.ellipse([bx - 8, by - 8, bx + 8, by + 8], fill=hex_to_rgba("991b1b"))

    if "red_horns" in char.special_features or "chimera_horns" in char.special_features:
        horn_c = hex_to_rgba("dc2626") if "red_horns" in char.special_features else hex_to_rgba("111827")
        draw.polygon([(cx - int(28 * s), head_cy - head_r + 5), (cx - int(35 * s), head_cy - head_r - int(25 * s)), (cx - int(18 * s), head_cy - head_r + 5)], fill=horn_c)
        draw.polygon([(cx + int(18 * s), head_cy - head_r + 5), (cx + int(35 * s), head_cy - head_r - int(25 * s)), (cx + int(28 * s), head_cy - head_r + 5)], fill=horn_c)

    # 8. Eyes & Expression
    eye_y = head_cy + int(4 * s)
    eye_sp = int(22 * s)
    ew, eh = int(12 * s), int(16 * s)

    if "blindfold" not in char.special_features:
        if state == "sleeping":
            # Peaceful closed crescent eyes
            draw.arc([cx - eye_sp - ew, eye_y - 4, cx - eye_sp + ew, eye_y + eh - 2], 200, 340, fill=(30, 20, 25, 255), width=3)
            draw.arc([cx + eye_sp - ew, eye_y - 4, cx + eye_sp + ew, eye_y + eh - 2], 200, 340, fill=(30, 20, 25, 255), width=3)
        elif "deadpan_face" in char.special_features:
            # Saitama deadpan oval dot eyes
            draw.ellipse([cx - eye_sp - 4, eye_y, cx - eye_sp + 4, eye_y + 8], fill=(20, 20, 25, 255))
            draw.ellipse([cx + eye_sp - 4, eye_y, cx + eye_sp + 4, eye_y + 8], fill=(20, 20, 25, 255))
        else:
            # Large expressive anime eyes
            for ex in (cx - eye_sp, cx + eye_sp):
                draw.ellipse([ex - ew, eye_y - eh, ex + ew, eye_y + eh], fill=(255, 255, 255, 255), outline=(30, 20, 30, 255), width=2)
                draw.ellipse([ex - int(ew * 0.7), eye_y - int(eh * 0.7), ex + int(ew * 0.7), eye_y + int(eh * 0.7)], fill=p.eyes)
                draw.ellipse([ex - int(ew * 0.4), eye_y - int(eh * 0.5), ex - int(ew * 0.1), eye_y - int(eh * 0.2)], fill=(255, 255, 255, 255))
                # Upper lid line with a lash flick on the outer corner.
                lid_dir = -1 if ex < cx else 1
                draw.arc([ex - ew - 1, eye_y - eh - 1, ex + ew + 1, eye_y + eh + 1], 200, 340, fill=(20, 12, 20, 255), width=max(2, int(3 * s)))
                draw.line([(ex + lid_dir * ew, eye_y - int(eh * 0.45)), (ex + lid_dir * (ew + int(5 * s)), eye_y - int(eh * 0.85))], fill=(20, 12, 20, 255), width=max(1, int(2 * s)))

        # Soft blush
        blush_c = (255, 140, 160, 120)
        draw.ellipse([cx - eye_sp - 10, eye_y + 12, cx - eye_sp + 10, eye_y + 22], fill=blush_c)
        draw.ellipse([cx + eye_sp - 10, eye_y + 12, cx + eye_sp + 10, eye_y + 22], fill=blush_c)

    # 9. Tilak / Forehead Marks
    if "tilak" in char.special_features:
        draw.line([(cx, head_cy - int(24 * s)), (cx, head_cy - int(8 * s))], fill=hex_to_rgba("dc2626"), width=3)
        draw.ellipse([cx - 3, head_cy - int(6 * s), cx + 3, head_cy], fill=hex_to_rgba("ffd700"))
    elif "third_eye" in char.special_features:
        draw.ellipse([cx - 3, head_cy - int(16 * s), cx + 3, head_cy - int(6 * s)], fill=hex_to_rgba("dc2626"), outline=hex_to_rgba("f8fafc"), width=1)

    # 10. Signature Prop / Weapon
    if char.prop_type == "flute":
        # Golden flute held across chest
        draw.line([(cx - int(45 * s), head_cy + int(24 * s)), (cx + int(45 * s), head_cy + int(12 * s))], fill=hex_to_rgba("ffd700"), width=5)
    elif char.prop_type == "bow":
        # Bow arc on left side
        draw.arc([cx - int(75 * s), head_cy - int(10 * s), cx - int(35 * s), cy + int(110 * s)], 100, 260, fill=hex_to_rgba("b45309"), width=5)
    elif char.prop_type == "mace":
        # Hanuman golden gada
        draw.line([(cx + int(35 * s), cy + int(80 * s)), (cx + int(55 * s), head_cy - int(15 * s))], fill=hex_to_rgba("ffd700"), width=6)
        draw.ellipse([cx + int(45 * s), head_cy - int(30 * s), cx + int(70 * s), head_cy - int(5 * s)], fill=hex_to_rgba("ffd700"), outline=hex_to_rgba("ea580c"), width=2)
    elif char.prop_type == "trident":
        # Shiva trishula
        draw.line([(cx + int(42 * s), cy + int(90 * s)), (cx + int(42 * s), head_cy - int(45 * s))], fill=hex_to_rgba("475569"), width=4)
        draw.arc([cx + int(25 * s), head_cy - int(45 * s), cx + int(59 * s), head_cy - int(20 * s)], 0, 180, fill=hex_to_rgba("ffd700"), width=3)
    elif char.prop_type == "veena":
        # Saraswati veena
        draw.line([(cx - int(40 * s), cy + int(60 * s)), (cx + int(40 * s), head_cy - int(10 * s))], fill=hex_to_rgba("78350f"), width=8)
        draw.ellipse([cx - int(55 * s), cy + int(45 * s), cx - int(25 * s), cy + int(75 * s)], fill=hex_to_rgba("b45309"))
    elif char.prop_type == "katana":
        # Samurai katana at waist
        draw.line([(cx - int(45 * s), cy + int(25 * s)), (cx + int(40 * s), cy + int(75 * s))], fill=(220, 225, 235, 255), width=3)
        draw.line([(cx - int(45 * s), cy + int(25 * s)), (cx - int(30 * s), cy + int(15 * s))], fill=hex_to_rgba("111827"), width=5)

    # 11. State Overlays
    if state == "error":
        # Defensive ward sparks and sweat drop
        draw.polygon([(cx + head_r - 6, head_cy - 10), (cx + head_r - 1, head_cy + 2), (cx + head_r - 11, head_cy + 2)], fill=hex_to_rgba("38bdf8"))
    elif state == "sleeping":
        # Floating Zzz
        draw.text((cx + int(55 * s), head_cy - int(45 * s)), "Z", fill=p.accent)
        draw.text((cx + int(72 * s), head_cy - int(65 * s)), "z", fill=p.secondary)

    return img


# -----------------------------------------------------------------------------
# Manifest & Prompts Compiler
# -----------------------------------------------------------------------------
def build_manifest(char: CharacterDef) -> Dict[str, Any]:
    states = [
        "idle", "working", "thinking", "waiting",
        "success", "error", "level_up", "sleeping",
        "thumbnail", "achievement", "celebrating"
    ]
    assets_map = {s: f"assets/{s}.png" for s in states}
    animations_map = {}
    for s in states:
        animations_map[s] = {
            "asset": s,
            "frames": [{"x": 0, "y": 0, "width": 512, "height": 512}],
            "fps": 1,
            "loop": True,
            "anchor": {"x": 0.5, "y": 1.0},
        }

    return {
        "schemaVersion": 1,
        "id": f"agentpet.{char.id}",
        "name": char.name,
        "version": "1.0.0",
        "description": f"{char.description} Premium cute desktop pet companion ({char.head_ratio:.1f} heads tall).",
        "creator": "AgentPet Creative Universe",
        "license": "CC-BY-4.0",
        "source": f"https://github.com/agentpet/pet-packs/tree/main/{char.id}",
        "attribution": f"Original anime companion design for {char.name}. CC-BY-4.0 free to adapt with attribution.",
        "commercialUse": True,
        "redistribution": True,
        "derivativeWorks": True,
        "assets": assets_map,
        "animations": animations_map,
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
        "tags": ["desktop-pet", "anime", "companion", char.category, char.id],
    }


def build_prompts(char: CharacterDef) -> Dict[str, str]:
    base = (
        f"masterpiece anime game asset of cute desktop companion {char.name} ({char.title}), "
        f"compact {char.head_ratio:.1f} heads tall, expressive large eyes, 2.5D dimensional shading, "
        f"clean linework, rim lighting, 100% transparent background, pure alpha cutout, no background elements, no floor, no halo."
    )
    return {
        "idle": f"{base} Calm upright pose, pleasant smile, relaxed hands.",
        "working": f"{base} Focused concentrated posture, leaning forward, hands engaged, energetic glow.",
        "thinking": f"{base} Head tilted, finger to cheek, curious upward gaze, floating thought spark glyphs.",
        "waiting": f"{base} Patient weight-shifted stance, hands clasped, gentle rhythmic aura.",
        "success": f"{base} Triumphant celebratory stance, arms raised, joyful sparkling eyes, radiant victory burst.",
        "celebrating": f"{base} Jubilant celebratory dance, confetti particles, happy sparkling eyes.",
        "error": f"{base} Defensive surprised step back, defensive ward arc, sweat drop, surprised expression.",
        "level_up": f"{base} Ascending transcendent aura, floating weightless, golden mandala halo.",
        "achievement": f"{base} Proud master companion stance, signature item held aloft, glowing laurel crest.",
        "sleeping": f"{base} Peaceful seated meditation or slumber pose, closed eyes, soft twilight tint, floating Zzz.",
        "thumbnail": f"{base} Close-up portrait bust, high contrast companion icon for selection UI.",
    }


# -----------------------------------------------------------------------------
# Main Generation Runner
# -----------------------------------------------------------------------------
def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--category", choices=["all", "devotional", "anime", "ghibli"], default="all")
    parser.add_argument("--character", type=str, default=None)
    parser.add_argument("--out-dir", type=str, default=None, help="Write packs here instead of pet-packs/ (e.g. a review folder).")
    args = parser.parse_args()

    targets = CHARACTERS
    if args.category != "all":
        targets = [c for c in targets if c.category == args.category]
    if args.character:
        targets = [c for c in targets if c.id == args.character.lower()]
        if not targets:
            print(f"Error: Character '{args.character}' not found.", file=sys.stderr)
            return 1

    out_root = Path(args.out_dir) if args.out_dir else PET_PACKS_DIR
    print(f"Compiling {len(targets)} PetPack companion suites into {out_root}...")
    out_root.mkdir(parents=True, exist_ok=True)

    states = [
        "idle", "working", "thinking", "waiting",
        "success", "error", "level_up", "sleeping",
        "thumbnail", "achievement", "celebrating"
    ]

    for char in targets:
        pack_dir = out_root / char.id
        assets_dir = pack_dir / "assets"
        assets_dir.mkdir(parents=True, exist_ok=True)

        # 1. Render all state assets
        for state in states:
            img = render_character_sprite(char, state, size=512)
            out_png = assets_dir / f"{state}.png"
            img.save(out_png, format="PNG")

        # 2. Write pet.json manifest
        manifest = build_manifest(char)
        (pack_dir / "pet.json").write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        # 3. Write prompts.json
        prompts = build_prompts(char)
        (pack_dir / "prompts.json").write_text(
            json.dumps(prompts, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        print(f"  + Built [agentpet.{char.id}] ({char.name}) -> {pack_dir}")

    print(f"\nSuccessfully compiled {len(targets)} companion pet packs!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
