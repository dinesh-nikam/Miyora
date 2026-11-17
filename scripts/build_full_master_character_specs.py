import json
import os

# Read existing 8 characters
with open('docs/detailed_character_specifications.json', 'r', encoding='utf-8') as f:
    existing_chars = json.load(f)

additional_characters = [
    # -------------------------------------------------------------
    # 9. ANIME: Son Goku
    # -------------------------------------------------------------
    {
        "id": "char.son_goku",
        "slug": "goku",
        "name": "Son Goku (Kakarot)",
        "native_name": "孫悟空 (カカロット)",
        "origin_lore": "Dragon Ball / Dragon Ball Z / Super",
        "creator": "Akira Toriyama",
        "archetype": "Earth's Greatest Defender & Saiyan Martial Artist",
        "category": "anime_character",
        "canonical_stats": {
            "age": 40,
            "birthday": "April 16, Age 736",
            "height_cm": 175,
            "weight_kg": 62,
            "zodiac": "Aries",
            "blood_type": "Saiyan"
        },
        "micro_physical_details": {
            "skin_tone": "#fcd34d",
            "skin_description": "Sun-kissed tanned golden skin with dense, explosive muscle definition and broad shoulders",
            "eyes": {
                "color_base": "#171717",
                "color_highlight": "#38bdf8",
                "pupil_style": "Round dark black pupils (piercing electric cyan-green in Super Saiyan forms)",
                "expression_default": "Bright innocent enthusiasm or intense battle concentration"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#f59e0b",
                "style": "Signature unyielding black Saiyan spikes: three bangs hanging to the right and five distinct crest spikes radiating left and backward",
                "length": "Medium voluminous spikes",
                "physics_stiffness": 0.85
            },
            "distinguishing_marks": [
                "Tail mark: Base of spine where his prehensile Saiyan monkey tail was permanently removed",
                "Turtle Hermit (Kame) and King Kai (Kaio) kanji emblems printed proudly on gi front and back"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Dark navy-blue weighted compression undershirt",
            "outer_wear": "Iconic sleeveless orange martial arts gi with V-neck opening and deep blue belt sash (obi)",
            "bottom_wear": "Matching loose orange martial arts pants tucked into boots",
            "footwear": "Dark navy-blue weighted combat boots with red laces, yellow edge trims, and thick white rubber soles",
            "relics_and_gear": [
                "Flying Nimbus (Kinto'un): Pure golden cloud used for aerial traversal",
                "Power Pole (Nyoi-bo): Magical red expanding staff strapped over back in youth"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Muscular Martial Biped",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Kamehameha cupped palm posture and tight clenched fist guards"
            },
            "leg_articulation": {
                "hip_swing_range": "[-50°, 130°]",
                "knee_flexion_limit": "150° (allows explosive martial high kicks and horse-riding stances)",
                "ankle_mobility": "Spring-loaded ankle flexibility"
            },
            "spring_bone_colliders": ["spiky_hair_clusters", "belt_sash_tails", "wristband_edges"],
            "blendshapes": ["cheerful_smile", "kamehameha_scream", "super_saiyan_grit", "eating_happy", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "High-Throughput Load Testing & Distributed Stress Resilience",
            "motto": "Hey, I'm Goku! Let's train this cluster until it can handle a billion queries per second!",
            "state_reactions": {
                "Idle": "Hands clasped behind head with big carefree grin, tail of belt fluttering in the breeze",
                "Working": "Furious lightning-fast keystrokes producing a faint golden aura around hands",
                "Thinking": "Scratches back of his head with funny sheepish grin: 'Gee, this distributed deadlock looks tough!'",
                "Success": "Flashing double thumbs up with beaming grin: 'We did it! Time for a giant bowl of ramen!'",
                "Error": "Drops into martial stance, eyes blazing: 'Now I'm getting excited! Let's hit that bug with a 10x Kaioken!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 10. ANIME: Monkey D. Luffy
    # -------------------------------------------------------------
    {
        "id": "char.monkey_d_luffy",
        "slug": "luffy",
        "name": "Monkey D. Luffy",
        "native_name": "モンキー・D・ルフィ",
        "origin_lore": "One Piece",
        "creator": "Eiichiro Oda",
        "archetype": "Captain of the Straw Hat Pirates & Warrior of Liberation (Sun God Nika)",
        "category": "anime_character",
        "canonical_stats": {
            "age": 19,
            "birthday": "May 5",
            "height_cm": 174,
            "weight_kg": 64,
            "zodiac": "Taurus",
            "blood_type": "F"
        },
        "micro_physical_details": {
            "skin_tone": "#fed7aa",
            "skin_description": "Sun-bronzed sea-weathered skin, lean wiry athletic musculature with rubber elasticity",
            "eyes": {
                "color_base": "#171717",
                "color_highlight": "#475569",
                "pupil_style": "Large round black eyes with simple pupil dots, utterly devoid of guile or fear",
                "expression_default": "Massive ear-to-ear grin full of boundless adventurous spirit"
            },
            "hair": {
                "base_color": "#09090b",
                "tip_gradient": "#18181b",
                "style": "Shaggy, unruly black hair peeking out from underneath his straw hat rim",
                "length": "Short shaggy crop",
                "physics_stiffness": 0.50
            },
            "distinguishing_marks": [
                "Two-stitch curved scar under left eye (self-inflicted in childhood to prove bravery to Shanks)",
                "Large X-shaped burn scar across center of chest (inflicted by Akainu during Marineford)"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Bare chest open under vest",
            "outer_wear": "Open red button-down sleeveless vest with gold buttons",
            "bottom_wear": "Rolled-up blue denim Bermuda shorts with white fluffy fleece cuffs, fastened by a yellow sash",
            "footwear": "Simple woven straw sandals with black straps",
            "relics_and_gear": [
                "Shanks' Straw Hat: Woven yellow straw with a distinctive red ribbon band, tied around neck by string in combat"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Elastic Biped",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Wide gripping fists with stretchable limb deformers"
            },
            "leg_articulation": {
                "hip_swing_range": "[-50°, 140°]",
                "knee_flexion_limit": "155° (enables rubber recoil springboard stances)",
                "ankle_mobility": "High elastic rebound"
            },
            "spring_bone_colliders": ["straw_hat_rim", "yellow_sash_ends", "shaggy_hair"],
            "blendshapes": ["huge_grin", "gear_5_laugh", "determined", "shocked_eyes_out", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Rapid Greenfield Prototyping & Fearless Tech Stack Exploration",
            "motto": "I'm gonna be King of the Code! Let's set sail into unchartered architecture!",
            "state_reactions": {
                "Idle": "Laughs with hand on his straw hat, rocking back on his straw sandals",
                "Working": "Types with hilarious stretching arm flourishes, grinning at code compilation",
                "Thinking": "Fingers probe nose absentmindedly: 'Hmm, I don't get this database thing, but let's punch it!'",
                "Success": "Arms shoot straight up into the air with a giant roar: 'MEAT!! The build passed!'",
                "Error": "Comedic jaw-drop eyes-bulging shock, then instantly grins: 'Alright, Gear Second, let's fix it!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 11. ANIME: Roronoa Zoro
    # -------------------------------------------------------------
    {
        "id": "char.roronoa_zoro",
        "slug": "zoro",
        "name": "Roronoa Zoro",
        "native_name": "ロロノア・ゾロ",
        "origin_lore": "One Piece",
        "creator": "Eiichiro Oda",
        "archetype": "Master of Three-Sword Style (Santoryu) & King of Hell",
        "category": "anime_character",
        "canonical_stats": {
            "age": 21,
            "birthday": "November 11",
            "height_cm": 181,
            "weight_kg": 80,
            "zodiac": "Scorpio",
            "blood_type": "XF"
        },
        "micro_physical_details": {
            "skin_tone": "#f5d0be",
            "skin_description": "Rugged, deeply tanned skin with massive muscular physique and heavy battle scars",
            "eyes": {
                "color_base": "#1e293b",
                "color_highlight": "#10b981",
                "pupil_style": "Sharp intimidating grey-black right eye; left eye permanently scarred shut with a vertical slice",
                "expression_default": "Stoic scowl or bloodthirsty confident grin during battle"
            },
            "hair": {
                "base_color": "#15803d",
                "tip_gradient": "#22c55e",
                "style": "Short cropped moss-green hair (Marimo)",
                "length": "Short buzz/crop cut",
                "physics_stiffness": 0.90
            },
            "distinguishing_marks": [
                "Vertical scar slicing cleanly across left eye",
                "Massive diagonal slash scar across chest from left shoulder to right hip (inflicted by Dracule Mihawk)",
                "Scars on both ankles from attempting to sever his own feet at Little Garden",
                "Three identical gold teardrop hoop earrings dangling from left earlobe"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Dark green wool haramaki (stomach band) holding his three katana scabbards",
            "outer_wear": "Long dark green samurai coat tucked into belt, open at the chest to display scar",
            "bottom_wear": "Dark green trousers tucked into boots",
            "footwear": "Heavy black leather knee-high pirate boots",
            "relics_and_gear": [
                "Wado Ichimonji: Pure white hilt/sheath katana (held in mouth for Santoryu)",
                "Sandai Kitetsu: Cursed red-lacquered katana",
                "Enma: Legendary demon sword capable of drawing out the user's advanced Conqueror's Haki",
                "Black bandanna tied around left bicep (tied around head only for serious mortal combat)"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Heavy Swordsman",
            "bone_count": 59,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Dual katana side grip with mouth-blade jaw parenting bone"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 120°]",
                "knee_flexion_limit": "145° (enables deep Onigiri lunge)",
                "ankle_mobility": "Firm rooted stance"
            },
            "spring_bone_colliders": ["green_coat_tails", "scabbard_trio", "earrings", "bicep_bandanna"],
            "blendshapes": ["stoic_scowl", "sword_in_mouth", "bloodthirsty_grin", "sleeping", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/masc_vroid.vrm",
            "thumbnail_url": "./public/thumbnails/masc_vroid.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Deep Refactoring & Cutting Monolithic Code into Modular Slices",
            "motto": "Nothing happened. Just clean interfaces and zero regressions.",
            "state_reactions": {
                "Idle": "Arms crossed over chest, head slumped forward taking a heavy power nap with one sword rattling",
                "Working": "Fierce single-minded focus carving through spaghetti code like a thousand-pound cannon",
                "Thinking": "Stares blankly at directory tree: 'Wait, did this directory move left? I think I'm lost again.'",
                "Success": "Sheathes katana with a metallic click: 'Three-thousand lines of tech debt... sliced.'",
                "Error": "Ties black bandanna over head: 'A segmentation fault? Let's see if it can withstand Enma.'"
            }
        }
    },

    # -------------------------------------------------------------
    # 12. ANIME: Anya Forger
    # -------------------------------------------------------------
    {
        "id": "char.anya_forger",
        "slug": "anya",
        "name": "Anya Forger (Test Subject 007)",
        "native_name": "アーニャ・フォージャー",
        "origin_lore": "Spy x Family",
        "creator": "Tatsuya Endo",
        "archetype": "Telepathic Escaped Subject & Starlight Princess",
        "category": "anime_character",
        "canonical_stats": {
            "age": 5,
            "birthday": "October (estimated)",
            "height_cm": 99.5,
            "weight_kg": 16,
            "zodiac": "Scorpio (estimated)",
            "blood_type": "Unknown"
        },
        "micro_physical_details": {
            "skin_tone": "#fdf2f8",
            "skin_description": "Soft baby-smooth rosy pale skin with round chubby cheeks",
            "eyes": {
                "color_base": "#10b981",
                "color_highlight": "#6ee7b7",
                "pupil_style": "Large vibrant emerald-green round anime eyes with exaggerated emotional range",
                "expression_default": "Wide innocent curiosity or the iconic smug 'heh' smirk (waku-waku)"
            },
            "hair": {
                "base_color": "#f472b6",
                "tip_gradient": "#fb7185",
                "style": "Soft strawberry-pink bob with small curled tips and a prominent ahoge strand atop head",
                "length": "Chin-length bob",
                "physics_stiffness": 0.35
            },
            "distinguishing_marks": [
                "Twin black conical horn-like hair ornaments with gold borders (housing telepathic antennae)",
                "Super-deformed chibi proportions: approximately 2.6 heads tall"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "White collared school blouse with red tie bow",
            "outer_wear": "Eden Academy Stella Scholar uniform: navy-blue pleated dress with gold embroidery around collar and cuffs",
            "bottom_wear": "White knee-high school socks",
            "footwear": "Polished black Mary Jane dress shoes",
            "relics_and_gear": [
                "Chimera-san plush doll (winged lion familiar with snake tail)",
                "Bag of crunchy roasted peanuts (secret energy supply)"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Chibi Humanoid / Petite 2.6 Heads",
            "bone_count": 206,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Tiny cute fists and wide open palm waku-waku gestures"
            },
            "leg_articulation": {
                "hip_swing_range": "[-30°, 85°]",
                "knee_flexion_limit": "130°",
                "ankle_mobility": "Toddler waddle and happy bounce"
            },
            "spring_bone_colliders": ["pink_bob_hair", "horn_ornaments", "dress_skirt", "chimera_tail"],
            "blendshapes": ["heh_smug", "waku_waku", "shocked_cry", "starlight_proud", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/AvatarSample_A.vrm",
            "thumbnail_url": "./public/thumbnails/AvatarSample_A.png"
        },
        "miyora_companion_role": {
            "work_specialty": "Developer Sentiment Analysis & Mental Focus Cheerleading",
            "motto": "Waku-waku! Papa's terminal is so exciting! Anya will help read the system's mind!",
            "state_reactions": {
                "Idle": "Hops softly on both feet while hugging Chimera-san, eyes sparkling with anticipation",
                "Working": "Slaps small keyboard with serious determined face: 'Anya is doing secret agent work!'",
                "Thinking": "Fingers touch her black horn clips: 'Zzzz-zzzz... Anya is reading the compiler's mind...'",
                "Success": "Flings hands in air with iconic 'Heh' face: 'Anya deserves ten bags of peanuts for this!'",
                "Error": "Teary-eyed trembling shock: 'Gasp! The bad guys broke the deployment! Papa, save me!'"
            }
        }
    },

    # -------------------------------------------------------------
    # 13. DEITY: Lord Ganesha (Vighnaharta)
    # -------------------------------------------------------------
    {
        "id": "char.lord_ganesha",
        "slug": "ganesha",
        "name": "Lord Ganesha (Vighnaharta)",
        "native_name": "गणेश (विघ्नहर्ता)",
        "origin_lore": "Ganesha Purana, Mudgala Purana, Shiva Purana",
        "creator": "Eternal Ancient Vedic Traditions",
        "archetype": "Remover of All Obstacles, Lord of Beginnings & Supreme Intellect (Buddhi-Pati)",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Eternal Divine Child / Sovereign of Ganas",
            "birthday": "Ganesh Chaturthi (Bhadrapada Shukla Chaturthi)",
            "height_cm": 160,
            "weight_kg": 95,
            "zodiac": "Muladhara Chakra Guardian",
            "blood_type": "Divine Prana"
        },
        "micro_physical_details": {
            "skin_tone": "#f59e0b",
            "skin_description": "Radiant golden-saffron complexion with majestic elephantine head and compassionate human body",
            "eyes": {
                "color_base": "#78350f",
                "color_highlight": "#fbbf24",
                "pupil_style": "Small, deeply wise and twinkle-filled elephant eyes radiating infinite intellectual acumen",
                "expression_default": "Playful, benevolent smile with gentle eyes brimming with infinite wisdom"
            },
            "hair": {
                "base_color": "#171717",
                "tip_gradient": "#fbbf24",
                "style": "Crowned by an elaborate golden jewel-studded Mukuta, with divine elephant trunk gracefully curving left",
                "length": "Crowned headpiece",
                "physics_stiffness": 0.80
            },
            "distinguishing_marks": [
                "Gajanan: Noble elephant head with broad ears that filter wisdom from trivial chatter",
                "Ekadanta: Single whole right tusk; left tusk broken off to transcribe the entire Mahabharata epic without pausing",
                "Lambodara: Large bountiful belly wrapped with a sacred serpent cord, symbolizing that the entire universe rests within him",
                "Trident-shaped Chandan and vermilion Tilaka centered proudly on forehead"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Soft sacred yellow silk dhoti undergarment",
            "outer_wear": "Rich golden-bordered saffron dhoti with royal purple and gold silk sash (angavastram)",
            "bottom_wear": "Seated gracefully in Lalitasana (one leg folded, one resting on lotus base)",
            "footwear": "Bare holy feet adorned with auspicious gold anklets",
            "relics_and_gear": [
                "Modak: Sweet golden delicacy held lovingly in the left palm and touched by trunk tip",
                "Ankusha (Elephant goad) in upper right hand: Steers humanity toward righteousness",
                "Pasha (Noose) in upper left hand: Captures and removes worldly attachments and obstacles",
                "Broken tusk stylus: Instrument of divine literary and coding mastery",
                "Mushika: Loyal divine mouse vahana sitting reverently at his feet"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Elephantine Sovereign",
            "bone_count": 65,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Abhaya Mudra (fearlessness) palm + Modak bowl cradle + Ankusha grip"
            },
            "leg_articulation": {
                "hip_swing_range": "[-35°, 95°]",
                "knee_flexion_limit": "135° (comfortable royal Lalitasana pose)",
                "ankle_mobility": "Solid grounding"
            },
            "spring_bone_colliders": ["elephant_ears", "trunk_segments", "crown_tassels", "silk_sash"],
            "blendshapes": ["trunk_happy_curve", "wise_smile", "ear_flutter", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/paladin.glb",
            "thumbnail_url": "./public/thumbnails/paladin.svg"
        },
        "miyora_companion_role": {
            "work_specialty": "CI/CD Blocker Removal & New Project Scaffolding Initiation",
            "motto": "Vakratunda Mahakaya: Removing all blockers from your pipeline before your first commit.",
            "state_reactions": {
                "Idle": "Regal seated Lalitasana pose, elephant ears gently fanning, trunk tasting a sweet modak",
                "Working": "Broken tusk stylus scribes flawless code with lightning clarity, zero compilation errors",
                "Thinking": "Ears flap slowly with deep contemplative hum: 'Every obstacle is merely a knot waiting to untie.'",
                "Success": "Trunk raises triumphantly in high trumpet salute, showering blessings upon the green build",
                "Error": "Ankusha glows with warm golden light, dissolving the pipeline roadblock in a single strike"
            }
        }
    },

    # -------------------------------------------------------------
    # 14. DEITY: Lord Hanuman (Bajrangbali)
    # -------------------------------------------------------------
    {
        "id": "char.lord_hanuman",
        "slug": "hanuman",
        "name": "Lord Hanuman (Bajrangbali)",
        "native_name": "हनुमान (बजरंगबली)",
        "origin_lore": "Ramayana, Ramcharitmanas, Hanuman Chalisa",
        "creator": "Sage Valmiki / Goswami Tulsidas",
        "archetype": "The Supreme Devotee, Master of Breath (Prana) & Indomitable Strength",
        "category": "deity_inspired",
        "canonical_stats": {
            "age": "Immortal Chiranjivi (Eternal Presence on Earth)",
            "birthday": "Hanuman Jayanti (Chaitra Purnima)",
            "height_cm": 185,
            "weight_kg": 90,
            "zodiac": "Rudra Avatara",
            "blood_type": "Divine Devotion"
        },
        "micro_physical_details": {
            "skin_tone": "#ea580c",
            "skin_description": "Vibrant Sindoor (vermilion/saffron-orange) glowing skin; diamond-hard indestructible body (Vajra-Anga)",
            "eyes": {
                "color_base": "#78350f",
                "color_highlight": "#fbbf24",
                "pupil_style": "Golden-brown eyes filled with profound humility, boundless devotion, and volcanic power",
                "expression_default": "Reverent devotion with hands folded at chest, or fearless heroic resolve"
            },
            "hair": {
                "base_color": "#c2410c",
                "tip_gradient": "#fbbf24",
                "style": "Lush reddish-golden ascetic mane crowned with a radiant golden Mukuta",
                "length": "Medium warrior mane",
                "physics_stiffness": 0.70
            },
            "distinguishing_marks": [
                "Sacred Janeu (Yajnopavita / holy sacred thread) worn across his broad chest from left shoulder to right hip",
                "Long prehensile tail curving gracefully upward over head like a protective parasol",
                "Chest can be opened to reveal the divine forms of Sita and Rama enshrined permanently in his heart"
            ]
        },
        "wardrobe_and_layering": {
            "inner_wear": "Sacred saffron kaupina (loincloth under-wrap)",
            "outer_wear": "Draped saffron-orange silk dhoti tightly secured with golden waistband",
            "bottom_wear": "Bare heroic feet planted with earth-shattering stability",
            "footwear": "Bare divine feet adorned with silver ankle ornaments",
            "relics_and_gear": [
                "Gada (Heavy golden mace): Symbol of concentrated cosmic strength and destruction of pride",
                "Dronagiri Mountain: Peak carrying the Sanjeevani life-saving herb resting on left palm",
                "Sacred garland of Tulsi leaves and Sindoor paste"
            ]
        },
        "anatomical_3d_rig": {
            "rig_standard": "VRM 1.0 Humanoid / Indomitable Warrior Biped",
            "bone_count": 65,
            "hand_articulation": {
                "finger_bones": 15,
                "fingers": ["Thumb", "Index", "Middle", "Ring", "Little"],
                "grip_style": "Namaskara Mudra (folded palms of devotion) and crushing two-handed Gada mace grip"
            },
            "leg_articulation": {
                "hip_swing_range": "[-45°, 130°]",
                "knee_flexion_limit": "150° (allows monumental ocean-crossing leap postures)",
                "ankle_mobility": "Rock-solid mountain grounding"
            },
            "spring_bone_colliders": ["tail_curve", "crown_tassels", "janeu_cord", "saffron_dhoti"],
            "blendshapes": ["devoted_humility", "heroic_roar", "gentle_smile", "blink", "a", "i", "u", "e", "o"],
            "model_url": "./public/models/Avatar_Orion.vrm",
            "thumbnail_url": "./public/thumbnails/Avatar_Orion.svg"
        },
        "miyora_companion_role": {
            "work_specialty": "Mission-Critical Incident Response & Unbreakable Production Recovery",
            "motto": "Jai Shri Ram: No service outage is beyond recovery when dedication is absolute.",
            "state_reactions": {
                "Idle": "Kneels reverently on one knee with palms folded together, tail poised gracefully overhead",
                "Working": "Powerhouse keystrokes, smashing through 10,000 log errors with joyful enthusiasm",
                "Thinking": "Chants mentally with closed eyes, invoking infinite inner clarity and memory",
                "Success": "Lifts golden mace into the air with triumphant roar: 'Production cluster restored!'",
                "Error": "Steadfast resolve: 'Even if the servers fall, I shall carry the entire infrastructure back to life!'"
            }
        }
    }
]

# Merge and remove duplicates
existing_ids = {c['id'] for c in existing_chars}
for ac in additional_characters:
    if ac['id'] not in existing_ids:
        existing_chars.append(ac)

out_path = 'docs/detailed_character_specifications.json'
with open(out_path, 'w', encoding='utf-8') as f:
    json.dump(existing_chars, f, indent=2, ensure_ascii=False)

print(f"Master Database updated! Total characters: {len(existing_chars)}")
