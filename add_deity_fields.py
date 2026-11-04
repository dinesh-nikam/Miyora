#!/usr/bin/env python3
"""Add deity/pet classification and broader spec fields to all characters in characters.json."""
import json

# Elemental affinities available
ELEMENTAL_AFFINITIES = [
    "fire", "water", "earth", "air", "lightning", "wind", "ice", "void",
    "light", "dark", "nature", "cosmic", "stellar", "lunar", "solar"
]

# Pet/deity archetypes
PET_ARCHETYPES = [
    "faithful familiar", "playful companion", "mystical spirit", "protective guardian",
    "wise mentor", "joyful friend", "serene presence", "watchful ally",
    "creative muse", "steadfast partner"
]

# Spiritual domains
SPIRITUAL_DOMAINS = [
    "code", "debugging", "architecture", "testing", "deployment",
    "creation", "destruction", "balance", "order", "chaos",
    "knowledge", "wisdom", "creativity", "healing", "protection"
]

# Manifestation forms
MANIFESTATION_FORMS = [
    "humanoid", "spirit form", "animal", "elemental", "celestial",
    "shadow", "glow", "statue", "projection", "dual-nature"
]

# Sacred artifacts
SACRED_ARTIFACTS = [
    "none", "staff", "wand", "crest", "amulet", "blade", "token",
    "crystal", "scroll", "blessing"
]

# Realm presence
REALM_PRESENCES = [
    "the digital ether", "the cloud", "the etheric plane",
    "the spirit realm", "the mortal plane", "the celestial realm"
]

# Mapping of character IDs to their likely classification
# Characters with mythological/divine themes → deity, others → pet
DIVINE_CHARACTERS = {
    "miyora.mira", "miyora.sun-wukong", "miyora.ignis", "miyora.draga", "miyora.ga", "miyora.saffi",
    "miyora.ganesha", "miyora.shiva", "miyora.parvati", "miyora.lakshmi", "miyora.saraswati",
    "miyora.hanuman", "miyora.krishna", "miyora.radha", "miyora.rama", "miyora.sita",
    "miyora.arjuna", "miyora.yuji", "miyora.tanjiro", "miyora.nezuko",
    "miyora.goku", "miyora.vegeta", "miyora.levi", "miyora.mikasa", "miyora.gon",
    "miyora.killua", "miyora.saitama", "miyora.genos", "miyora.denji", "miyora.power",
    "miyora.frieren", "miyora.pikachu", "miyora.kodama-spirit", "miyora.witch-apprentice",
    "miyora.sun-wukong"
}

