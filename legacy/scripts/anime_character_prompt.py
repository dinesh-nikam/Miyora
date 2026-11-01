"""
Anime Character Prompt Builder
Converts user input into detailed, original anime-character 3D generation prompts.
Enforces originality: strictly removes and replaces copyrighted franchise character names
(e.g., Nezuko, Naruto, Goku, Pikachu) with original stylistic descriptions.
"""

from __future__ import annotations

import re
from typing import Dict, List, Tuple

# Comprehensive mapping of copyrighted franchise characters to original descriptive archetypes
COPYRIGHT_REPLACEMENTS: Dict[str, str] = {
    r"\bnezuko\s*(kamado)?\b": "original demon-inspired anime heroine, long flowing black hair with gradient salmon-pink tips, soft pink eyes, bamboo-inspired accessory, Japanese-patterned clothing",
    r"\btanjiro\s*(kamado)?\b": "original swordsman youth, dark red-tinted hair, burgundy eyes, checked haori mantle, determined expression",
    r"\bzenitsu\s*(agatsuma)?\b": "original lightning warrior youth, yellow-to-orange gradient hair, golden eyes, thunder motif haori",
    r"\binosuke\s*(hashibira)?\b": "original dual-wielding wild warrior, wild dark hair with blue tips, emerald eyes, animal pelt details",
    r"\bnaruto\s*(uzumaki)?\b": "original spirited ninja youth, spiky blond hair, sky-blue eyes, energetic orange and black shinobi outfit",
    r"\bsasuke\s*(uchiha)?\b": "original cool avenger prodigy, high-collar dark tunic, dark swept hair, intense gaze",
    r"\bgoku\s*(son\s*goku)?\b": "original martial arts champion, wild spiky black hair, vibrant orange and navy gi, athletic build",
    r"\bvegeta\b": "original proud warrior prince, swept-up dark hair, futuristic segmented battle armor",
    r"\bpikachu\b": "original pocket electric mascot creature, bright golden yellow fur, cheerful expression, lightning-themed tail",
    r"\bluffy\s*(monkey\s*d\.?\s*luffy)?\b": "original adventurous sea captain, wide-brimmed straw hat, red vest, fearless grin",
    r"\bzoro\s*(roronoa\s*zoro)?\b": "original stoic swordsman, cropped moss-green hair, dark green coat, focused gaze",
    r"\bsanji\b": "original culinary martial artist, blond fringe covering one eye, formal slim suit",
    r"\bgojo\s*(satoru)?\b": "original sorcerer, snow-white spiky hair, sleek high-collar dark uniform, luminous cyan eyes",
    r"\byuji\s*(itadori)?\b": "original athletic brawler youth, spiked salmon-pink hair, red hoodie collar under dark uniform",
    r"\blevi\s*(ackerman)?\b": "original tactical scout captain, dark undercut hairstyle, white cravat, tailored green hooded cloak",
    r"\bmikasa\s*(ackerman)?\b": "original elite scout heroine, sleek black bob hair, dark red knit scarf, steadfast gaze",
    r"\banya\s*(forger)?\b": "original cute telepathic child, pastel pink bob hair, ornamental dark horn hairclips",
    r"\byor\s*(forger)?\b": "original elegant assassin heroine, long black hair, crimson-lined dark gown, golden accessories",
    r"\bfrieren\b": "original timeless elven mage, silver-white twintails, long pointed ears, ornate traveling robe",
    r"\bdenji\b": "original wild devil hunter, messy blond hair, relaxed white button-up shirt and necktie",
    r"\bpower\b": "original blood fiend heroine, strawberry-blond locks, vibrant red horns, unruly vibrant energy",
    r"\bsailor\s*moon\b": "original magical girl heroine, golden blonde twintails with buns, celestial sailor uniform",
    r"\bdeku\s*(izuku\s*midoriya)?\b": "original hero student, curly dark-green hair, green and black utility suit",
}

# Generic list of known copyrighted names for strict assertion checks
BANNED_FRANCHISE_NAMES: List[str] = [
    "nezuko", "tanjiro", "zenitsu", "inosuke", "kamado", "naruto", "sasuke",
    "sakura", "kakashi", "uchiha", "uzumaki", "goku", "vegeta", "gohan",
    "pikachu", "charizard", "pokemon", "pokémon", "luffy", "zoro", "sanji",
    "nami", "gojo", "sukuna", "itadori", "levi", "mikasa", "eren", "anya",
    "forger", "frieren", "denji", "makima", "sailor moon", "deku", "bakugo",
]


def sanitize_copyright(text: str) -> str:
    """
    Detect and replace any copyrighted franchise character names with original,
    descriptive stylistic phrases.
    """
    sanitized = text
    for pattern, replacement in COPYRIGHT_REPLACEMENTS.items():
        sanitized = re.sub(pattern, f"{replacement} ", sanitized, flags=re.IGNORECASE)

    # Secondary sweep: neutralize any remaining direct mentions
    for banned in BANNED_FRANCHISE_NAMES:
        pattern = rf"\b{re.escape(banned)}\b"
        sanitized = re.sub(pattern, "original stylized character", sanitized, flags=re.IGNORECASE)

    sanitized = re.sub(r"\s+", " ", sanitized).strip()
    return sanitized


def validate_prompt_originality(prompt: str) -> bool:
    """
    Verify that the final prompt does not contain any banned copyrighted names.
    Raises ValueError if a banned name is detected.
    """
    lower = prompt.lower()
    found = [name for name in BANNED_FRANCHISE_NAMES if re.search(rf"\b{re.escape(name)}\b", lower)]
    if found:
        raise ValueError(
            f"Prompt contains copyrighted franchise names: {found}. "
            "Prompts must specify only original designs."
        )
    return True


