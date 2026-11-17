#!/usr/bin/env python3
"""
Expanded Canonical Character Research Data for Miyora
Comprehensive micro-details for Anime and Deity characters:
Canonical stats, skin hex, eye limbal rings/pupils, hair gradients/physics,
wardrobe layering, relics/weapons, 3D rig hierarchy (15 finger bones), and Miyora states.
"""

EXPANDED_CHARACTERS = [
    {
        "id": "char.naruto_uzumaki",
        "slug": "naruto",
        "name": "Naruto Uzumaki",
        "native_name": "うずまき ナルト (Seventh Hokage)",
        "origin_lore": "Naruto / Naruto Shippuden",
        "creator": "Masashi Kishimoto",
        "archetype": "Child of Prophecy & Nine-Tails Jinchuriki",
        "category": "anime_character",
        "canonical_stats": {
            "age": 17,
            "birthday": "October 10",
            "height_cm": 166,
            "weight_kg": 50.9,
            "zodiac": "Libra",
            "blood_type": "B"
        },
        "micro_physical_details": {
            "skin_tone": "#f5d3b3",
            "skin_description": "Sun-tanned warm complexion with three distinct curved whisker marks on each cheek",
            "eyes": {
                "color_base": "#0284c7",
                "color_highlight": "#38bdf8",
                "pupil_style": "Bright blue irises; horizontal bar pupils in Sage Mode with orange eyeshadow pigmentation",
                "limbal_ring": "#0369a1",
                "expression_default": "Unyielding grin with fierce determination"
            },
            "hair": {
                "base_color": "#eab308",
                "tip_gradient": "#facc15",
                "style": "Spiky vibrant blonde hair radiating outward like sun rays",
                "length": "Short spiky locks",
                "bangs_description": "Spikes framing forehead underneath metal headband",
                "physics_stiffness": 0.75
            },
            "distinguishing_marks": [
                "Three whiskers on each cheek from Kurama's chakra resonance",
                "Eight Trigrams Sealing Style formula on stomach",
                "Leaf Village forehead protector with long black fabric ties"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Black mesh armor undershirt",
            "outer_wear": "Black-and-orange tracksuit jacket with red Uzumaki swirl crest on back and shoulders",
            "bottom_wear": "Matching orange ninja trousers with shuriken holster wrapped to right thigh",
            "footwear": "Standard open-toed black shinobi sandals",
            "relics_and_gear": [
                "Leaf Village engraved forehead protector (Hitai-ate)",
                "Rasengan sphere of swirling high-density chakra",
                "Giant Toad summoning scroll"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 62,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "articulation_type": "Full 5-finger independent phalange articulation",
                "grip_style": "Shadow Clone cross hand seal (cross fingers) and Rasengan rotating palm"
            },
            "leg_articulation": {
                "hip_swing_range": "[-50°, 130°]",
                "knee_flexion_limit": "150° (shinobi wall-running and leaping)",
                "ankle_mobility": "Extreme tree-branch landing adaptation"
            },
            "spring_bone_colliders": ["headband_ties", "hair_spikes", "jacket_collar"],
            "blendshapes": ["joy", "determined", "sage_mode_eyes", "kyuubi_fangs", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Multi-Shadow Clone Concurrency, Relentless Retry Logic & Team Spirit",
            "motto": "I never go back on my word! That's my nindo, my ninja way of coding!",
            "state_reactions": {
                "Idle": "Adjusts his headband ties with a toothy grin: 'Dattebayo! What mission are we on today?'",
                "Working": "Forms the cross seal: 'Kage Bunshin no Jutsu! Spawning 100 parallel worker threads!'",
                "Thinking": "Folds arms in Sage Mode, absorbing environmental CPU telemetry in total balance",
                "Success": "Delivers a victorious thumbs-up: 'Believe it! Every single test passed on the first try!'",
                "Error": "Eyes glow crimson with Kurama chakra: 'A timeout?! I'll never give up until this service responds!'"
            }
        }
    },
    {
        "id": "char.sasuke_uchiha",
        "slug": "sasuke",
        "name": "Sasuke Uchiha",
        "native_name": "うちは サスケ (Shadow Hokage)",
        "origin_lore": "Naruto / Naruto Shippuden",
        "creator": "Masashi Kishimoto",
        "archetype": "Avenger of the Uchiha & Master of Lightning (Chidori)",
        "category": "anime_character",
        "canonical_stats": {
            "age": 17,
            "birthday": "July 23",
            "height_cm": 168,
            "weight_kg": 52.2,
            "zodiac": "Leo",
            "blood_type": "AB"
        },
        "micro_physical_details": {
            "skin_tone": "#fceee8",
            "skin_description": "Pale refined Uchiha clan complexion with sharp angular jawline",
            "eyes": {
                "color_base": "#18181b",
                "color_highlight": "#3f3f46",
                "pupil_style": "Right: Eternal Mangekyo Sharingan (straight tomoe); Left: Six-Tomoe Rinnegan (purple ripples)",
                "limbal_ring": "#7f1d1d",
                "expression_default": "Cold, penetrating, aloof, analytical"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Jet black spiky hair swept backward in ducktail style with long bangs framing face",
                "length": "Medium layered cut",
                "bangs_description": "Twin bangs framing both sides of face",
                "physics_stiffness": 0.65
            },
            "distinguishing_marks": [
                "Six-Tomoe Rinnegan in left eye",
                "Eternal Mangekyo Sharingan in right eye",
                "Kusanagi sword scabbard worn at hip"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "High-collared grey short-sleeve tunic open at chest",
            "outer_wear": "Dark blue hooded traveling cloak with purple lining",
            "bottom_wear": "Dark blue shinobi pants wrapped with thick purple shimenawa rope belt",
            "footwear": "Black high shinobi sandals",
            "relics_and_gear": [
                "Sword of Kusanagi (straight chokuto blade with black sheath)",
                "Purple shimenawa rope belt"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 62,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "articulation_type": "Full 5-finger independent phalange articulation",
                "grip_style": "One-handed seal formation and reverse Chidori sword thrust"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "145°",
                "ankle_mobility": "Precise acrobatic footwork"
            },
            "spring_bone_colliders": ["shimenawa_rope", "cloak_hem", "ducktail_hair"],
            "blendshapes": ["cold_scowl", "sharingan_ignite", "rinnegan_pulse", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Lightning-Fast Execution, Zero-Fluff Precision & Strict Code Isolation",
            "motto": "Hmph. If you can't optimize this query yourself, stay out of my way.",
            "state_reactions": {
                "Idle": "Rests hand on the hilt of his Kusanagi sword, staring coldly past the monitor",
                "Working": "Chidori sparks across the keyboard: sub-millisecond query execution with surgical precision",
                "Thinking": "Rinnegan activates: 'Amenotejikara... swapping the broken pointer with the safe immutable reference.'",
                "Success": "Sheathes Kusanagi with a faint click: 'Hmph. As expected.'",
                "Error": "Mangekyo Sharingan blazes crimson: 'Amaterasu. Burn this legacy technical debt to black ash.'"
            }
        }
    },
    {
        "id": "char.saitama",
        "slug": "saitama",
        "name": "Saitama",
        "native_name": "サイタマ (One Punch Man)",
        "origin_lore": "One-Punch Man",
        "creator": "ONE & Yusuke Murata",
        "archetype": "The Hero for Fun & Breaker of Limiters",
        "category": "anime_character",
        "canonical_stats": {
            "age": 25,
            "birthday": "Unknown",
            "height_cm": 175,
            "weight_kg": 70,
            "zodiac": "Cancer",
            "blood_type": "AB"
        },
        "micro_physical_details": {
            "skin_tone": "#f5d0ba",
            "skin_description": "Clean, smooth complexion with completely hairless scalp",
            "eyes": {
                "color_base": "#27272a",
                "color_highlight": "#71717a",
                "pupil_style": "Derp mode: simple oval black dots; Serious mode: razor-sharp piercing shaded irises",
                "limbal_ring": "#09090b",
                "expression_default": "Completely disinterested derp face; shifts instantaneously to terrifying serious mode"
            },
            "hair": {
                "base_color": "#000000",
                "tip_gradient": "#000000",
                "style": "Completely bald head polished to a high mirror shine",
                "length": "0 cm (Bald)",
                "bangs_description": "None (Bald)",
                "physics_stiffness": 1.0
            },
            "distinguishing_marks": [
                "Shining bald head that reflects stadium lights",
                "Instant shift between comedic doodle face and hyper-detailed Yusuke Murata serious face",
                "Unbroken Limiter: infinitely scalable strength"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "None",
            "outer_wear": "Bright yellow one-piece hero jumpsuit with white round zipper collar",
            "bottom_wear": "Black belt with large round gold buckle",
            "footwear": "Thick red leather superhero boots with black soles",
            "relics_and_gear": [
                "Flowing white hero cape fastened with large round black buttons to shoulders",
                "Pair of thick red rubber hero gloves",
                "Supermarket grocery bags filled with discounted scallions"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 56,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "articulation_type": "Full 5-finger independent phalange articulation",
                "grip_style": "Normal Punch fist and Serious Punch planet-shattering punch"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "145°",
                "ankle_mobility": "Solid unbudging footing"
            },
            "spring_bone_colliders": ["white_cape_hem", "belt_buckle"],
            "blendshapes": ["derp_face", "serious_face", "bargain_day_panic", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/readyplayer.me.glb",
            "thumbnail_url": "./public/thumbnails/readyplayer.me.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Instant Resolution, Single-Action Bug Annihilation & Supermarket Reminders",
            "motto": "OK. One punch is all I need. Wait, today is Tuesday discount day at the market?!",
            "state_reactions": {
                "Idle": "Picks his ear with pinky finger, looking blankly at your terminal: 'OK.'",
                "Working": "Throws one single punch at the build server: 0.001s compilation time",
                "Thinking": "Eyes turn into panic spirals: 'Did I leave the hot pot boiling at home?!'",
                "Success": "Delivers the Serious Punch: 'All 5,000 legacy bugs obliterated in one motion. OK.'",
                "Error": "Stares blankly: 'Can I punch this error? No? Then you deal with it, Genos.'"
            }
        }
    },
    {
        "id": "char.lord_rama",
        "slug": "rama",
        "name": "Lord Rama (Maryada Purushottama)",
        "native_name": "भगवान श्री राम (मर्यादा पुरुषोत्तम)",
        "origin_lore": "Sanatana Dharma / Ramayana / Ayodhya Kanda",
        "creator": "Eternal Seventh Avatar of Vishnu",
        "archetype": "The Supreme Embodiment of Righteousness, Duty & Unshakable Dharma",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Eternal / Treta Yuga",
            "birthday": "Rama Navami (Chaitra Shukla Navami)",
            "height_cm": 182,
            "weight_kg": 76,
            "zodiac": "Cancer (Punarvasu)",
            "blood_type": "Divine"
        },
        "micro_physical_details": {
            "skin_tone": "#38bdf8",
            "skin_description": "Megha shyamala: radiant blue like a fresh raincloud, carrying supreme nobility and royal grace",
            "eyes": {
                "color_base": "#1e293b",
                "color_highlight": "#38bdf8",
                "pupil_style": "Large blooming lotus eyes (Rajeeva Lochana) reflecting boundless compassion and dharma",
                "limbal_ring": "#0f172a",
                "expression_default": "Tranquil, royal, compassionate, unyielding in truth"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Lustrous dark curls tied into an ascetic top-knot bun (Jata) during exile, crowned with royal Mukut in Ayodhya",
                "length": "Shoulder-length royal locks",
                "bangs_description": "Clean brow marked with sacred Urdhva Pundra white and red Chandan tilak",
                "physics_stiffness": 0.50
            },
            "distinguishing_marks": [
                "Ajanubahu: long graceful arms reaching all the way to knees",
                "Kodanda celestial divine bow slung over left shoulder",
                "Inexhaustible quiver (Akshaya Tunira) filled with golden arrows"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Sacred white Yajnopavita thread across chest",
            "outer_wear": "Ascetic bark garments (Valkala) or brilliant golden royal silk robes with emerald brocade",
            "bottom_wear": "Saffron-gold dhoti tied with royal elegance",
            "footwear": "Wooden Paduka sandals (which ruled Ayodhya with devotion upon the throne)",
            "relics_and_gear": [
                "Kodanda (Divine bow of cosmic justice)",
                "Brahmashira and celestial divine arrows of truth"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 66,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "articulation_type": "Full 5-finger independent phalange articulation",
                "grip_style": "Kodanda bow grip and arrow-drawing string tension"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "145° (warrior stance of dharma)",
                "ankle_mobility": "Firm unshakeable balance"
            },
            "spring_bone_colliders": ["bow_string", "quiver_straps", "hair_curls", "dhoti_pleats"],
            "blendshapes": ["serene_grace", "dharma_resolve", "compassion", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Sendagaya_Shino.vrm",
            "thumbnail_url": "./public/thumbnails/Sendagaya_Shino.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Flawless Ethical Compliance, Long-Term Stability & Righteous Architecture",
            "motto": "Raghukul reet sada chali aayi, pran jaaye par vachan na jaayi. A contract promised must be fulfilled.",
            "state_reactions": {
                "Idle": "Holds the Kodanda bow gracefully, standing with royal composure and gentle eyes",
                "Working": "Draws an arrow of pure truth, striking through architecture flaws with single-shot precision",
                "Thinking": "Contemplates duty and righteousness: 'Let every API contract remain honest and unbreakable.'",
                "Success": "Smiles with profound warmth: 'Righteousness prevails. The release is worthy of Ayodhya.'",
                "Error": "Stands firm as a mountain: 'Do not yield to despair. Stand by truth and the path shall clear.'"
            }
        }
    },
    {
        "id": "char.goddess_durga",
        "slug": "durga",
        "name": "Goddess Durga (Mahishasuramardini)",
        "native_name": "देवी दुर्गा (महिषासुरमर्दिनी)",
        "origin_lore": "Sanatana Dharma / Devi Mahatmyam / Chandi Path",
        "creator": "Combined Cosmic Radiance (Tejas) of All Devas",
        "archetype": "The Invincible Mother, Slayer of the Demon of Delusion & Supreme Protectress",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Eternal",
            "birthday": "Durga Puja (Navaratri)",
            "height_cm": 174,
            "weight_kg": 60,
            "zodiac": "All (Supreme Shakti)",
            "blood_type": "Divine Shakti"
        },
        "micro_physical_details": {
            "skin_tone": "#fde047",
            "skin_description": "Tapta kanchana varna: brilliant molten-gold complexion radiating the combined light of 1,000 suns",
            "eyes": {
                "color_base": "#b91c1c",
                "color_highlight": "#f87171",
                "pupil_style": "Three blazing eyes (Trinayani) filled with invincible motherly protection and fiery justice",
                "limbal_ring": "#450a0a",
                "expression_default": "Majestic, serene, fearless, supremely powerful"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Lustrous dark mane crowned with an ornate golden Kirita Mukuta crown glowing with gems",
                "length": "Long cascading locks",
                "bangs_description": "Third Eye glowing with red Kumkum tilak on brow",
                "physics_stiffness": 0.45
            },
            "distinguishing_marks": [
                "Rides a fierce, majestic golden lion (Simha Vahana)",
                "Ten arms brandishing celestial divine weapons gifted by each of the Gods",
                "Three eyes symbolizing fire, sun, and moon"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Golden armor bodice encrusted with rubies",
            "outer_wear": "Vibrant blood-red Banarasi silk saree woven with radiant gold thread",
            "bottom_wear": "Graceful red and gold silk pleats",
            "footwear": "Bare divine feet crushing the demon of ignorance",
            "relics_and_gear": [
                "Shiva's Trishula (Trident)",
                "Vishnu's Sudarshana Chakra (Discus)",
                "Varuna's Shankha (Conch shell)",
                "Indra's Vajra (Thunderbolt)"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Mixamo Bipedal",
            "bone_count": 72,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "articulation_type": "Full 5-finger independent phalange articulation",
                "grip_style": "Trishula impaling strike and Abhaya Mudra fearlessness"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 125°]",
                "knee_flexion_limit": "150° (lion-riding posture)",
                "ankle_mobility": "Sovereign grace"
            },
            "spring_bone_colliders": ["red_saree_pallu", "lion_mane", "golden_crown"],
            "blendshapes": ["fearless_majesty", "motherly_love", "chandi_wrath", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Victoria_Rubin.vrm",
            "thumbnail_url": "./public/thumbnails/Victoria_Rubin.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Total Security Defense, Zero-Day Vulnerability Elimination & Indomitable Courage",
            "motto": "Ya Devi Sarvabhuteshu Shakti-Rupena Samsthita. The invincible shield over your systems.",
            "state_reactions": {
                "Idle": "Stands beside her golden lion with supreme majesty, ten weapons glowing in calm readiness",
                "Working": "Strikes through malicious penetration attempts with the radiant golden Trishula",
                "Thinking": "Three eyes illuminate the dark attack vectors: 'No backdoor shall evade detection.'",
                "Success": "Blows the divine conch shell: 'Victory is won! The vulnerability is utterly destroyed!'",
                "Error": "Roars alongside her lion: 'Rise! No setback can prevail against the power of courage!'"
            }
        }
    }
]
