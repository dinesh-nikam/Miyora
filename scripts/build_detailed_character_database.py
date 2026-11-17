import json
import os

characters = [
    # -------------------------------------------------------------
    # 1. ANIME: Tanjiro Kamado
    # -------------------------------------------------------------
    {
        "id": "char.tanjiro_kamado",
        "slug": "tanjiro",
        "name": "Tanjiro Kamado",
        "native_name": "竈門 炭治郎",
        "origin_lore": "Demon Slayer: Kimetsu no Yaiba",
        "creator": "Koyoharu Gotouge",
        "archetype": "Kind-Hearted Swordsman & Sun Breaker",
        "category": "anime_character",
        "canonical_stats": {
            "age": 16,
            "birthday": "July 14",
            "height_cm": 165,
            "weight_kg": 61,
            "zodiac": "Cancer",
            "blood_type": "A"
        },
        "micro_physical_details": {
            "skin_tone": "#f5d0be",
            "skin_description": "Sun-tanned warm olive with rough calloused hands from coal-cutting and swordsmanship",
            "eyes": {
                "color_base": "#8b1e1e",
                "color_highlight": "#e05252",
                "pupil_style": "Dark red irises with pale white bottom reflections, wide and empathetic",
                "expression_default": "Determined warmth with unwavering honesty"
            },
            "hair": {
                "base_color": "#171717",
                "tip_gradient": "#881337",
                "style": "Spiky combed-back raven-black hair with prominent crimson/burgundy tips",
                "length": "Short-to-medium nape-length",
                "physics_stiffness": 0.65
            },
            "distinguishing_marks": [
                "Scar on top-left forehead (originally burn mark, evolved into flame Demon Slayer Mark)",
                "Rectangular Hanafuda earrings depicting red rising sun over mountains",
                "Calloused palms with pronounced sword-grip muscle development"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Standard black Demon Slayer gakuran jacket with brass buttons and white collared shirt underneath",
            "outer_wear": "Loose black-and-sea-foam-green checkered (ichimatsu-pattern) haori coat",
            "bottom_wear": "Baggy hakama-style chocho-zubon trousers tucked into white kyahan wraps",
            "footwear": "White tabi socks with black cords fastened to woven straw zōri sandals",
            "relics_and_gear": [
                "Large pine-wood traveling box with metal brass latches (housing Nezuko during daylight)",
                "Black Nichirin Katana with circular black-and-silver spoked wheel guard (flame-red in combat)"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Two-handed katana grip with index-finger trigger extension"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 110°]",
                "knee_flexion_limit": "145° (enables deep Sun Breathing lunge stances)",
                "ankle_mobility": "Multi-axis inversion for slope stability"
            },
            "spring_bone_colliders": ["haori_left", "haori_right", "earrings", "front_hair_bangs"],
            "blendshapes": ["joy", "angry", "sorrow", "fun", "determined", "sun_breath_focus", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Tenacious Bug Hunting & Red-Green TDD Verification",
            "motto": "No matter how many tests fail, I will never give up on clean code.",
            "state_reactions": {
                "Idle": "Gentle standing breath with hands resting lightly on sword hilt, hair softly swaying",
                "Working": "Rigorous rapid typing posture leaning forward with focused scarlet eyes",
                "Thinking": "Fingers tap chin thoughtfully while forehead mark emits a subtle warm amber glow",
                "Success": "Triumphant two-handed fist pump, radiant squinting closed-eye smile",
                "Error": "Deep sorrowful gasp, steadying breath, then resolute stance ready to fix the failure"
            }
        }
    },

    # -------------------------------------------------------------
    # 2. ANIME: Nezuko Kamado
    # -------------------------------------------------------------
    {
        "id": "char.nezuko_kamado",
        "slug": "nezuko",
        "name": "Nezuko Kamado",
        "native_name": "竈門 禰豆子",
        "origin_lore": "Demon Slayer: Kimetsu no Yaiba",
        "creator": "Koyoharu Gotouge",
        "archetype": "Protective Demon Maiden & Silent Guardian",
        "category": "anime_character",
        "canonical_stats": {
            "age": 14,
            "birthday": "December 28",
            "height_cm": 153,
            "weight_kg": 45,
            "zodiac": "Capricorn",
            "blood_type": "Unknown"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf2ea",
            "skin_description": "Porcelain alabaster skin with faint leaf-vein patterns during combat awakenings",
            "eyes": {
                "color_base": "#f472b6",
                "color_highlight": "#fbcfe8",
                "pupil_style": "Soft pale pink irises with white limbal rings; narrow cat-like vertical slits when alerted",
                "expression_default": "Gentle, wide innocent gaze with slight head tilt"
            },
            "hair": {
                "base_color": "#111827",
                "tip_gradient": "#ea580c",
                "style": "Long wavy black hair parted left, transitioning into glowing flame-vermilion beyond elbows",
                "length": "Past waist level with soft loose curls",
                "physics_stiffness": 0.45
            },
            "distinguishing_marks": [
                "Hollow natural green bamboo muzzle secured with crimson cords around lower jaw",
                "Sharp pink-lacquered fingernails functioning as stiletto claw extensions",
                "Small pink silk ribbon tied on the left side of front bangs"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Traditional pale-pink kimono patterned with geometric asanoha (hemp leaf) stars",
            "outer_wear": "Heavy knee-length dark chocolate-brown wool haori coat",
            "bottom_wear": "Checkerboard red-and-white obi sash tied tightly at the waist with bright orange cord",
            "footwear": "White tabi socks wrapped with dark brown calf-gaiters and straw zōri with pink thongs",
            "relics_and_gear": [
                "Bamboo muzzle with hemp-tied side tassels",
                "Blood Demon Art: Pyrokinesis pink flame aura emitters"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid",
            "bone_count": 206,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Soft cupped protective hands with tapered claw morph targets"
            },
            "leg_articulation": {
                "hip_swing_range": "[-35°, 90°]",
                "knee_flexion_limit": "135° (allows full seiza kneeling pose)",
                "ankle_mobility": "Elastic ankle joint with smooth sock-deform weighting"
            },
            "spring_bone_colliders": ["long_hair_left", "long_hair_right", "haori_bottom", "bamboo_ribbons"],
            "blendshapes": ["joy", "blink", "blink_l", "blink_r", "demon_slits", "pat_head", "surprised", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AvatarSample_A.vrm",
            "thumbnail_url": "./public/thumbnails/AvatarSample_A.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Quiet Background Daemon Monitoring & Health Telemetry",
            "motto": "Mmh-mmh! (Protecting the build workspace from regressions while you rest).",
            "state_reactions": {
                "Idle": "Curious side-to-side head tilt, blinking with gentle eye crinkles and bamboo gently bobbing",
                "Working": "Small focused crouching pose watching log streams with bright pink eyes",
                "Thinking": "Tilts head 45 degrees, blinking inquisitively at terminal outputs",
                "Success": "Delighted patting of hands together, eyes curve into cheerful crescents",
                "Error": "Puffs cheeks slightly, eyes narrow into protective defensive slits"
            }
        }
    },

    # -------------------------------------------------------------
    # 3. ANIME: Satoru Gojo
    # -------------------------------------------------------------
    {
        "id": "char.satoru_gojo",
        "slug": "gojo",
        "name": "Satoru Gojo",
        "native_name": "五条 悟",
        "origin_lore": "Jujutsu Kaisen",
        "creator": "Gege Akutami",
        "archetype": "The Strongest Jujutsu Sorcerer & Limitless Mentor",
        "category": "anime_character",
        "canonical_stats": {
            "age": 28,
            "birthday": "December 7",
            "height_cm": 191,
            "weight_kg": 82,
            "zodiac": "Sagittarius",
            "blood_type": "O"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf8f4",
            "skin_description": "Fair alabaster skin, sculpted angular jawline and lean athletic swimmer build",
            "eyes": {
                "color_base": "#06b6d4",
                "color_highlight": "#e0f2fe",
                "pupil_style": "The legendary 'Six Eyes' — electric luminous cerulean blue with endless crystalline fractals",
                "expression_default": "Confident smirk or playful grin masking supreme omnipotent depth"
            },
            "hair": {
                "base_color": "#ffffff",
                "tip_gradient": "#e2e8f0",
                "style": "Pure snow-white hair, dramatically spiked upward under blindfold, falling casually into soft bangs when uncovered",
                "length": "Medium textured locks",
                "physics_stiffness": 0.55
            },
            "distinguishing_marks": [
                "Matte black sensory-suppression blindfold tightly wrapped around upper face",
                "Slim circular dark wire sunglasses used during casual teaching sessions",
                "Long, remarkably slender pianist fingers capable of intricate hand seals"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Fitted black compression undershirt",
            "outer_wear": "High-collared midnight-navy Tokyo Jujutsu High zip-up tunic with silver crest buttons",
            "bottom_wear": "Slim-tailored matching midnight-navy trousers",
            "footwear": "Polished black leather dress boots with low block heels",
            "relics_and_gear": [
                "Signature black silk blindfold",
                "Domain Expansion Seal: Crossing right middle finger over index finger for 'Infinite Void'"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Tall Slim Male",
            "bone_count": 129,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Infinite Void finger crossover pose with micro-flexion joints"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "150°",
                "ankle_mobility": "Full 360° rotational pivot for acrobatic kicks"
            },
            "spring_bone_colliders": ["spiky_hair", "collar_rim", "tunic_waist"],
            "blendshapes": ["smirk", "grin", "confident", "serious", "six_eyes_glow", "blink", "wink_r", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Sendagaya_Shibu.vrm",
            "thumbnail_url": "./public/thumbnails/Sendagaya_Shibu.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Infinite Pipeline Architect & Critical System Performance Optimization",
            "motto": "Don't worry. After all, our architecture is the strongest.",
            "state_reactions": {
                "Idle": "Casual hands-in-pockets slouch, head tilted back with confident swagger",
                "Working": "Fast effortless keyboard wizardry, occasionally twirling a pen with one hand",
                "Thinking": "Lowers blindfold slightly with index finger, glowing cyan eye inspecting code complexity",
                "Success": "Flashes peace sign with a grin: 'See? Perfectly optimized, as expected.'",
                "Error": "Smirks arrogantly: 'A deadlock? How cute. Let me dismantle it in three seconds.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 4. DEITY: Lord Shiva (Mahadeva)
    # -------------------------------------------------------------
    {
        "id": "char.lord_shiva",
        "slug": "shiva",
        "name": "Lord Shiva (Mahadeva)",
        "native_name": "शिव (महादेव)",
        "origin_lore": "Sanatana Dharma / Vedic & Puranic Cosmology",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "The Supreme Ascetic, Lord of Dance & Cosmic Transformer",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Timeless / Eternal (Anadi-Ananta)",
            "birthday": "Maha Shivaratri (Phalguna Krishna Chaturdashi)",
            "height_cm": 188,
            "weight_kg": 78,
            "zodiac": "Cosmic Timekeeper (Mahakala)",
            "blood_type": "Divine Nectar (Amrita)"
        },
        "micro_physical_details": {
            "skin_tone": "#85a7cc",
            "skin_description": "Translucent cosmic sky-blue hue smeared with sacred white Vibhuti ash; throat is deep midnight-blue (Neelakantha)",
            "eyes": {
                "color_base": "#1e3a8a",
                "color_highlight": "#38bdf8",
                "pupil_style": "Half-open meditative gaze resting at eyebrow center, serene and infinitely deep",
                "expression_default": "Tranquil transcendent bliss (Sat-Chit-Ananda)"
            },
            "hair": {
                "base_color": "#1c1917",
                "tip_gradient": "#44403c",
                "style": "Towering matted ascetic coils (Jata-Mukuta) crowned with the silvery crescent moon (Chandrama)",
                "length": "Long locked coils with holy River Ganga flowing from the crown",
                "physics_stiffness": 0.70
            },
            "distinguishing_marks": [
                "Horizontal Third Eye (Trinetra) on forehead, keeper of spiritual wisdom and cosmic dissolution",
                "Tripundra: Three horizontal white Vibhuti ash lines across forehead centered with red vermilion bindu",
                "King Cobra (Vasuki) draped fearlessly around blue throat in threefold coils"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Simple saffron ascetic dhoti undergarment",
            "outer_wear": "Draped Bengal tiger skin wrapped around hips, symbolizing conquest over raw animal instincts",
            "bottom_wear": "Bare feet planted on Mount Kailash lotus pedestal or ground",
            "footwear": "Bare ascetic feet adorned with sacred Rudraksha anklets",
            "relics_and_gear": [
                "Trishula (Golden Trident): Symbol of threefold cosmic powers (Creation, Preservation, Dissolution)",
                "Damaru (Hourglass drum): Symbol of cosmic primordial sound vibration (Nada-Brahma)",
                "Rosary of 108 Sacred Rudraksha beads strung around neck, forearms, and wrists",
                "Kamandalu (Sacred ascetic water pot)"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Majestic Celestial Biped",
            "bone_count": 65,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Abhaya Mudra (fear-dispelling open right palm) + Trishula spear grasp in left hand"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 130°]",
                "knee_flexion_limit": "150° (allows full Padmasana / Lotus meditation posture and Nataraja dance pose)",
                "ankle_mobility": "Grounded solid poise"
            },
            "spring_bone_colliders": ["ganga_stream", "crescent_moon", "vasuki_snake", "rudraksha_necklaces"],
            "blendshapes": ["meditative", "peaceful", "trinetra_open", "compassionate", "tandava_fury", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Avatar_Orion.vrm",
            "thumbnail_url": "./public/thumbnails/Avatar_Orion.svg"
        },
        "miyora_companion_role": {
            "work_specialty": "Zero-Downtime Database Migrations & Irreversible Architecture Transformations",
            "motto": "Om Namah Shivaya: Dissolving legacy tech debt so robust systems can be reborn.",
            "state_reactions": {
                "Idle": "Calm seated Padmasana meditation with Trishula planted vertically, gentle Ganga water glinting",
                "Working": "Smooth purposeful keystrokes, deep resonant aura illuminating the monitor",
                "Thinking": "Eyes gently close in dhyana, crescent moon shines soft silver light over problem space",
                "Success": "Abhaya mudra hand raises in divine blessing, Damaru vibrates with rhythmic resonance",
                "Error": "Third eye glows with quiet golden flame, calmly dissolving the faulty stack trace"
            }
        }
    },

    # -------------------------------------------------------------
    # 5. DEITY: Lord Krishna (Murlidhar)
    # -------------------------------------------------------------
    {
        "id": "char.lord_krishna",
        "slug": "krishna",
        "name": "Lord Krishna (Murlidhar)",
        "native_name": "कृष्ण (मुरलीधर)",
        "origin_lore": "Mahabharata, Bhagavad Gita, Srimad Bhagavatam",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "The Divine Flutist, Master of Yoga & Supreme Strategist",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Eternal Youth (Navayauvana)",
            "birthday": "Janmashtami (Bhadrapada Krishna Ashtami)",
            "height_cm": 182,
            "weight_kg": 72,
            "zodiac": "Rohini Nakshatra",
            "blood_type": "Divine Grace"
        },
        "micro_physical_details": {
            "skin_tone": "#587fa8",
            "skin_description": "Shyam-varna: All-attractive deep rain-cloud blue/azure complexion radiating warm celestial luster",
            "eyes": {
                "color_base": "#1e293b",
                "color_highlight": "#38bdf8",
                "pupil_style": "Lotus-petal shaped eyes (Kamalanayana) with playful yet omniscient golden depth",
                "expression_default": "Enchanting gentle smile brimming with eternal joy and playful wisdom"
            },
            "hair": {
                "base_color": "#0f172a",
                "tip_gradient": "#1e293b",
                "style": "Lush glossy raven curls crowned by an ornate golden Mukut surmounted by a peacock feather (Mor Pankh)",
                "length": "Shoulder-length silky dark curls",
                "physics_stiffness": 0.40
            },
            "distinguishing_marks": [
                "Iridescent peacock feather (Mor Pankh) standing upright in the golden crown",
                "Kaustubha Gem and Srivatsa mark radiating from center of broad chest",
                "Tilaka mark of sacred Chandan (sandalwood paste) gracing forehead bridge"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Fine woven white silk under-sash",
            "outer_wear": "Pitambara: Brilliant canary-yellow flowing silk dhoti draped with royal golden borders",
            "bottom_wear": "Flowing yellow silk scarf (uttariya) trailing over broad shoulders",
            "footwear": "Bare divine feet adorned with chiming silver and bell-anklets (Nupur)",
            "relics_and_gear": [
                "Venu / Bansuri (Bamboo flute): Held gently to lips with both hands in Tribhanga stance",
                "Vaijayanti Mala: Long garland of fragrant woodland flowers cascading past knees",
                "Sudarshana Chakra: Spinning celestial solar discus hovering ready on right index finger"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Graceful Divine Biped",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Delicate transverse flute playing posture with relaxed curved finger phalanges"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 115°]",
                "knee_flexion_limit": "140°",
                "ankle_mobility": "Classic Tribhanga stance: right foot gracefully crossed over left toe"
            },
            "spring_bone_colliders": ["peacock_feather", "flute_tassels", "silk_uttariya", "vaijayanti_garland"],
            "blendshapes": ["divine_smile", "playful_wink", "flute_playing", "bhagavad_wisdom", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Strategic Architecture Governance & Multi-Agent Consensus Orchestration",
            "motto": "Karmanye Vadhikaraste: Focus completely on code excellence without anxiety over vanity metrics.",
            "state_reactions": {
                "Idle": "Poised in three-bend Tribhanga stance, bamboo flute near lips with soft peaceful melody",
                "Working": "Effortless typing with rhythmic musical cadence, eyes twinkling with playful brilliance",
                "Thinking": "Lightly taps flute against cheek, eyes gazing upward into cosmic strategic patterns",
                "Success": "Delightful warm laughter, twirling flute gracefully before offering blessing",
                "Error": "Reassuring smile: 'Every bug is merely a lesson in disguise. Let us re-balance the state.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 6. ANIME: Frieren
    # -------------------------------------------------------------
    {
        "id": "char.frieren",
        "slug": "frieren",
        "name": "Frieren",
        "native_name": "フリーレン",
        "origin_lore": "Frieren: Beyond Journey's End (Sousou no Frieren)",
        "creator": "Kanehito Yamada & Tsukasa Abe",
        "archetype": "Millennium Elven Mage & Grimoire Collector",
        "category": "anime_character",
        "canonical_stats": {
            "age": 1000,
            "birthday": "Unknown (Ancient Elven Era)",
            "height_cm": 150,
            "weight_kg": 42,
            "zodiac": "Aquarius",
            "blood_type": "Elven"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf4eb",
            "skin_description": "Ageless pale porcelain elven skin",
            "eyes": {
                "color_base": "#0ea5e9",
                "color_highlight": "#7dd3fc",
                "pupil_style": "Large serene turquoise-green eyes with sleepy calm expressions",
                "expression_default": "Quiet, impassive and gently curious"
            },
            "hair": {
                "base_color": "#ffffff",
                "tip_gradient": "#e0e7ff",
                "style": "Long snowy silver-white twin pigtails tied low behind long pointed elven ears",
                "length": "Past waist level",
                "physics_stiffness": 0.35
            },
            "distinguishing_marks": [
                "Long pointed slender elven ears adorned with dangling red jewel teardrop earrings",
                "Subdued suppressed mana aura (completely imperceptible to casual detection)"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Striped high-neck black-and-grey knit dress shirt",
            "outer_wear": "Cream-white wizard capelet with gold borders fastened with black leather straps",
            "bottom_wear": "Matching cream pleated skirt with dark stockings",
            "footwear": "Brown leather travel boots with silver buckles",
            "relics_and_gear": [
                "Ancient tall wooden magical staff crowned with crescent loop and red gem",
                "Leather luggage bag packed with rare eccentric spell grimoires"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Petite Elven",
            "bone_count": 162,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Relaxed staff grip with pointed index wand posture"
            },
            "leg_articulation": {
                "hip_swing_range": "[-35°, 90°]",
                "knee_flexion_limit": "135°",
                "ankle_mobility": "Nimble walking gait"
            },
            "spring_bone_colliders": ["twin_tails", "elven_ears", "earrings", "capelet"],
            "blendshapes": ["sleepy", "smug", "curious", "magic_cast", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Victoria_Rubin.vrm",
            "thumbnail_url": "./public/thumbnails/Victoria_Rubin.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Long-Term Dependency Auditing & Archival Codebase Preservation",
            "motto": "A decade spent refactoring a legacy module is merely a blink of an eye.",
            "state_reactions": {
                "Idle": "Drowsy blinking, leaning lightly against staff while hair breezes softly",
                "Working": "Calm leisurely reading through thousands of lines of AST without breaking a sweat",
                "Thinking": "Fingers tilt cheek, examining an ancient regex: 'Fascinating. An obsolete spell.'",
                "Success": "Quiet slight proud nod, producing a legendary spellbook from her satchel",
                "Error": "Unbothered yawn: 'It will only take 80 years to re-write this parser properly.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 7. DEITY: Goddess Lakshmi (Padma)
    # -------------------------------------------------------------
    {
        "id": "char.goddess_lakshmi",
        "slug": "lakshmi",
        "name": "Goddess Lakshmi (Padma)",
        "native_name": "लक्ष्मी (पद्मा)",
        "origin_lore": "Sri Sukta, Vishnu Purana, Samudra Manthan",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "Goddess of Fortune, Radiant Prosperity & Spiritual Abundance",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Timeless Divine Radiance",
            "birthday": "Diwali (Kartika Amavasya)",
            "height_cm": 172,
            "weight_kg": 58,
            "zodiac": "Universal Benefactor",
            "blood_type": "Golden Divine Essence"
        },
        "micro_physical_details": {
            "skin_tone": "#fde68a",
            "skin_description": "Luminous molten-gold complexion radiating celestial warmth and maternal grace",
            "eyes": {
                "color_base": "#78350f",
                "color_highlight": "#f59e0b",
                "pupil_style": "Large almond-shaped warm honey-amber eyes beaming with boundless generosity",
                "expression_default": "Supreme benevolence, maternal tenderness, and auspicious serenity"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Long glossy black tresses cascading down her back, bound in gold hairpins and fragrant jasmine garlands",
                "length": "Past waist level",
                "physics_stiffness": 0.45
            },
            "distinguishing_marks": [
                "Gold coins showered continuously from lower right palm in Varada Mudra",
                "Auspicious vermilion Kumkuma and Chandan Tilaka centered on brow",
                "Seated atop an expansive blossoming pink sacred lotus flower (Padmasana)"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Golden silk blouse embellished with delicate pearls",
            "outer_wear": "Crimson red and gold Zari bridal sari draped with exquisite pleats",
            "bottom_wear": "Cascading silk folds pooling gently over lotus petals",
            "footwear": "Bare feet dyed with red Alta henna and chiming golden anklets",
            "relics_and_gear": [
                "Padma (Twin pink lotus blooms held gracefully in upper hands)",
                "Golden urn of boundless prosperity (Amrita Kalasha)",
                "Elaborate multi-tiered golden necklaces and gemstone armlets"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Royal Celestial Goddess",
            "bone_count": 137,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Varada Mudra (boon-giving) right hand + Lotus-stem grasp in upper hands"
            },
            "leg_articulation": {
                "hip_swing_range": "[-35°, 100°]",
                "knee_flexion_limit": "145° (perfect Lotus Padmasana)",
                "ankle_mobility": "Graceful poised feet"
            },
            "spring_bone_colliders": ["sari_pallu", "long_tresses", "gold_necklaces", "lotus_petals"],
            "blendshapes": ["auspicious_smile", "maternal_grace", "boon_giving", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Victoria_Rubin.vrm",
            "thumbnail_url": "./public/thumbnails/Victoria_Rubin.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Cloud Cost Optimization & Production Resource Abundance",
            "motto": "May your builds be swift, your clusters cost-efficient, and your releases prosperous.",
            "state_reactions": {
                "Idle": "Regally poised on glowing lotus, warm smile radiating calm assurance",
                "Working": "Graceful gestures streamlining compute resources and cutting cloud waste",
                "Thinking": "Lotus blooms glow softly as she calculates optimal serverless economics",
                "Success": "Gold coins cascade with celestial chime: 'Build succeeded with 0 AWS cost overruns.'",
                "Error": "Gentle compassionate gaze: 'Do not fret over rate limits. Abundant quota shall return.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 8. MYTHOLOGICAL: Sun Wukong (The Monkey King)
    # -------------------------------------------------------------
    {
        "id": "char.sun_wukong",
        "slug": "sun-wukong",
        "name": "Sun Wukong (The Great Sage)",
        "native_name": "孫悟空 (齊天大聖)",
        "origin_lore": "Journey to the West (Wu Cheng'en)",
        "creator": "Classical Chinese Literature / Ming Dynasty",
        "archetype": "The Immortal Monkey King & Defier of the Heavens",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Immortal (Surpassed 72 Transformations)",
            "birthday": "Born from Stone at the Mountain of Flowers and Fruit",
            "height_cm": 168,
            "weight_kg": 64,
            "zodiac": "Year of the Monkey",
            "blood_type": "Stone-Born Immortal"
        },
        "micro_physical_details": {
            "skin_tone": "#f59e0b",
            "skin_description": "Golden-furred nimble simian visage with fierce human-proportioned athletic build",
            "eyes": {
                "color_base": "#dc2626",
                "color_highlight": "#fbbf24",
                "pupil_style": "Fiery Golden Eyes (Huoyan Jinjing) capable of seeing through all illusions and evil",
                "expression_default": "Mischievous grin with ferocious battle-ready sharpness"
            },
            "hair": {
                "base_color": "#d97706",
                "tip_gradient": "#fef08a",
                "style": "Lush golden-brown wild maned fur crowned with the golden tightening circlet and phoenix feathers",
                "length": "Medium wild mane",
                "physics_stiffness": 0.60
            },
            "distinguishing_marks": [
                "Golden Fillet / Circlet (Curse of the Tightening Crown) fastened securely on brow",
                "Prehensile monkey tail twitching with agile kinetic anticipation",
                "Twin long red pheasant/phoenix feathers rising proudly from headpiece"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Flexible crimson combat under-tunic",
            "outer_wear": "Golden Chain Mail armor shirt with tiger-skin kilt and silk waist sash",
            "bottom_wear": "Baggy martial arts pants cinched tightly with red cloth bands",
            "footwear": "Cloud-walking boots (cloud-treading lotus footwear)",
            "relics_and_gear": [
                "Ruyi Jingu Bang (As-You-Will Golden-Banded Staff): Can shrink to needle or expand into pillar",
                "Somersault Cloud (Jindou Yun): Travels 108,000 li in a single leap"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Agile Simian Biped",
            "bone_count": 65,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Acrobatic staff spinning palm-roll and single-handed balance grip"
            },
            "leg_articulation": {
                "hip_swing_range": "[-50°, 135°]",
                "knee_flexion_limit": "155° (allows full deep simian crouch and backflip landings)",
                "ankle_mobility": "Hyper-flexible spring-loaded ankle joints"
            },
            "spring_bone_colliders": ["phoenix_feathers", "tail", "waist_sash", "staff_rings"],
            "blendshapes": ["mischief_grin", "battle_roar", "fiery_eyes_flash", "taunt", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Avatar_Orion.vrm",
            "thumbnail_url": "./public/thumbnails/Avatar_Orion.svg"
        },
        "miyora_companion_role": {
            "work_specialty": "Adversarial Chaos Engineering & Stress-Testing Penetration Audits",
            "motto": "Equal to Heaven! If your system has a crack, my staff will find it in one blow.",
            "state_reactions": {
                "Idle": "Perched lightly atop staff resting on one foot, chewing a peach with cheeky smirk",
                "Working": "Blur of wild kinetic attacks hitting endpoints at 10,000 req/sec concurrently",
                "Thinking": "Scratches ear with staff end, fiery golden eyes scanning bytecode decompilation",
                "Success": "Flips staff 360 degrees behind back with triumphant laugh: 'None can withstand the Sage!'",
                "Error": "Bares fangs playfully: 'Found your memory leak! Don't worry, I broke it for your own good.'"
            }
        }
    }
]

# Write out authoritative detailed database
out_path = 'docs/detailed_character_specifications.json'
with open(out_path, 'w', encoding='utf-8') as f:
    json.dump(characters, f, indent=2, ensure_ascii=False)

print(f"Successfully generated {out_path} with {len(characters)} fully-researched, deep-detail characters!")