def extract_visual_elements(user_input: str) -> Dict[str, str]:
    """
    Parse short or detailed descriptions to detect visual attributes:
    archetype, hair, eyes, outfit, accessories, and expression.
    """
    text = user_input.strip()

    # Determine archetype / gender
    archetype = "girl character"
    if re.search(r"\b(boy|guy|male|prince|swordsman|warrior)\b", text, re.IGNORECASE):
        archetype = "boy character"
    elif re.search(r"\b(elf|fairy|mage)\b", text, re.IGNORECASE):
        archetype = "elven character"
    elif re.search(r"\b(creature|spirit|mascot)\b", text, re.IGNORECASE):
        archetype = "companion creature"

    # Hair extraction
    hair_match = re.search(
        r"\b(?:with\s+)?((?:long|short|spiky|fluffy|curly|flowing|gradient|black|pink|blonde|silver|blue|green|red|orange|purple|white|brown|dark|light)[\w\s-]*hair)",
        text,
        re.IGNORECASE,
    )
    if hair_match:
        hair = hair_match.group(1).strip()
        # Default to long pink hair if just 'pink hair'
        if hair.lower() == "pink hair":
            hair = "long pink hair"
    else:
        hair = "long hair"

    # Eyes extraction
    eyes_match = re.search(
        r"\b((?:pink|blue|cyan|emerald|green|golden|red|purple|amber|dark|large|expressive|soft)[\w\s-]*eyes)",
        text,
        re.IGNORECASE,
    )
    eyes = eyes_match.group(1).strip() if eyes_match else "large expressive eyes"

    # Outfit extraction
    outfit_match = re.search(
        r"\b((?:stylish|modern|japanese-inspired|traditional|school|casual|battle|cyberpunk)?[\w\s-]*(?:outfit|kimono|clothes|clothing|uniform|robe|jacket|suit|dress|haori|armor))",
        text,
        re.IGNORECASE,
    )
    if outfit_match:
        outfit = outfit_match.group(1).strip()
        if outfit.lower() in ("clothes", "clothing", "modern clothes"):
            outfit = "stylish modern outfit"
    else:
        outfit = "stylish modern outfit"

    # Accessories extraction
    accessory_match = re.search(
        r"([\w\s-]+(?:accessory|accessories|scarf|ribbon|horns|ears|headband|bamboo|hat|bow))",
        text,
        re.IGNORECASE,
    )
    accessory = accessory_match.group(1).strip() if accessory_match else ""

    # Expression extraction
    expression_match = re.search(
        r"([\w\s-]+(?:gentle|cheerful|cool|stoic|fierce|happy|serene)\s+expression)",
        text,
        re.IGNORECASE,
    )
    expression = expression_match.group(1).strip() if expression_match else ""

    return {
        "archetype": archetype,
        "hair": hair,
        "eyes": eyes,
        "outfit": outfit,
        "accessory": accessory,
        "expression": expression,
    }


def build_anime_prompt(user_input: str) -> str:
    """
    Convert a user's short or descriptive request into a production-grade,
    original anime-character 3D generation prompt.

    Guarantees:
    - Anchored with 'Original full-body 3D anime-style...'
    - Enforces clean silhouette, T-pose, clean topology, PBR textures, game-ready
    - Strips all copyrighted character names
    """
    if not user_input or not user_input.strip():
        raise ValueError("User input prompt cannot be empty.")

    # 1. Sanitize against franchise and copyrighted names
    clean_input = sanitize_copyright(user_input.strip())

    # Check if the user already provided a full, highly detailed prompt
    has_full_body = bool(re.search(r"full[- ]?body", clean_input, re.IGNORECASE))
    has_t_pose = bool(re.search(r"t[- ]?pose", clean_input, re.IGNORECASE))
    has_topology = bool(re.search(r"clean topology|game[- ]?ready", clean_input, re.IGNORECASE))

    if has_full_body and has_t_pose and has_topology:
        # Prompt is already detailed; ensure canonical prefix and clean structure
        prompt = clean_input
        if not prompt.lower().startswith("original"):
            prompt = f"Original {prompt}"
        # Canonicalize spacing and comma separation
        prompt = re.sub(r"\s+", " ", prompt).strip()
        validate_prompt_originality(prompt)
        return prompt

    # 2. Extract key visual attributes for concise prompt expansion
    elements = extract_visual_elements(clean_input)

    # Build canonical detailed prompt
    parts: List[str] = [
        f"Original full-body 3D anime-style {elements['archetype']}",
        elements["hair"],
        elements["eyes"],
        elements["outfit"],
    ]

    if elements["accessory"]:
        parts.append(elements["accessory"])

    if elements["expression"]:
        parts.append(elements["expression"])

    # Quality and 3D modeling anchors
    parts.extend([
        "clean silhouette",
        "appealing stylized proportions",
        "T-pose",
        "symmetrical face",
        "high-quality PBR textures",
        "clean topology",
        "game-ready 3D model",
    ])

    final_prompt = ", ".join(p.strip() for p in parts if p.strip())
    final_prompt = re.sub(r"\s+", " ", final_prompt).strip()

    validate_prompt_originality(final_prompt)
    return final_prompt


if __name__ == "__main__":
    test_inputs = [
        "anime girl with pink hair",
        "Nezuko in modern clothes",
        "anime boy warrior with spiky blue hair",
        "Original 3D anime-style girl character, long black-to-pink gradient hair, pink eyes, Japanese-inspired modern outfit, bamboo-inspired hair accessory, gentle expression, full body, T-pose, clean topology, game-ready.",
    ]
    for inp in test_inputs:
        res = build_anime_prompt(inp)
        print(f"Input:    {inp}\nOutput:   {res}\n")
