import os
import math
import json
from PIL import Image, ImageDraw

def ensure_dirs(pack_name):
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    base = os.path.join(repo_root, "pet-packs", pack_name, "assets")
    os.makedirs(base, exist_ok=True)
    return base

# --- SUN WUKONG GENERATOR ---
def generate_sun_wukong():
    assets_dir = ensure_dirs("sun-wukong")
    
    RED = (230, 57, 70, 255)
    RED_DARK = (180, 30, 45, 255)
    GOLD = (255, 183, 3, 255)
    GOLD_LIGHT = (255, 215, 80, 255)
    GOLD_AURA = (255, 200, 30, 180)
    FUR = (215, 150, 95, 255)
    FUR_FACE = (250, 215, 185, 255)
    STAFF_IRON = (35, 40, 50, 255)
    CLOUD_WHITE = (245, 248, 255, 230)
    CLOUD_SHADOW = (210, 220, 240, 230)
    WHITE = (255, 255, 255, 255)
    BLACK = (20, 20, 25, 255)

    states = [
        "idle", "working", "thinking", "waiting",
        "success", "celebrating", "error", "level_up",
        "achievement", "sleeping", "thumbnail"
    ]

    for state in states:
        img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)

        cx, cy = 256, 290

        # Helper to draw Somersault Cloud
        def draw_cloud(clx, cly, scale=1.0):
            # Fluffy cloud puffs
            draw.ellipse([clx - 110*scale, cly + 10*scale, clx + 110*scale, cly + 90*scale], fill=CLOUD_SHADOW)
            draw.ellipse([clx - 90*scale, cly - 10*scale, clx - 10*scale, cly + 70*scale], fill=CLOUD_WHITE)
            draw.ellipse([clx + 10*scale, cly - 15*scale, clx + 95*scale, cly + 65*scale], fill=CLOUD_WHITE)
            draw.ellipse([clx - 50*scale, cly - 25*scale, clx + 50*scale, cly + 65*scale], fill=CLOUD_WHITE)
            # Cloud swirl trail
            draw.arc([clx - 130*scale, cly + 20*scale, clx - 80*scale, cly + 70*scale], 90, 270, fill=GOLD, width=3)

        if state == "sleeping":
            draw_cloud(cx, cy + 30, scale=1.2)
            # Curled up Wukong sleeping on the cloud
            draw.ellipse([cx - 55, cy - 20, cx + 55, cy + 50], fill=GOLD, outline=RED, width=3)
            # Head tucked
            draw.ellipse([cx - 50, cy - 35, cx + 15, cy + 25], fill=FUR)
            draw.ellipse([cx - 40, cy - 25, cx + 5, cy + 18], fill=FUR_FACE)
            # Closed happy eye
            draw.arc([cx - 28, cy - 10, cx - 10, cy + 5], 0, 180, fill=BLACK, width=3)
            # Golden headband
            draw.line([(cx - 48, cy - 22), (cx + 5, cy - 18)], fill=GOLD_LIGHT, width=4)
            # Staff resting against cloud
            draw.line([(cx - 100, cy + 60), (cx + 90, cy - 60)], fill=STAFF_IRON, width=8)
            draw.line([(cx - 100, cy + 60), (cx - 75, cy + 40)], fill=GOLD, width=9)
            draw.line([(cx + 65, cy - 40), (cx + 90, cy - 60)], fill=GOLD, width=9)
            # Zzz
            draw.text((cx + 50, cy - 80), "Z", fill=GOLD, font_size=28)
            draw.text((cx + 75, cy - 110), "z", fill=GOLD_LIGHT, font_size=22)

        elif state == "thumbnail":
            # Close up portrait
            tcx, tcy = 256, 260
            # Red scarf collar
            draw.polygon([(tcx - 120, 512), (tcx + 120, 512), (tcx + 90, tcy + 130), (tcx, tcy + 150), (tcx - 90, tcy + 130)], fill=RED, outline=GOLD, width=4)
            # Fur Head
            draw.ellipse([tcx - 90, tcy - 85, tcx + 90, tcy + 85], fill=FUR, outline=BLACK, width=2)
            # Monkey Heart Face Shape
            draw.polygon([
                (tcx, tcy + 70), (tcx - 65, tcy + 45), (tcx - 65, tcy - 30),
                (tcx - 30, tcy - 50), (tcx, tcy - 20), (tcx + 30, tcy - 50),
                (tcx + 65, tcy - 30), (tcx + 65, tcy + 45)
            ], fill=FUR_FACE)
            # Monkey ears
            draw.ellipse([tcx - 105, tcy - 20, tcx - 80, tcy + 25], fill=FUR)
            draw.ellipse([tcx - 100, tcy - 12, tcx - 85, tcy + 18], fill=RED_DARK)
            draw.ellipse([tcx + 80, tcy - 20, tcx + 105, tcy + 25], fill=FUR)
            draw.ellipse([tcx + 85, tcy - 12, tcx + 100, tcy + 18], fill=RED_DARK)
            # Blazing Golden Eyes (Huoyan Jinjing)
            draw.ellipse([tcx - 50, tcy - 12, tcx - 18, tcy + 14], fill=GOLD_AURA, outline=GOLD_LIGHT, width=2)
            draw.ellipse([tcx + 18, tcy - 12, tcx + 50, tcy + 14], fill=GOLD_AURA, outline=GOLD_LIGHT, width=2)
            draw.ellipse([tcx - 38, tcy - 5, tcx - 28, tcy + 7], fill=RED_DARK)
            draw.ellipse([tcx + 28, tcy - 5, tcx + 38, tcy + 7], fill=RED_DARK)
            draw.ellipse([tcx - 34, tcy - 3, tcx - 30, tcy + 2], fill=WHITE)
            draw.ellipse([tcx + 32, tcy - 3, tcx + 36, tcy + 2], fill=WHITE)
            # Nose & Confident Grin
            draw.ellipse([tcx - 8, tcy + 22, tcx + 8, tcy + 34], fill=BLACK)
            draw.arc([tcx - 25, tcy + 35, tcx + 25, tcy + 55], 0, 180, fill=BLACK, width=3)
            # Golden Circlet Headband (Tightening Crown)
            draw.arc([tcx - 85, tcy - 70, tcx + 85, tcy - 20], 190, 350, fill=GOLD, width=7)
            # Center curl on circlet
            draw.arc([tcx - 14, tcy - 72, tcx + 14, tcy - 45], 160, 380, fill=GOLD_LIGHT, width=5)

        else:
            # Cloud base for all action states
            draw_cloud(cx, cy + 110, scale=1.0)

            # Body / Tiger skirt & gold robe
            draw.polygon([(cx - 38, cy + 20), (cx + 38, cy + 20), (cx + 45, cy + 110), (cx - 45, cy + 110)], fill=GOLD, outline=BLACK, width=2)
            # Tiger stripes
            draw.line([(cx - 35, cy + 50), (cx - 15, cy + 55)], fill=BLACK, width=4)
            draw.line([(cx + 35, cy + 75), (cx + 15, cy + 80)], fill=BLACK, width=4)
            # Red sash
            draw.rectangle([cx - 40, cy + 15, cx + 40, cy + 32], fill=RED)

            # Head
            hy = cy - 35
            draw.ellipse([cx - 35, hy - 35, cx + 35, hy + 35], fill=FUR)
            # Face heart
            draw.polygon([
                (cx, hy + 28), (cx - 25, hy + 18), (cx - 25, hy - 12),
                (cx - 12, hy - 20), (cx, hy - 8), (cx + 12, hy - 20),
                (cx + 25, hy - 12), (cx + 25, hy + 18)
            ], fill=FUR_FACE)
            # Ears
            draw.ellipse([cx - 42, hy - 10, cx - 32, hy + 12], fill=FUR)
            draw.ellipse([cx + 32, hy - 10, cx + 42, hy + 12], fill=FUR)
            # Eyes
            if state == "error":
                draw.line([(cx - 20, hy - 2), (cx - 8, hy + 2)], fill=BLACK, width=3)
                draw.line([(cx + 8, hy + 2), (cx + 20, hy - 2)], fill=BLACK, width=3)
            elif state in ["success", "celebrating"]:
                draw.arc([cx - 22, hy - 8, cx - 8, hy + 4], 180, 360, fill=BLACK, width=3)
                draw.arc([cx + 8, hy - 8, cx + 22, hy + 4], 180, 360, fill=BLACK, width=3)
                draw.arc([cx - 12, hy + 8, cx + 12, hy + 22], 0, 180, fill=BLACK, width=2)
            else:
                draw.ellipse([cx - 20, hy - 5, cx - 8, hy + 6], fill=GOLD_AURA)
                draw.ellipse([cx + 8, hy - 5, cx + 20, hy + 6], fill=GOLD_AURA)
                draw.ellipse([cx - 16, hy - 2, cx - 11, hy + 3], fill=BLACK)
                draw.ellipse([cx + 11, hy - 2, cx + 16, hy + 3], fill=BLACK)

            # Golden circlet
            draw.line([(cx - 32, hy - 22), (cx + 32, hy - 22)], fill=GOLD, width=4)
            draw.arc([cx - 6, hy - 32, cx + 6, hy - 18], 150, 390, fill=GOLD_LIGHT, width=3)

            # Props / Actions
            if state == "working":
                # Rapidly spinning staff creating glowing gold wheel!
                draw.ellipse([cx - 120, cy - 90, cx + 120, cy + 70], outline=GOLD_AURA, width=5)
                draw.line([(cx - 100, cy - 80), (cx + 100, cy + 60)], fill=STAFF_IRON, width=7)
                draw.line([(cx - 110, cy - 90), (cx - 80, cy - 65)], fill=GOLD, width=9)
                draw.line([(cx + 80, cy + 45), (cx + 110, cy + 70)], fill=GOLD, width=9)

            elif state == "waiting":
                # Shading eyes looking into the horizon
                draw.line([(cx - 15, cy - 10), (cx - 25, hy - 10)], fill=FUR, width=8)
                draw.line([(cx - 30, hy - 12), (cx, hy - 14)], fill=FUR, width=8) # hand over brow
                # Staff resting upright
                draw.line([(cx + 50, cy - 100), (cx + 50, cy + 110)], fill=STAFF_IRON, width=7)
                draw.line([(cx + 50, cy - 100), (cx + 50, cy - 70)], fill=GOLD, width=8)
                draw.line([(cx + 50, cy + 80), (cx + 50, cy + 110)], fill=GOLD, width=8)

            elif state == "thinking":
                # Scratching cheek
                draw.line([(cx + 15, cy - 10), (cx + 25, hy + 10)], fill=FUR, width=8)
                # Staff across back
                draw.line([(cx - 80, cy - 20), (cx + 80, cy + 20)], fill=STAFF_IRON, width=7)

            elif state in ["success", "celebrating"]:
                # High leap, staff overhead
                draw.line([(cx - 100, cy - 110), (cx + 100, cy - 110)], fill=STAFF_IRON, width=7)
                draw.line([(cx - 100, cy - 110), (cx - 70, cy - 110)], fill=GOLD, width=8)
                draw.line([(cx + 70, cy - 110), (cx + 100, cy - 110)], fill=GOLD, width=8)
                # Fireworks
                for fx, fy in [(cx - 90, cy - 80), (cx + 90, cy - 80), (cx, cy - 150)]:
                    draw.line([(fx - 8, fy), (fx + 8, fy)], fill=GOLD_LIGHT, width=2)
                    draw.line([(fx, fy - 8), (fx, fy + 8)], fill=RED, width=2)

            elif state == "error":
                # Singed smoke puff
                draw.ellipse([cx - 40, cy - 110, cx + 40, cy - 60], fill=(60, 60, 70, 200))
                draw.line([(cx - 60, cy + 50), (cx + 60, cy + 70)], fill=STAFF_IRON, width=7)

            elif state in ["level_up", "achievement"]:
                # Massive golden beam / Great Sage power
                draw.ellipse([cx - 110, cy - 120, cx + 110, cy + 100], outline=GOLD_LIGHT, width=4)
                draw.line([(cx, cy - 170), (cx, cy + 110)], fill=STAFF_IRON, width=12)
                draw.line([(cx, cy - 170), (cx, cy - 120)], fill=GOLD, width=14)
                draw.line([(cx, cy + 60), (cx, cy + 110)], fill=GOLD, width=14)

            else: # idle
                # Staff resting diagonally across shoulders
                draw.line([(cx - 90, cy - 60), (cx + 80, cy + 40)], fill=STAFF_IRON, width=7)
                draw.line([(cx - 90, cy - 60), (cx - 65, cy - 40)], fill=GOLD, width=8)
                draw.line([(cx + 55, cy + 20), (cx + 80, cy + 40)], fill=GOLD, width=8)

        img.save(os.path.join(assets_dir, f"{state}.png"))
    print("Sun Wukong assets generated successfully.")


