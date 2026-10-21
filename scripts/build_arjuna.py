import os
import math
from PIL import Image, ImageDraw, ImageFilter

def create_radial_gradient(size, inner_color, outer_color):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    center = size / 2.0
    max_radius = size / 2.0
    for r in range(int(max_radius), 0, -2):
        ratio = r / max_radius
        # linear interpolate RGBA
        c = tuple(
            int(inner_color[i] * (1 - ratio) + outer_color[i] * ratio)
            for i in range(4)
        )
        draw.ellipse([center - r, center - r, center + r, center + r], fill=c)
    return img

def ensure_dirs(pack_name):
    base = os.path.join("pet-packs", pack_name, "assets")
    os.makedirs(base, exist_ok=True)
    return base

# --- ARJUNA GENERATOR ---
def generate_arjuna():
    assets_dir = ensure_dirs("arjuna")
    
    # Palette
    INDIGO_DARK = (10, 10, 32, 255)
    INDIGO_ARMOR = (25, 28, 65, 255)
    SAFFRON = (255, 153, 51, 255)
    SAFFRON_GLOW = (255, 180, 80, 200)
    GOLD = (255, 215, 0, 255)
    GOLD_LIGHT = (255, 240, 130, 255)
    SKIN = (245, 210, 185, 255)
    SKIN_SHADOW = (220, 180, 155, 255)
    BOW_WOOD = (180, 110, 40, 255)
    CYAN_ENERGY = (0, 240, 255, 255)
    WHITE = (255, 255, 255, 255)

    states = [
        "idle", "working", "thinking", "waiting",
        "success", "celebrating", "error", "level_up",
        "achievement", "sleeping", "thumbnail"
    ]

    for state in states:
        img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)

        # Background subtle celestial aura
        if state in ["working", "success", "celebrating", "level_up", "achievement"]:
            aura_color = (255, 215, 0, 60) if state != "level_up" else (255, 153, 51, 90)
            aura = create_radial_gradient(460, aura_color, (0, 0, 0, 0))
            img.paste(aura, (26, 26), aura)

        # Base coordinates
        cx, cy = 256, 310

        # Sleeping State
        if state == "sleeping":
            # Meditation sitting pose asleep
            # Body
            draw.ellipse([cx - 70, cy + 20, cx + 70, cy + 120], fill=INDIGO_ARMOR, outline=GOLD, width=3)
            # Saffron sash draped
            draw.chord([cx - 65, cy + 30, cx + 65, cy + 110], 0, 180, fill=SAFFRON)
            # Head tilted resting
            draw.ellipse([cx - 45, cy - 60, cx + 45, cy + 30], fill=SKIN, outline=INDIGO_DARK, width=2)
            # Closed eyes (arcs)
            draw.arc([cx - 30, cy - 15, cx - 10, cy], 0, 180, fill=INDIGO_DARK, width=3)
            draw.arc([cx + 10, cy - 15, cx + 30, cy], 0, 180, fill=INDIGO_DARK, width=3)
            # Gentle smile
            draw.arc([cx - 10, cy + 10, cx + 10, cy + 20], 0, 180, fill=INDIGO_DARK, width=2)
            # Mukut (crown)
            draw.polygon([(cx - 40, cy - 50), (cx, cy - 100), (cx + 40, cy - 50), (cx, cy - 40)], fill=GOLD, outline=GOLD_LIGHT, width=2)
            # Bow resting beside
            draw.arc([cx + 60, cy - 70, cx + 120, cy + 130], 270, 90, fill=BOW_WOOD, width=6)
            # Floating ZZZ
            draw.text((cx + 50, cy - 120), "Z", fill=SAFFRON, font_size=28)
            draw.text((cx + 75, cy - 150), "z", fill=GOLD, font_size=22)
            draw.text((cx + 95, cy - 175), "z", fill=GOLD_LIGHT, font_size=16)

        elif state == "thumbnail":
            # Close-up hero bust
            tcx, tcy = 256, 270
            # Armor collar
            draw.polygon([(tcx - 140, 512), (tcx + 140, 512), (tcx + 110, tcy + 110), (tcx, tcy + 135), (tcx - 110, tcy + 110)], fill=INDIGO_ARMOR, outline=GOLD, width=4)
            # Saffron collar trim
            draw.line([(tcx - 90, tcy + 110), (tcx, tcy + 150), (tcx + 90, tcy + 110)], fill=SAFFRON, width=8)
            # Neck
            draw.rectangle([tcx - 45, tcy + 40, tcx + 45, tcy + 115], fill=SKIN_SHADOW)
            # Head
            draw.ellipse([tcx - 95, tcy - 80, tcx + 95, tcy + 85], fill=SKIN, outline=INDIGO_DARK, width=3)
            # Fierce noble eyes
            draw.polygon([(tcx - 65, tcy), (tcx - 45, tcy - 12), (tcx - 20, tcy), (tcx - 45, tcy + 8)], fill=WHITE, outline=INDIGO_DARK, width=2)
            draw.polygon([(tcx + 20, tcy), (tcx + 45, tcy - 12), (tcx + 65, tcy), (tcx + 45, tcy + 8)], fill=WHITE, outline=INDIGO_DARK, width=2)
            draw.ellipse([tcx - 48, tcy - 8, tcx - 36, tcy + 4], fill=INDIGO_DARK)
            draw.ellipse([tcx + 36, tcy - 8, tcx + 48, tcy + 4], fill=INDIGO_DARK)
            # Eyebrows
            draw.line([(tcx - 70, tcy - 18), (tcx - 20, tcy - 10)], fill=INDIGO_DARK, width=4)
            draw.line([(tcx + 20, tcy - 10), (tcx + 70, tcy - 18)], fill=INDIGO_DARK, width=4)
            # Tilak on forehead
            draw.line([(tcx, tcy - 50), (tcx, tcy - 25)], fill=SAFFRON, width=5)
            draw.ellipse([tcx - 4, tcy - 20, tcx + 4, tcy - 12], fill=GOLD)
            # Mukut (Grand Royal Crown)
            draw.polygon([
                (tcx - 90, tcy - 70), (tcx - 55, tcy - 140), (tcx, tcy - 200),
                (tcx + 55, tcy - 140), (tcx + 90, tcy - 70), (tcx, tcy - 60)
            ], fill=GOLD, outline=GOLD_LIGHT, width=4)
            # Jewel in crown
            draw.ellipse([tcx - 16, tcy - 130, tcx + 16, tcy - 98], fill=CYAN_ENERGY, outline=WHITE, width=2)
            # Earrings (Kundala)
            draw.ellipse([tcx - 105, tcy + 15, tcx - 90, tcy + 45], fill=GOLD, outline=GOLD_LIGHT, width=2)
            draw.ellipse([tcx + 90, tcy + 15, tcx + 105, tcy + 45], fill=GOLD, outline=GOLD_LIGHT, width=2)

        else:
            # Full body standing/action pose
            # Quiver on back with arrows
            draw.polygon([(cx + 35, cy - 60), (cx + 65, cy - 80), (cx + 80, cy + 70), (cx + 50, cy + 80)], fill=GOLD, outline=GOLD_LIGHT, width=2)
            # Quiver arrows (tied to quota metaphor)
            num_arrows = 5 if state != "error" else 2
            for i in range(num_arrows):
                ox = cx + 45 + i * 6
                oy = cy - 75 - i * 4
                draw.line([(ox, oy), (ox + 12, oy - 22)], fill=GOLD_LIGHT, width=3)
                # Arrow fletching
                draw.line([(ox + 8, oy - 15), (ox + 5, oy - 18)], fill=SAFFRON, width=2)
                draw.line([(ox + 11, oy - 20), (ox + 8, oy - 23)], fill=SAFFRON, width=2)

            # Legs / Dhoti
            draw.polygon([(cx - 45, cy + 60), (cx - 15, cy + 180), (cx - 45, cy + 180)], fill=INDIGO_ARMOR, outline=GOLD, width=2)
            draw.polygon([(cx + 45, cy + 60), (cx + 15, cy + 180), (cx + 45, cy + 180)], fill=INDIGO_ARMOR, outline=GOLD, width=2)
            # Golden boots
            draw.rectangle([cx - 48, cy + 175, cx - 12, cy + 195], fill=GOLD)
            draw.rectangle([cx + 12, cy + 175, cx + 48, cy + 195], fill=GOLD)

            # Torso / Armor
            draw.polygon([(cx - 50, cy - 20), (cx + 50, cy - 20), (cx + 40, cy + 70), (cx - 40, cy + 70)], fill=INDIGO_ARMOR, outline=GOLD, width=3)
            # Saffron sash diagonally across chest
            draw.polygon([(cx - 48, cy - 18), (cx - 28, cy - 20), (cx + 40, cy + 65), (cx + 25, cy + 70)], fill=SAFFRON)

            # Head & Face
            hy = cy - 65
            draw.ellipse([cx - 35, hy - 40, cx + 35, hy + 35], fill=SKIN, outline=INDIGO_DARK, width=2)
            # Eyes
            if state in ["error"]:
                # squinting/pain
                draw.line([(cx - 25, hy - 5), (cx - 10, hy - 2)], fill=INDIGO_DARK, width=3)
                draw.line([(cx + 10, hy - 2), (cx + 25, hy - 5)], fill=INDIGO_DARK, width=3)
            elif state in ["success", "celebrating"]:
                # happy curved eyes
                draw.arc([cx - 25, hy - 12, cx - 8, hy + 2], 180, 360, fill=INDIGO_DARK, width=3)
                draw.arc([cx + 8, hy - 12, cx + 25, hy + 2], 180, 360, fill=INDIGO_DARK, width=3)
                draw.arc([cx - 10, hy + 8, cx + 10, hy + 22], 0, 180, fill=INDIGO_DARK, width=2)
            else:
                draw.ellipse([cx - 22, hy - 6, cx - 12, hy + 4], fill=INDIGO_DARK)
                draw.ellipse([cx + 12, hy - 6, cx + 22, hy + 4], fill=INDIGO_DARK)
                draw.ellipse([cx - 18, hy - 5, cx - 14, hy - 1], fill=WHITE)
                draw.ellipse([cx + 16, hy - 5, cx + 20, hy - 1], fill=WHITE)

            # Tilak
            draw.line([(cx, hy - 30), (cx, hy - 15)], fill=SAFFRON, width=3)

            # Mukut (Crown)
            draw.polygon([(cx - 32, hy - 35), (cx, hy - 80), (cx + 32, hy - 35), (cx, hy - 30)], fill=GOLD, outline=GOLD_LIGHT, width=2)
            draw.ellipse([cx - 6, hy - 60, cx + 6, hy - 48], fill=CYAN_ENERGY)

            # Specific State Props & Poses:
            if state == "working":
                # Aiming Gandiva bow drawn taut!
                # Bow arc
                draw.arc([cx - 120, cy - 130, cx + 20, cy + 90], 100, 260, fill=BOW_WOOD, width=7)
                # Bowstring pulled back to cheek
                draw.line([(cx - 70, cy - 110), (cx + 20, cy - 40)], fill=GOLD_LIGHT, width=2)
                draw.line([(cx - 70, cy + 70), (cx + 20, cy - 40)], fill=GOLD_LIGHT, width=2)
                # Golden blazing arrow of light
                draw.line([(cx - 100, cy - 40), (cx + 25, cy - 40)], fill=GOLD_LIGHT, width=4)
                # Arrowhead
                draw.polygon([(cx - 118, cy - 40), (cx - 100, cy - 48), (cx - 100, cy - 32)], fill=CYAN_ENERGY)
                # Arm drawing string
                draw.line([(cx + 30, cy - 10), (cx + 20, cy - 40)], fill=SKIN, width=12)

            elif state == "waiting":
                # Panchajanya Conch Shell held high!
                # Left hand holding conch to chest/lips
                # Conch shell geometry (white spiral with gold trim)
                shx, shy = cx - 55, cy - 30
                draw.ellipse([shx - 22, shy - 25, shx + 22, shy + 25], fill=WHITE, outline=GOLD, width=3)
                draw.polygon([(shx + 10, shy - 20), (shx + 40, shy - 10), (shx + 20, shy + 15)], fill=WHITE, outline=GOLD, width=2)
                # Soundwaves pulsing from conch
                draw.arc([shx + 40, shy - 30, shx + 70, shy + 10], 300, 60, fill=CYAN_ENERGY, width=3)
                draw.arc([shx + 55, shy - 45, shx + 95, shy + 25], 300, 60, fill=GOLD_LIGHT, width=3)
                # Gandiva slung on right shoulder
                draw.arc([cx + 30, cy - 100, cx + 110, cy + 100], 280, 80, fill=BOW_WOOD, width=6)

            elif state == "thinking":
                # Golden Sudarshana / Chakra wheel contemplation glyph floating above head
                chx, chy = cx, cy - 120
                draw.ellipse([chx - 30, chy - 30, chx + 30, chy + 30], outline=GOLD_LIGHT, width=3)
                for a in range(0, 360, 45):
                    rad = math.radians(a)
                    draw.line([(chx, chy), (chx + int(28 * math.cos(rad)), chy + int(28 * math.sin(rad)))], fill=GOLD, width=2)
                # Hand on chin
                draw.line([(cx - 20, cy + 10), (cx - 15, cy - 45)], fill=SKIN, width=10)
                # Gandiva upright
                draw.arc([cx - 85, cy - 120, cx - 15, cy + 100], 100, 260, fill=BOW_WOOD, width=6)

            elif state in ["success", "celebrating"]:
                # Gandiva raised high in triumph!
                draw.arc([cx - 40, cy - 200, cx + 80, cy - 20], 30, 220, fill=BOW_WOOD, width=7)
                draw.line([(cx - 10, cy - 10), (cx + 20, cy - 110)], fill=SKIN, width=12) # arm reaching up
                # Sparkling stars
                for sx, sy in [(cx - 90, cy - 120), (cx + 100, cy - 140), (cx - 70, cy - 40), (cx + 90, cy - 30)]:
                    draw.line([(sx - 10, sy), (sx + 10, sy)], fill=GOLD_LIGHT, width=2)
                    draw.line([(sx, sy - 10), (sx, sy + 10)], fill=GOLD_LIGHT, width=2)

            elif state == "error":
                # Defensive guard with bow deflecting sparks
                draw.arc([cx - 80, cy - 80, cx - 10, cy + 100], 110, 250, fill=BOW_WOOD, width=8)
                # Energy deflection shield (amber/red sparks)
                draw.arc([cx - 95, cy - 90, cx - 35, cy + 90], 100, 260, fill=(255, 80, 50, 220), width=4)
                # Sparks
                for spx, spy in [(cx - 110, cy - 40), (cx - 100, cy - 10), (cx - 120, cy + 20)]:
                    draw.ellipse([spx - 4, spy - 4, spx + 4, spy + 4], fill=(255, 200, 50, 255))

            elif state in ["level_up", "achievement"]:
                # Divine Golden Avatar transformation!
                # Radiant mandala rings behind head
                draw.ellipse([cx - 70, cy - 135, cx + 70, cy + 5], outline=GOLD_LIGHT, width=3)
                draw.ellipse([cx - 90, cy - 155, cx + 90, cy + 25], outline=SAFFRON, width=2)
                # Raised both arms holding bow and celestial arrow
                draw.arc([cx - 30, cy - 180, cx + 80, cy - 20], 30, 220, fill=BOW_WOOD, width=7)
                draw.line([(cx - 30, cy - 10), (cx - 60, cy - 90)], fill=SKIN, width=11)
                draw.line([(cx + 30, cy - 10), (cx + 60, cy - 90)], fill=SKIN, width=11)
                draw.line([(cx - 80, cy - 100), (cx - 40, cy - 80)], fill=CYAN_ENERGY, width=4)

            else: # idle
                # Standing calm with Gandiva resting vertically beside him
                draw.arc([cx - 85, cy - 120, cx - 15, cy + 100], 100, 260, fill=BOW_WOOD, width=6)
                # Bowstring
                draw.line([(cx - 50, cy - 115), (cx - 50, cy + 95)], fill=GOLD_LIGHT, width=1)
                # Hand resting on bow
                draw.line([(cx - 30, cy - 10), (cx - 50, cy - 15)], fill=SKIN, width=10)

        # Save image
        img.save(os.path.join(assets_dir, f"{state}.png"))
    print("Arjuna assets generated successfully.")

generate_arjuna()