def classify_character(char):
    """Determine pet/deity classification and assign appropriate fields."""
    char_id = char["id"]
    
    # Determine classification
    if char_id in DIVINE_CHARACTERS:
        classification = "deity"
    else:
        classification = "pet"
    
    # Assign elemental affinity (some have thematic elements)
    if classification == "deity":
        # Divine characters get meaningful elemental affinities
        elemental = "fire"  # default, could be more specific per character
        # Map based on name/theme for divine chars
        name_lower = char["name"].lower()
        if any(kw in name_lower for kw in ["fire", "ignis", "lava", "magma", "sun", "solar"]):
            elemental = "fire"
        elif any(kw in name_lower for kw in ["water", "ocean", "tide", "caspian", "caspian"]):
            elemental = "water"
        elif any(kw in name_lower for kw in ["ice", "frost", "winter", "glacier", "yuki", "suna"]):
            elemental = "ice"
        elif any(kw in name_lower for kw in ["wind", "sky", "sora", "lyra", "kaede", "kazane"]):
            elemental = "wind"
        elif any(kw in name_lower for kw in ["earth", "stone", "mountain", "bramble", "woodland"]):
            elemental = "earth"
        elif any(kw in name_lower for kw in ["light", "sun", "solar", "lumi", "luna", "nova"]):
            elemental = "light"
        elif any(kw in name_lower for kw in ["dark", "shadow", "night", "nyx", "shade", "velis"]):
            elemental = "void"
        else:
            elemental = "cosmic"
    else:
        # Pet characters: sometimes have elemental affinity, sometimes not
        if char_id in {"miyora.bao", "miyora.kuma", "miyora.nori", "miyora.mochi", "miyora.drako"}:
            elemental = "earth"  # animal/creature pets
        elif char_id in {"miyora.tora", "miyora.sol", "miyora.fawn"}:
            elemental = "nature"
        elif char_id in {"miyora.shadow", "miyora.velis", "miyora.zara"}:
            elemental = "void"
        elif char_id in {"miyora.sora", "miyora.kai", "miyora.amara"}:
            elemental = "light"
        elif char_id in {"miyora.yuki", "miyora.ivo", "miyora.bao"}:
            elemental = "ice"
        else:
            elemental = None
    
    # Assign spiritual domain
    if classification == "deity":
        domain = "creation"  # deities oversee creation domains
    else:
        # Map based on character specialty
        specialty = char.get("specialty", "").lower()
        if any(kw in specialty for kw in ["code", "debug", "pipeline", "concurrency"]):
            domain = "code"
        elif any(kw in specialty for kw in ["architecture", "system", "design"]):
            domain = "architecture"
        elif any(kw in specialty for kw in ["test", "quality", "benchmark"]):
            domain = "testing"
        elif any(kw in specialty for kw in ["deploy", "release", "courier"]):
            domain = "deployment"
        elif any(kw in specialty for kw in ["magic", "hex", "spell", "ritual"]):
            domain = "magic"
        elif any(kw in specialty for kw in ["heal", "mend", "cure"]):
            domain = "healing"
        elif any(kw in specialty for kw in ["forest", "nature", "garden", "botanist"]):
            domain = "nature"
        elif any(kw in specialty for kw in ["ocean", "sea", "water", "ship", "navigate"]):
            domain = "water"
        elif any(kw in specialty for kw in ["sky", "wind", "flight", "courier"]):
            domain = "air"
        elif any(kw in specialty for kw in ["fire", "flame", "forge", "smith"]):
            domain = "fire"
        elif any(kw in specialty for kw in ["ice", "cold", "winter", "glacier"]):
            domain = "ice"
        else:
            domain = "code"  # default
    
    # Assign manifestation form
    if classification == "deity":
        manifest = "celestial"  # deities have celestial manifestation
    else:
        # Pets often have animal or spirit forms
        body = char.get("body", "humanoid")
        if body == "humanoid":
            manifest = "humanoid"
        elif any(kw in char.get("name", "").lower() for kw in ["cat", "cat-", "feline", "kuma", "panda", "bear", "dog", "wolf", "fox", "bat", "rat", "raccoon"]):
            manifest = "animal"
        elif any(kw in char.get("name", "").lower() for kw in ["slime", "sprite", "wisp", "fairy", "sprite", "koala", "chipmunk"]):
            manifest = "spirit"
        else:
            manifest = "humanoid"
    
    # Assign sacred artifact
    if classification == "deity":
        artifact = "crest"  # divine crest or symbol
    else:
        artifact = "none"  # pets typically don't have sacred artifacts
    
    # Pet/deity archetype
    if classification == "deity":
        archetype = "divine entity"
    else:
        archetype = PET_ARCHETYPES[0]  # faithful familiar as default
    
    # Elemental touch for companion personality
    if classification == "deity" and elemental:
        elemental_touch = f" subtle {elemental} essence"
    elif classification == "pet" and elemental:
        elemental_touch = f" {elemental}-touched"
    else:
        elemental_touch = None
    
    # Sacred offering
    if classification == "deity":
        offering = "devotional prayer"
    else:
        offering = "treats and affection"
    
    # Update character
    char["pet_deity_classification"] = classification
    char["elemental_affinity"] = elemental
    char["spiritual_domain"] = domain
    char["manifestation_form"] = manifest
    char["sacred_artifact"] = artifact
    
    # Update companionPersonality with new fields
    cp = char.get("companionPersonality", {})
    cp["pet_deity_archetype"] = archetype
    cp["elemental_touch"] = elemental_touch
    cp["sacred_offering"] = offering
    cp["realm_presence"] = REALM_PRESENCES[0]  # will be set below
    char["companionPersonality"] = cp
    
    # Set realm_presence based on classification
    if classification == "deity":
        cp["realm_presence"] = "the celestial realm"
    else:
        cp["realm_presence"] = "the mortal plane"
    char["companionPersonality"] = cp
    
    return char

# Load the catalog
with open('crates/avatar/catalog/characters.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Process all characters
characters = data.get('characters', [])
print(f"Processing {len(characters)} characters...")

for i, char in enumerate(characters):
    char = classify_character(char)
    # Also ensure pet_deity_archetype is set in companionPersonality properly
    # (classify_character already does this)
    # Just verify the fields are there
    assert "pet_deity_classification" in char, f"Missing classification for {char['id']}"
    assert "elemental_affinity" in char, f"Missing elemental for {char['id']}"
    assert "spiritual_domain" in char, f"Missing domain for {char['id']}"
    assert "manifestation_form" in char, f"Missing manifestation for {char['id']}"
    assert "sacred_artifact" in char, f"Missing artifact for {char['id']}"
    assert "pet_deity_archetype" in char.get("companionPersonality", {}), f"Missing archetype for {char['id']}"
    assert "elemental_touch" in char.get("companionPersonality", {}), f"Missing elemental_touch for {char['id']}"
    assert "sacred_offering" in char.get("companionPersonality", {}), f"Missing sacred_offering for {char['id']}"
    assert "realm_presence" in char.get("companionPersonality", {}), f"Missing realm_presence for {char['id']}"
    
    if (i + 1) % 50 == 0:
        print(f"  Processed {i+1}/{len(characters)}")

# Save updated catalog
with open('crates/avatar/catalog/characters.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, indent=2, ensure_ascii=False)

print(f"Done! Updated {len(characters)} characters with deity/pet specification fields.")
print(f"Classification counts: deity={sum(1 for c in characters if c.get('pet_deity_classification') == 'deity')}, pet={sum(1 for c in characters if c.get('pet_deity_classification') == 'pet')}")