# --- NINJA CAT GENERATOR ---
def generate_ninja_cat():
    assets_dir = ensure_dirs("ninja-cat")

    SLATE_DARK = (17, 25, 39, 255)
    SLATE_MID = (31, 41, 55, 255)
    CYAN_NEON = (0, 240, 255, 255)
    CYAN_GLOW = (0, 240, 255, 140)
    PINK_INNER = (255, 150, 180, 255)
    WHITE = (255, 255, 255, 255)
    GOLD = (255, 215, 0, 255)
    BLACK = (20, 20, 25, 255)
    SCROLL_PAPER = (240, 230, 200, 255)

    states = [
        "idle", "working", "thinking", "waiting",
        "success", "celebrating", "error", "level_up",
        "achievement", "sleeping", "thumbnail"
    ]

    for state in states:
        img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)

        cx, cy = 256, 310

        if state == "sleeping":
            # Curled into a neat cat ball with ninja cowl
            draw.ellipse([cx - 75, cy - 15, cx + 75, cy + 85], fill=SLATE_DARK, outline=CYAN_NEON, width=2)
            # Tail curled around front with neon tip
            draw.arc([cx - 65, cy + 10, cx + 65, cy + 80], 0, 180, fill=SLATE_MID, width=16)
            draw.arc([cx + 35, cy + 30, cx + 65, cy + 75], 0, 90, fill=CYAN_NEON, width=16)
            # Cat ears poking out
            draw.polygon([(cx - 55, cy - 10), (cx - 70, cy - 45), (cx - 35, cy - 25)], fill=SLATE_DARK)
            draw.polygon([(cx - 20, cy - 25), (cx + 5, cy - 45), (cx - 5, cy - 10)], fill=SLATE_DARK)
            # Zzz in cyan
            draw.text((cx + 45, cy - 60), "Z", fill=CYAN_NEON, font_size=26)
            draw.text((cx + 70, cy - 90), "z", fill=CYAN_NEON, font_size=20)

        elif state == "thumbnail":
            tcx, tcy = 256, 260
            # Ninja cowl collar
            draw.polygon([(tcx - 130, 512), (tcx + 130, 512), (tcx + 100, tcy + 120), (tcx, tcy + 140), (tcx - 100, tcy + 120)], fill=SLATE_DARK, outline=CYAN_NEON, width=3)
            # Head base
            draw.ellipse([tcx - 90, tcy - 70, tcx + 90, tcy + 80], fill=SLATE_DARK)
            # Big Cat Ears
            draw.polygon([(tcx - 85, tcy - 50), (tcx - 105, tcy - 145), (tcx - 30, tcy - 65)], fill=SLATE_DARK)
            draw.polygon([(tcx - 80, tcy - 55), (tcx - 98, tcy - 135), (tcx - 38, tcy - 70)], fill=PINK_INNER)
            draw.polygon([(tcx + 30, tcy - 65), (tcx + 105, tcy - 145), (tcx + 85, tcy - 50)], fill=SLATE_DARK)
            draw.polygon([(tcx + 38, tcy - 70), (tcx + 98, tcy - 135), (tcx + 80, tcy - 55)], fill=PINK_INNER)
            # Cyber Visor (Sleek horizontal visor)
            draw.polygon([
                (tcx - 75, tcy - 15), (tcx + 75, tcy - 15),
                (tcx + 65, tcy + 25), (tcx - 65, tcy + 25)
            ], fill=CYAN_NEON)
            # Visor digital eyes
            draw.ellipse([tcx - 48, tcy - 5, tcx - 24, tcy + 15], fill=WHITE)
            draw.ellipse([tcx + 24, tcy - 5, tcx + 48, tcy + 15], fill=WHITE)
            # Whisker marks (neon)
            draw.line([(tcx - 85, tcy + 35), (tcx - 60, tcy + 38)], fill=CYAN_NEON, width=2)
            draw.line([(tcx - 88, tcy + 50), (tcx - 60, tcy + 48)], fill=CYAN_NEON, width=2)
            draw.line([(tcx + 60, tcy + 38), (tcx + 85, tcy + 35)], fill=CYAN_NEON, width=2)
            draw.line([(tcx + 60, tcy + 48), (tcx + 88, tcy + 50)], fill=CYAN_NEON, width=2)

        else:
            # Action body
            # Crossed mini katanas on back
            draw.line([(cx - 65, cy - 70), (cx + 65, cy + 60)], fill=(160, 170, 180, 255), width=4)
            draw.line([(cx - 75, cy - 80), (cx - 55, cy - 60)], fill=CYAN_NEON, width=6) # hilt
            draw.line([(cx + 65, cy - 70), (cx - 65, cy + 60)], fill=(160, 170, 180, 255), width=4)
            draw.line([(cx + 75, cy - 80), (cx + 55, cy - 60)], fill=CYAN_NEON, width=6) # hilt

            # Body / Cloak
            draw.polygon([(cx - 45, cy), (cx + 45, cy), (cx + 50, cy + 120), (cx - 50, cy + 120)], fill=SLATE_DARK, outline=SLATE_MID, width=2)
            # Cyber belt with cyan pouch
            draw.rectangle([cx - 46, cy + 60, cx + 46, cy + 74], fill=SLATE_MID)
            draw.rectangle([cx - 12, cy + 58, cx + 12, cy + 76], fill=CYAN_NEON)

            # Tail with cyan glowing tip
            draw.arc([cx + 30, cy + 40, cx + 110, cy + 125], 270, 90, fill=SLATE_DARK, width=12)
            draw.arc([cx + 50, cy + 40, cx + 110, cy + 100], 270, 360, fill=CYAN_NEON, width=12)

            # Head
            hy = cy - 40
            draw.ellipse([cx - 50, hy - 40, cx + 50, hy + 40], fill=SLATE_DARK)
            # Ears
            draw.polygon([(cx - 48, hy - 25), (cx - 58, hy - 75), (cx - 18, hy - 35)], fill=SLATE_DARK)
            draw.polygon([(cx - 44, hy - 28), (cx - 52, hy - 70), (cx - 22, hy - 36)], fill=PINK_INNER)
            draw.polygon([(cx + 18, hy - 35), (cx + 58, hy - 75), (cx + 48, hy - 25)], fill=SLATE_DARK)
            draw.polygon([(cx + 22, hy - 36), (cx + 52, hy - 70), (cx + 44, hy - 28)], fill=PINK_INNER)

            # Cyber Visor
            draw.polygon([(cx - 42, hy - 10), (cx + 42, hy - 10), (cx + 36, hy + 15), (cx - 36, hy + 15)], fill=CYAN_NEON)
            if state == "error":
                # Glitch crosses in visor
                draw.line([(cx - 26, hy - 4), (cx - 14, hy + 8)], fill=BLACK, width=2)
                draw.line([(cx - 14, hy - 4), (cx - 26, hy + 8)], fill=BLACK, width=2)
                draw.line([(cx + 14, hy - 4), (cx + 26, hy + 8)], fill=BLACK, width=2)
                draw.line([(cx + 26, hy - 4), (cx + 14, hy + 8)], fill=BLACK, width=2)
            elif state in ["success", "celebrating"]:
                draw.arc([cx - 28, hy - 8, cx - 12, hy + 8], 180, 360, fill=WHITE, width=3)
                draw.arc([cx + 12, hy - 8, cx + 28, hy + 8], 180, 360, fill=WHITE, width=3)
            else:
                draw.ellipse([cx - 26, hy - 5, cx - 14, hy + 8], fill=WHITE)
                draw.ellipse([cx + 14, hy - 5, cx + 26, hy + 8], fill=WHITE)

            # Props / Actions
            if state == "working":
                # Dual paw claws slicing glowing cyber data streams
                draw.line([(cx - 90, cy - 30), (cx - 20, cy + 20)], fill=CYAN_NEON, width=3)
                draw.line([(cx - 70, cy - 40), (cx, cy + 10)], fill=CYAN_NEON, width=3)
                draw.line([(cx + 90, cy - 30), (cx + 20, cy + 20)], fill=CYAN_NEON, width=3)
                draw.line([(cx + 70, cy - 40), (cx, cy + 10)], fill=CYAN_NEON, width=3)

            elif state == "thinking":
                # Unrolled glowing cyber scroll
                draw.rectangle([cx - 60, cy + 30, cx + 60, cy + 65], fill=SCROLL_PAPER, outline=CYAN_NEON, width=2)
                # Code runes on scroll
                draw.line([(cx - 45, cy + 42), (cx + 45, cy + 42)], fill=CYAN_NEON, width=2)
                draw.line([(cx - 45, cy + 52), (cx + 25, cy + 52)], fill=CYAN_NEON, width=2)

            elif state == "waiting":
                # Exclamation mark above head
                draw.rectangle([cx - 4, hy - 110, cx + 4, hy - 85], fill=CYAN_NEON)
                draw.ellipse([cx - 4, hy - 78, cx + 4, hy - 70], fill=CYAN_NEON)

            elif state in ["success", "celebrating"]:
                # Dual katanas drawn and held upward
                draw.line([(cx - 30, cy), (cx - 70, cy - 80)], fill=(200, 240, 255, 255), width=5)
                draw.line([(cx + 30, cy), (cx + 70, cy - 80)], fill=(200, 240, 255, 255), width=5)
                # Sakura embers
                for sx, sy in [(cx - 90, cy - 60), (cx + 90, cy - 40), (cx - 50, cy - 110)]:
                    draw.ellipse([sx - 4, sy - 4, sx + 4, sy + 4], fill=PINK_INNER)

            elif state == "error":
                # Classic substitution decoy log
                draw.rectangle([cx - 40, cy + 40, cx + 40, cy + 115], fill=(130, 85, 45, 255), outline=BLACK, width=2)
                # Rings on top of log
                draw.ellipse([cx - 40, cy + 30, cx + 40, cy + 50], fill=(180, 130, 80, 255), outline=BLACK, width=2)

            elif state in ["level_up", "achievement"]:
                # Supercharged electric cyan aura
                draw.ellipse([cx - 95, cy - 90, cx + 95, cy + 100], outline=CYAN_NEON, width=3)
                # Lightning zaps
                draw.line([(cx - 75, cy - 60), (cx - 45, cy - 30), (cx - 65, cy)], fill=CYAN_NEON, width=3)
                draw.line([(cx + 75, cy - 60), (cx + 45, cy - 30), (cx + 65, cy)], fill=CYAN_NEON, width=3)

        img.save(os.path.join(assets_dir, f"{state}.png"))
    print("Ninja Cat assets generated successfully.")

if __name__ == "__main__":
    generate_sun_wukong()
    generate_ninja_cat()
