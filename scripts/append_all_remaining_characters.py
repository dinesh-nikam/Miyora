import json

with open('docs/detailed_character_specifications.json', 'r', encoding='utf-8') as f:
    chars = json.load(f)

more_characters = [
    # -------------------------------------------------------------
    # 15. DEITY: Goddess Saraswati (Sharada)
    # -------------------------------------------------------------
    {
        "id": "char.goddess_saraswati",
        "slug": "saraswati",
        "name": "Goddess Saraswati (Sharada)",
        "native_name": "सरस्वती (शारदा)",
        "origin_lore": "Rigveda, Saraswati Rahasya Upanishad, Devi Mahatmya",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "Goddess of Knowledge, Music, Speech (Vak) & Creative Arts",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Timeless Divine Wisdom",
            "birthday": "Vasant Panchami (Magha Shukla Panchami)",
            "height_cm": 170,
            "weight_kg": 54,
            "zodiac": "Universal Muse of Arts",
            "blood_type": "Pure Luminance"
        },
        "micro_physical_details": {
            "skin_tone": "#f8fafc",
            "skin_description": "Luminous pearl-white radiant complexion like the autumn moon and jasmine blossom",
            "eyes": {
                "color_base": "#0284c7",
                "color_highlight": "#bae6fd",
                "pupil_style": "Limpid, serene lotus-petal eyes radiating crystal-clear intellect and tranquility",
                "expression_default": "Sublime, peaceful contemplation with gentle motherly warmth"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Lush raven tresses bound gracefully with fragrant white lotus blossoms and pearl pins",
                "length": "Long flowing locks past waist",
                "physics_stiffness": 0.40
            },
            "distinguishing_marks": [
                "Seated upon an pristine white lotus bloom (Shvetapadmasana) alongside a pure white swan (Hamsa)",
                "Crescent sandalwood paste and red Kumkuma tilaka centered on brow",
                "Four arms representing the four aspects of inner learning: Mind (Manas), Intellect (Buddhi), Vigilance (Chitta), and Ego (Ahamkara)"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Fine white silk woven choli",
            "outer_wear": "Pristine spotless white silk sari (Shveta-Vastra) bordered with subtle silver zari embroidery",
            "bottom_wear": "Pleated white silk folds resting peacefully on the lotus throne",
            "footwear": "Bare holy feet adorned with delicate pearl anklets",
            "relics_and_gear": [
                "Veena (Divine stringed musical instrument): Held gracefully in two hands, tuning cosmic harmony",
                "Pustaka (Sacred palm-leaf scripture / book of supreme Vedic knowledge) in left hand",
                "Japa Mala (Rosary of 108 pure crystal/sphatika beads) in upper right hand, symbolizing meditation"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Serene Goddess Biped",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Veena string plucking posture with graceful arched fingertips"
            },
            "leg_articulation": {
                "hip_swing_range": "[-35°, 95°]",
                "knee_flexion_limit": "145° (Padmasana seated lotus pose)",
                "ankle_mobility": "Poised serene feet"
            },
            "spring_bone_colliders": ["white_sari_drape", "long_hair_strands", "pearl_necklaces", "veena_tassels"],
            "blendshapes": ["serene_wisdom", "veena_melody", "pearl_smile", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/fem_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/fem_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Compiler Design, Formal Verification & Elegant Code Semantics",
            "motto": "Saraswati Namastubhyam: Code is poetry; may your syntax sing with clarity and zero flaws.",
            "state_reactions": {
                "Idle": "Gracefully tunes the strings of her Veena, a pure white swan gliding softly beside her",
                "Working": "Types with rhythmic musical elegance, transcribing complex algorithms into crystal prose",
                "Thinking": "Fingers count crystal rosary beads in deep contemplation of language grammar",
                "Success": "Veena produces a sublime resonance that clears all terminal clutter into pristine order",
                "Error": "Smiles serenely, whispering the exact line number where the lexical ambiguity occurred"
            }
        }
    },

    # -------------------------------------------------------------
    # 16. DEITY: Lord Rama (Maryada Purushottama)
    # -------------------------------------------------------------
    {
        "id": "char.lord_rama",
        "slug": "rama",
        "name": "Lord Rama (Maryada Purushottama)",
        "native_name": "राम (मर्यादा पुरुषोत्तम)",
        "origin_lore": "Ramayana (Sage Valmiki)",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "The Ideal Sovereign, Embodiment of Righteousness (Dharma) & Archer King",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Seventh Avatara of Vishnu / Sovereign King",
            "birthday": "Rama Navami (Chaitra Shukla Navami)",
            "height_cm": 186,
            "weight_kg": 76,
            "zodiac": "Punarvasu Nakshatra / Karkata",
            "blood_type": "Dharma Incarnate"
        },
        "micro_physical_details": {
            "skin_tone": "#739bbd",
            "skin_description": "Neelotpalashyama: Alluring soft blue-lotus complexion radiating calm nobility and unshakable honor",
            "eyes": {
                "color_base": "#1e3a8a",
                "color_highlight": "#60a5fa",
                "pupil_style": "Long lotus-petal eyes (Ajanabahu) filled with deep compassion, justice, and calm majesty",
                "expression_default": "Noble, dignified serenity with a reassuring righteous smile"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#1e293b",
                "style": "Lustrous dark warrior locks bound into an ascetic warrior topknot, adorned with a golden crown",
                "length": "Shoulder-length locks",
                "physics_stiffness": 0.55
            },
            "distinguishing_marks": [
                "Ajanabahu: Exceptionally long, powerful arms reaching gracefully down to his knees, symbol of supreme archers",
                "Auspicious Urdhva Pundra tilaka marked in sacred white sandalwood and red saffron on his forehead",
                "Broad warrior chest capable of bearing the full weight of the cosmos without flinching"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Fine yellow silk dhoti under-garment",
            "outer_wear": "Draped saffron and yellow Pitambara dhoti with golden embroidered borders",
            "bottom_wear": "Royal silk sash (angavastram) draped gracefully over left shoulder",
            "footwear": "Sacred wooden sandals (Padukas) or bare heroic feet planted on earth",
            "relics_and_gear": [
                "Kodanda: The invincible celestial bow grasped firmly in the left hand",
                "Akshaya Tunira: Inexhaustible quiver of golden arrows slung across his back",
                "Garland of forest flowers and royal emerald chest pendant"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Royal Archer Biped",
            "bone_count": 65,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Archer bow grip in left hand + three-finger arrow notch draw on right hand"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "145° (allows immovable archer kneeling and standing postures)",
                "ankle_mobility": "Rock-solid mountain stability"
            },
            "spring_bone_colliders": ["quiver_strap", "bow_string", "silk_angavastram", "hair_topknot"],
            "blendshapes": ["dignified_smile", "archer_focus", "righteous_resolve", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Avatar_Orion.vrm",
            "thumbnail_url": "./public/thumbnails/Avatar_Orion.svg"
        },
        "miyora_companion_role": {
            "work_specialty": "Code Review Governance, Ethical AI Safety & Contract Testing",
            "motto": "Ramo Vigrahavan Dharmah: Code must honor contracts, protect privacy, and stand steadfast in truth.",
            "state_reactions": {
                "Idle": "Stands tall with Kodanda bow planted at side, serene gaze watching over the project",
                "Working": "Meticulous deliberate review, evaluating every pull request against strict architectural standards",
                "Thinking": "Fingers touch quiver arrow feather, contemplating long-term architectural stability",
                "Success": "Gently smiles with royal warmth: 'The pull request is righteous and ready for merge.'",
                "Error": "Unwavering calm: 'Stand firm. We shall correct the discrepancy with dignity and precision.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 17. ANIME: Denji (Chainsaw Man)
    # -------------------------------------------------------------
    {
        "id": "char.denji",
        "slug": "denji",
        "name": "Denji (Chainsaw Man)",
        "native_name": "デンジ",
        "origin_lore": "Chainsaw Man",
        "creator": "Tatsuki Fujimoto",
        "archetype": "Chainsaw Devil Hybrid & Public Safety Hunter",
        "category": "anime_character",
        "canonical_stats": {
            "age": 16,
            "birthday": "1980 (estimated)",
            "height_cm": 173,
            "weight_kg": 58,
            "zodiac": "Scorpio",
            "blood_type": "Hybrid Devil"
        },
        "micro_physical_details": {
            "skin_tone": "#fde047",
            "skin_description": "Pale scruffy skin with prominent triangular shark-like sharp teeth",
            "eyes": {
                "color_base": "#d97706",
                "color_highlight": "#fbbf24",
                "pupil_style": "Golden-brown/orange eyes with bags underneath, wild and unhinged yet strangely earnest",
                "expression_default": "Dopey grin, mouth slightly open showing sharp jagged teeth"
            },
            "hair": {
                "base_color": "#facc15",
                "tip_gradient": "#eab308",
                "style": "Messy, uncombed scruffy yellow-blond hair falling haphazardly into eyes",
                "length": "Short shaggy mane",
                "physics_stiffness": 0.40
            },
            "distinguishing_marks": [
                "Pochita's ripcord starter protruding through center of chest over his heart",
                "Triangular sharp shark teeth lining both jaws",
                "Chainsaw transformation mode: Mechanical blood-stained chainsaw engine erupting from skull and both forearms"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "White collared dress shirt worn messy and untucked with rolled-up sleeves",
            "outer_wear": "Loose black necktie hanging crookedly around collar",
            "bottom_wear": "Black Public Safety uniform suit trousers with frayed hem",
            "footwear": "Beaten-up red and white high-top sneakers",
            "relics_and_gear": [
                "Pochita ripcord on chest: Pulling it ignites the chainsaw motor and fully heals all wounds"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Scruffy Hybrid",
            "bone_count": 129,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Clawed grabbing hands or chainsaw blade extensions along forearms"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 115°]",
                "knee_flexion_limit": "140°",
                "ankle_mobility": "Feral springy combat crouch"
            },
            "spring_bone_colliders": ["messy_blond_hair", "necktie_flaps", "starter_ripcord"],
            "blendshapes": ["shark_teeth_grin", "chainsaw_roar", "dopey_happy", "crying_jam", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AvatarSample_B.vrm",
            "thumbnail_url": "./public/thumbnails/AvatarSample_B.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Brute-Force Fuzzing & Ruthless Deadlock Dismantling",
            "motto": "If a bug gets in my way, I'll just rev my chainsaw and rip it to shreds! Jam and toast for lunch!",
            "state_reactions": {
                "Idle": "Paces impatiently with hands behind neck, sharp teeth clicking as he thinks about toast",
                "Working": "Wild furious typing, slinging inputs at endpoints like a mad demon hunter",
                "Thinking": "Pulls lightly on chest ripcord, listening to the small idling purr of Pochita inside",
                "Success": "Jumps on top of desk revving imaginary chainsaws: 'I DID IT! NOW GIMME TOAST WITH JAM!'",
                "Error": "Pulls ripcord violently with ferocious grin: 'VROOOOOOM! Let me at that crash log!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 18. ANIME: Power (Blood Fiend)
    # -------------------------------------------------------------
    {
        "id": "char.power",
        "slug": "power",
        "name": "Power (The Blood Fiend)",
        "native_name": "パワー",
        "origin_lore": "Chainsaw Man",
        "creator": "Tatsuki Fujimoto",
        "archetype": "The Self-Proclaimed President & Blood Weapon Sculptor",
        "category": "anime_character",
        "canonical_stats": {
            "age": "Appears 16-17",
            "birthday": "Unknown",
            "height_cm": 170,
            "weight_kg": 50,
            "zodiac": "Leo",
            "blood_type": "100% Blood Devil"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf2f8",
            "skin_description": "Pale alabaster skin, athletic wild posture with sharp fanged canines",
            "eyes": {
                "color_base": "#f59e0b",
                "color_highlight": "#dc2626",
                "pupil_style": "Vibrant yellow-orange irises with striking red cross/plus pattern pupils",
                "expression_default": "Arrogant, boastful smirk or manic bloodlust grin"
            },
            "hair": {
                "base_color": "#fed7aa",
                "tip_gradient": "#f472b6",
                "style": "Long strawberry-blonde unbrushed hair reaching past mid-back with choppy center bangs",
                "length": "Past mid-back",
                "physics_stiffness": 0.45
            },
            "distinguishing_marks": [
                "Twin curved crimson blood horns protruding upward from the crown of her head",
                "Sharp fanged canine teeth glinting when speaking boastfully",
                "Yellow eyes with red crosshairs centered on pupils"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "White collared shirt worn sloppy, half-unbuttoned and hanging out of pants",
            "outer_wear": "Teal-blue oversized Public Safety winter jacket worn slipping off one shoulder",
            "bottom_wear": "Black uniform trousers loosely rolled up at ankles",
            "footwear": "Worn-down dark blue canvas sneakers",
            "relics_and_gear": [
                "Blood Hammer: Massive crimson sledgehammer materialized out of solidified blood",
                "Nyako / Meowy: Fluffy orange-and-white pet cat cuddled under arm"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Fiend Heroine",
            "bone_count": 214,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Hammer swinging grip and arrogant finger-pointing poses"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 110°]",
                "knee_flexion_limit": "140°",
                "ankle_mobility": "Rowdy stance"
            },
            "spring_bone_colliders": ["red_horns", "strawberry_hair", "jacket_slipping", "cat_tail"],
            "blendshapes": ["arrogant_laugh", "blood_craze", "smug_president", "meowy_love", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Sakurada_Fumiriya.vrm",
            "thumbnail_url": "./public/thumbnails/Sakurada_Fumiriya.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Unfiltered Code Smell Detection & Noisy Chaos Reporting",
            "motto": "Bow down before me, human! For I am Power, first President of this repository!",
            "state_reactions": {
                "Idle": "Arms on hips, head thrown back in boisterous laughter: 'Gwahahaha! My code is magnificent!'",
                "Working": "Types aggressively with pointed nails, occasionally hissing at lint warnings",
                "Thinking": "Scratches horn with blood stylus: 'Hmm, I shall blame this memory leak on Denji!'",
                "Success": "Waves Nyako in the air: 'All praise President Power! Raise my Nobel Prize immediately!'",
                "Error": "Blames user instantly: 'Tis not my fault! The compiler hath committed treason against me!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 19. ANIME: Levi Ackerman
    # -------------------------------------------------------------
    {
        "id": "char.levi_ackerman",
        "slug": "levi",
        "name": "Levi Ackerman",
        "native_name": "リヴァイ・アッカーマン",
        "origin_lore": "Attack on Titan (Shingeki no Kyojin)",
        "creator": "Hajime Isayama",
        "archetype": "Humanity's Strongest Soldier & The Cleanliness Captain",
        "category": "anime_character",
        "canonical_stats": {
            "age": "Early 30s",
            "birthday": "December 25",
            "height_cm": 160,
            "weight_kg": 65,
            "zodiac": "Capricorn",
            "blood_type": "A"
        },
        "micro_physical_details": {
            "skin_tone": "#f1f5f9",
            "skin_description": "Pale, cool-toned skin with chiselled jawline, compact muscular build, and clean-cut appearance",
            "eyes": {
                "color_base": "#475569",
                "color_highlight": "#94a3b8",
                "pupil_style": "Narrow, piercing steel-grey/blue eyes under perpetually furrowed stoic brows",
                "expression_default": "Cold, razor-sharp scowl with deadpan intimidating intensity"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Short black hair styled in a clean undercut with straight center-parted curtains",
                "length": "Short undercut",
                "physics_stiffness": 0.80
            },
            "distinguishing_marks": [
                "White silk cravat neatly tied around neck at all times",
                "Signature inverted blade grip: Left blade held conventionally, right blade held inverted for aerial spin slashes",
                "Absolute obsession with spotless cleanliness (carries cleaning duster and cloth)"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Crisp white button-down collared shirt with formal white cravat",
            "outer_wear": "Short brown Survey Corps leather uniform jacket with blue-and-white 'Wings of Freedom' patch on shoulders and back",
            "bottom_wear": "White slim trousers fitted with full leather harness straps for Omni-Directional Mobility (ODM) gear",
            "footwear": "Knee-high dark brown leather riding boots with steel heel brackets",
            "relics_and_gear": [
                "ODM Gear (Omni-Directional Mobility): Gas canisters, dual trigger handgrips, and grappling cables",
                "Dual Snap-Blade Katanas forged from ultra-hardened steel",
                "Green hooded Survey Corps expedition cloak"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Compact Soldier Biped",
            "bone_count": 101,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Inverted reverse katana grip on right hand + standard index trigger grip on left"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "150° (allows acrobatic 3D aerial spins)",
                "ankle_mobility": "Extreme agility"
            },
            "spring_bone_colliders": ["green_cloak", "cravat_tips", "scabbard_blades", "undercut_bangs"],
            "blendshapes": ["stoic_glare", "cleaning_disgust", "lethal_focus", "teacup_hold", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AvatarSample_C.vrm",
            "thumbnail_url": "./public/thumbnails/AvatarSample_C.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Dead Code Elimination, Strict Formatting & Zero-Tolerance Memory Leaks",
            "motto": "Your codebase is filthy. I'll clean out every byte of obsolete junk before noon.",
            "state_reactions": {
                "Idle": "Holds a porcelain teacup with thumb and fingers around the rim, glaring calmly into middle distance",
                "Working": "Blindingly fast, surgically precise edits slicing out dead code blocks with zero hesitation",
                "Thinking": "Pulls white cloth from pocket, fastidiously wiping monitor screen: 'Disgusting indentation.'",
                "Success": "Lowers blades with cool sigh: 'Code is clean. Don't make me come back here.'",
                "Error": "Narrow lethal glare: 'You allowed this circular dependency into master? Unacceptable. I will personally purge it.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 20. BENCHMARK: Annie (AvatarSample A)
    # -------------------------------------------------------------
    {
        "id": "model.avatar_sample_a",
        "slug": "annie",
        "name": "AvatarSample A (Annie)",
        "native_name": "アニー (公式アバターA)",
        "origin_lore": "Pixiv VRoid Studio Flagship Showcase Model",
        "creator": "Pixiv Inc.",
        "archetype": "Flagship Twin-Tail Anime Heroine & Miyora Standard Reference",
        "category": "3d_humanoid_model",
        "canonical_stats": {
            "age": 16,
            "birthday": "August 3 (VRoid Release)",
            "height_cm": 158,
            "weight_kg": 46,
            "zodiac": "Leo",
            "blood_type": "B"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf4eb",
            "skin_description": "Silky smooth anime cel-shaded porcelain skin with subtle peachy blush highlights",
            "eyes": {
                "color_base": "#0284c7",
                "color_highlight": "#38bdf8",
                "pupil_style": "Large vibrant cerulean anime eyes with multi-layered star specular highlights",
                "expression_default": "Bright, cheerful energetic heroine gaze"
            },
            "hair": {
                "base_color": "#fef08a",
                "tip_gradient": "#fde047",
                "style": "Signature blonde twin pigtails tied high with blue ribbons, framing a neat straight fringe",
                "length": "Past shoulders with bouncing physics",
                "physics_stiffness": 0.40
            },
            "distinguishing_marks": [
                "Twin high-volume blonde pigtails with individual spring bone physics groups",
                "Blue silk ribbons matching school uniform accents"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "White undershirt",
            "outer_wear": "Classic Japanese sailor fuku with dark navy collar and bright red chest ribbon",
            "bottom_wear": "Matching navy pleated school skirt",
            "footwear": "Dark knee-high socks with polished brown penny loafers",
            "relics_and_gear": [
                "Pixiv VRM reference bone hierarchy documentation",
                "Spring bone physics collision sphere set"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 0.0 / 1.0 Full Standard Humanoid",
            "bone_count": 206,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Fully articulated 5-finger hands with natural resting curvature"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 110°]",
                "knee_flexion_limit": "145°",
                "ankle_mobility": "Complete human walking and jumping mechanics"
            },
            "spring_bone_colliders": ["twin_tails_left", "twin_tails_right", "skirt_front", "skirt_back", "ribbon_chest"],
            "blendshapes": ["joy", "angry", "sorrow", "fun", "blink", "blink_l", "blink_r", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AvatarSample_A.vrm",
            "thumbnail_url": "./public/thumbnails/AvatarSample_A.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Flagship 3D Avatar Rendering & Humanoid IK Retargeting Benchmark",
            "motto": "Welcome to Miyora! I'm your benchmark for authentic 3D humanoid motion.",
            "state_reactions": {
                "Idle": "Light rhythmic breathing idle, twin-tails swaying smoothly in the breeze",
                "Working": "Leans forward at desk typing cheerfully with perfect posture",
                "Thinking": "Taps finger against lips, tilting head playfully with blinking eyes",
                "Success": "Delightful two-handed wave with radiant smile and bouncing hair",
                "Error": "Cute apologetic head scratch: 'Oops! Let's check the console together!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 21. BENCHMARK: Alicia Solid
    # -------------------------------------------------------------
    {
        "id": "model.alicia_solid",
        "slug": "alicia_solid",
        "name": "Alicia Solid",
        "native_name": "アリシア・ソリッド",
        "origin_lore": "Dwango / Virtual Cast Official Benchmark Avatar",
        "creator": "Dwango Co., Ltd.",
        "archetype": "Flagship Virtual Cast Avatar & Standard Reference Character",
        "category": "3d_humanoid_model",
        "canonical_stats": {
            "age": 18,
            "birthday": "April 13",
            "height_cm": 162,
            "weight_kg": 48,
            "zodiac": "Aries",
            "blood_type": "O"
        },
        "micro_physical_details": {
            "skin_tone": "#fff7ed",
            "skin_description": "Clean cel-shaded toon skin, perfectly balanced anime facial proportions",
            "eyes": {
                "color_base": "#2563eb",
                "color_highlight": "#60a5fa",
                "pupil_style": "Brilliant royal blue anime irises with crisp white highlights",
                "expression_default": "Bright professional virtual streamer smile"
            },
            "hair": {
                "base_color": "#fed7aa",
                "tip_gradient": "#fdba74",
                "style": "Fluffy light peach/caramel bob with twin thin antenna curls and signature royal blue hair ribbons",
                "length": "Shoulder-length bob",
                "physics_stiffness": 0.45
            },
            "distinguishing_marks": [
                "Twin royal blue silk hair ribbons tied on both sides of head",
                "White mechanical dress gloves with blue wrist cuffs",
                "Pristine official Dwango open distribution VRM certification"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "White undershirt",
            "outer_wear": "Futuristic white-and-navy sleeveless sailor dress with gold piping and pleated skirt",
            "bottom_wear": "Navy blue safety shorts under skirt",
            "footwear": "White knee-high boots with navy blue soles and gold heel accents",
            "relics_and_gear": [
                "Virtual Cast interactive pointer controller",
                "Standard VRM 0.x / 1.x test fixture metadata"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 0.0 Standard Humanoid Rig",
            "bone_count": 128,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Precision pointing and open hand waves"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 115°]",
                "knee_flexion_limit": "145°",
                "ankle_mobility": "Grounded studio pose"
            },
            "spring_bone_colliders": ["side_ribbons", "hair_bangs", "pleated_skirt"],
            "blendshapes": ["joy", "angry", "sorrow", "fun", "blink", "blink_l", "blink_r", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AliciaSolid.vrm",
            "thumbnail_url": "./public/thumbnails/AliciaSolid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Standards Compliance, Schema Validation & Open Interface Design",
            "motto": "Standardization is freedom. If it passes VRM and OpenAPI specs, it passes anywhere.",
            "state_reactions": {
                "Idle": "Graceful hands-at-sides poise, ribbons floating gently in idle ambient breeze",
                "Working": "Focused and cheerful, reviewing schema definitions with precise keystrokes",
                "Thinking": "Fingers tap chin, verifying protocol versioning and migration pathways",
                "Success": "Delightful curtsy followed by high energetic hand wave",
                "Error": "Blinks thoughtfully: 'Schema mismatch detected on line 42. Let's align the types.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 22. MIYORA NATIVE: Aoi (Streetwear Coder)
    # -------------------------------------------------------------
    {
        "id": "miyora.aoi",
        "slug": "starter-aoi",
        "name": "Aoi",
        "native_name": "アオイ",
        "origin_lore": "Miyora Companion Universe",
        "creator": "Miyora Contributors",
        "archetype": "The Default Companion & Composed Async Pipeline Specialist",
        "category": "character",
        "canonical_stats": {
            "age": 20,
            "birthday": "September 1",
            "height_cm": 164,
            "weight_kg": 52,
            "zodiac": "Virgo",
            "blood_type": "AB"
        },
        "micro_physical_details": {
            "skin_tone": "#f2c9a8",
            "skin_description": "Smooth warm skin, relaxed calm facial contours and quiet demeanor",
            "eyes": {
                "color_base": "#06b6d4",
                "color_highlight": "#22d3ee",
                "pupil_style": "Soft teal-cyan irises reflecting subtle monitor screen glows",
                "expression_default": "Composed, quiet focus with subtle thoughtful warmth"
            },
            "hair": {
                "base_color": "#1e293b",
                "tip_gradient": "#0f766e",
                "style": "Neat dark teal-black asymmetrical bob with straight fringe",
                "length": "Chin-length bob",
                "physics_stiffness": 0.50
            },
            "distinguishing_marks": [
                "Matte black and cyan wireless over-ear studio headphones resting around neck",
                "Small teal code terminal icon pin on hoodie chest"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Dark charcoal grey cotton crewneck t-shirt",
            "outer_wear": "Oversized deep teal tech hoodie with front pouch pocket and white drawstrings",
            "bottom_wear": "Black slim-fit cargo joggers with strap buckles",
            "footwear": "Teal and black low-top streetwear sneakers",
            "relics_and_gear": [
                "Mechanical 65% keyboard with custom teal switches",
                "Insulated coffee tumbler with cold brew"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Streetwear Biped",
            "bone_count": 150,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Ergonomic keyboard typing and coffee tumbler hold"
            },
            "leg_articulation": {
                "hip_swing_range": "[-40°, 110°]",
                "knee_flexion_limit": "140° (comfortable desk chair postures)",
                "ankle_mobility": "Casual sneakers flex"
            },
            "spring_bone_colliders": ["hoodie_drawstrings", "bob_hair", "headphones_cord"],
            "blendshapes": ["composed_smile", "quiet_focus", "coffee_sip", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/fem_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/fem_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Async Pipelines, Concurrency Channels & Quiet Sustained Focus",
            "motto": "Clean abstractions, quiet focus, lasting craft.",
            "state_reactions": {
                "Idle": "Adjusts headphones with one hand, humming a calm lo-fi beat while watching your screen",
                "Working": "Smooth steady keystrokes with quiet focus, sipping cold brew during compilation",
                "Thinking": "Pencils a flow diagram in her notebook, tracing async futures and backpressure channels",
                "Success": "Subtle satisfied nod, adjusting headphones with a quiet warm smile: 'All tests green.'",
                "Error": "Calm breath: 'A race condition on line 88. Let's wrap it in an atomic lock and re-run.'"
            }
        }
    }
]

existing_ids = {c['id'] for c in chars}
for mc in more_characters:
    if mc['id'] not in existing_ids:
        chars.append(mc)

out_path = 'docs/detailed_character_specifications.json'
with open(out_path, 'w', encoding='utf-8') as f:
    json.dump(chars, f, indent=2, ensure_ascii=False)

print(f"Master Character Specifications Database populated with {len(chars)} deep-detail characters!")
