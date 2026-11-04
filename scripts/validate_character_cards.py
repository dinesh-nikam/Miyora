"""Validate bundled Character Card V2 files against the published spec and the catalog.

Read-only. Checks every ``crates/avatar/catalog/cards/*.card.json`` file for the
required ``chara_card_v2`` shape, prompt-size budgets, lorebook entry invariants,
and cross-file consistency with ``characters.json`` (name, palette, identity).

Usage:
    python scripts/validate_character_cards.py

Exits 1 when any card fails validation.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG_PATH = ROOT / "crates" / "avatar" / "catalog" / "characters.json"
CARDS_DIR = ROOT / "crates" / "avatar" / "catalog" / "cards"

SPEC_NAME = "chara_card_v2"
SPEC_VERSION = "2.0"
MAX_CARD_BYTES = 64 * 1024
MAX_LORE_BYTES = 12 * 1024
REQUIRED_DATA = (
    "name",
    "description",
    "personality",
    "scenario",
    "first_mes",
    "mes_example",
    "creator_notes",
    "system_prompt",
    "post_history_instructions",
    "alternate_greetings",
    "character_book",
    "tags",
    "creator",
    "character_version",
    "extensions",
)
PALETTE_KEYS = ("skin", "hair", "eyes", "primary", "secondary", "accent")
TAG_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
KEY_RE = re.compile(r"^[a-z0-9][a-z0-9 -]*$")
HEX_RE = re.compile(r"^#[0-9a-f]{6}$")
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")
CONTROL_RE = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")
POSITIONS = {"before_char", "after_char"}


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def require_text(errors: list[str], value, field: str, minimum: int, maximum: int) -> None:
    if not isinstance(value, str):
        fail(errors, f"{field} must be a string")
        return
    if minimum > 0 and not value.strip():
        fail(errors, f"{field} must be a non-empty string")
    if len(value) < minimum:
        fail(errors, f"{field} is {len(value)} chars; expected at least {minimum}")
    if len(value) > maximum:
        fail(errors, f"{field} is {len(value)} chars; expected at most {maximum}")
    if CONTROL_RE.search(value):
        fail(errors, f"{field} contains disallowed control characters")


def require_bool(errors: list[str], value, field: str) -> None:
    if not isinstance(value, bool):
        fail(errors, f"{field} must be a boolean")


def require_int(errors: list[str], value, field: str, minimum: int, maximum: int) -> None:
    if not isinstance(value, int) or isinstance(value, bool):
        fail(errors, f"{field} must be an integer")
        return
    if not minimum <= value <= maximum:
        fail(errors, f"{field} is {value}; expected {minimum}..{maximum}")


def require_string_list(errors: list[str], value, field: str, minimum: int, maximum: int, pattern=None) -> None:
    if not isinstance(value, list):
        fail(errors, f"{field} must be an array")
        return
    if not minimum <= len(value) <= maximum:
        fail(errors, f"{field} has {len(value)} items; expected {minimum}..{maximum}")
    if len(set(map(str, value))) != len(value):
        fail(errors, f"{field} contains duplicates")
    for index, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            fail(errors, f"{field}[{index}] must be a non-empty string")
            continue
        if len(item) > 64:
            fail(errors, f"{field}[{index}] is longer than 64 chars")
        if pattern is not None and not pattern.match(item):
            fail(errors, f"{field}[{index}] {item!r} has invalid characters")


def validate_character_book(errors: list[str], book) -> int:
    if not isinstance(book, dict):
        fail(errors, "data.character_book must be an object")
        return 0
    for field in ("name", "description"):
        if field not in book:
            fail(errors, f"data.character_book.{field} is required")
        else:
            require_text(errors, book[field], f"data.character_book.{field}", 1, 200)
    require_int(errors, book.get("scan_depth"), "data.character_book.scan_depth", 1, 20)
    require_int(errors, book.get("token_budget"), "data.character_book.token_budget", 128, 4096)
    require_bool(errors, book.get("recursive_scanning"), "data.character_book.recursive_scanning")
    if not isinstance(book.get("extensions"), dict):
        fail(errors, "data.character_book.extensions must be an object")

    entries = book.get("entries")
    if not isinstance(entries, list):
        fail(errors, "data.character_book.entries must be an array")
        return 0
    if not 4 <= len(entries) <= 32:
        fail(errors, f"character_book has {len(entries)} entries; expected 4..32")

    seen_ids: set[int] = set()
    seen_names: set[str] = set()
    seen_orders: set[int] = set()
    lore_bytes = 0
    enabled = 0

    for index, entry in enumerate(entries):
        where = f"character_book.entries[{index}]"
        if not isinstance(entry, dict):
            fail(errors, f"{where} must be an object")
            continue

        entry_id = entry.get("id")
        require_int(errors, entry_id, f"{where}.id", 1, 1_000_000)
        if isinstance(entry_id, int) and not isinstance(entry_id, bool):
            if entry_id in seen_ids:
                fail(errors, f"{where}.id {entry_id} is duplicated")
            seen_ids.add(entry_id)

        name = entry.get("name")
        require_text(errors, name, f"{where}.name", 1, 80)
        if isinstance(name, str):
            if name in seen_names:
                fail(errors, f"{where}.name {name!r} is duplicated")
            seen_names.add(name)

        keys = entry.get("keys")
        require_string_list(errors, keys, f"{where}.keys", 1, 8, KEY_RE)
        if isinstance(entry.get("secondary_keys", []), list):
            require_string_list(errors, entry.get("secondary_keys", []), f"{where}.secondary_keys", 0, 8, KEY_RE)
        else:
            fail(errors, f"{where}.secondary_keys must be an array")

        require_text(errors, entry.get("content"), f"{where}.content", 20, 1500)
        if isinstance(entry.get("content"), str):
            lore_bytes += len(entry["content"].encode("utf-8"))
        require_text(errors, entry.get("comment", ""), f"{where}.comment", 0, 160)

        insertion = entry.get("insertion_order")
        require_int(errors, insertion, f"{where}.insertion_order", 0, 100_000)
        if isinstance(insertion, int) and not isinstance(insertion, bool):
            if insertion in seen_orders:
                fail(errors, f"{where}.insertion_order {insertion} is duplicated")
            seen_orders.add(insertion)

        require_int(errors, entry.get("priority", 10), f"{where}.priority", 0, 10_000)
        for flag in ("enabled", "constant", "selective", "case_sensitive"):
            require_bool(errors, entry.get(flag, False), f"{where}.{flag}")
        if entry.get("enabled") is True:
            enabled += 1
        position = entry.get("position")
        if position is not None and position not in POSITIONS:
            fail(errors, f"{where}.position must be one of {sorted(POSITIONS)}")
        if not isinstance(entry.get("extensions", {}), dict):
            fail(errors, f"{where}.extensions must be an object")

    if enabled == 0:
        fail(errors, "character_book needs at least one enabled entry")
    if lore_bytes > MAX_LORE_BYTES:
        fail(errors, f"character_book content is {lore_bytes} bytes; budget is {MAX_LORE_BYTES}")
    return len(entries)


def validate_render_prompt(errors: list[str], prompt) -> None:
    if not isinstance(prompt, dict):
        fail(errors, "extensions.miyora.renderPrompt must be an object")
        return
    if prompt.get("format") != "miyora.render-prompt/v1":
        fail(errors, "renderPrompt.format must be 'miyora.render-prompt/v1'")
    require_text(errors, prompt.get("subject"), "renderPrompt.subject", 20, 400)
    require_string_list(errors, prompt.get("style"), "renderPrompt.style", 1, 12)
    for field in ("camera", "lighting"):
        block = prompt.get(field)
        if not isinstance(block, dict) or not block:
            fail(errors, f"renderPrompt.{field} must be a non-empty object")
            continue
        for key, value in block.items():
            if not isinstance(value, str) or not value.strip():
                fail(errors, f"renderPrompt.{field}.{key} must be a non-empty string")
    require_string_list(errors, prompt.get("details"), "renderPrompt.details", 1, 24)
    require_string_list(errors, prompt.get("negativePrompt"), "renderPrompt.negativePrompt", 1, 32)


def validate_card(path: Path, catalog: dict) -> tuple[list[str], str]:
    errors: list[str] = []
    characters = {
        character["id"]: character
        for character in catalog.get("characters", [])
        if isinstance(character, dict) and isinstance(character.get("id"), str)
    }
    raw = path.read_bytes()
    if len(raw) > MAX_CARD_BYTES:
        fail(errors, f"card is {len(raw)} bytes; budget is {MAX_CARD_BYTES}")
    try:
        card = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        return [f"card is not valid UTF-8 JSON: {exc}"], ""
    if not isinstance(card, dict):
        return ["card root must be an object"], ""

    if card.get("spec") != SPEC_NAME:
        fail(errors, f"spec must be {SPEC_NAME!r}")
    if card.get("spec_version") != SPEC_VERSION:
        fail(errors, f"spec_version must be {SPEC_VERSION!r}")

    data = card.get("data")
    if not isinstance(data, dict):
        return errors + ["data must be an object"], ""

    for field in REQUIRED_DATA:
        if field not in data:
            fail(errors, f"data.{field} is required")

    require_text(errors, data.get("name"), "data.name", 1, 32)
    require_text(errors, data.get("description"), "data.description", 200, 6000)
    require_text(errors, data.get("personality"), "data.personality", 100, 4000)
    require_text(errors, data.get("scenario"), "data.scenario", 40, 2000)
    require_text(errors, data.get("first_mes"), "data.first_mes", 40, 2000)
    require_text(errors, data.get("mes_example"), "data.mes_example", 80, 6000)
    require_text(errors, data.get("creator_notes"), "data.creator_notes", 20, 2000)
    require_text(errors, data.get("system_prompt"), "data.system_prompt", 40, 3000)
    require_text(errors, data.get("post_history_instructions"), "data.post_history_instructions", 20, 2000)
    require_text(errors, data.get("creator"), "data.creator", 1, 80)

    version = data.get("character_version")
    if not isinstance(version, str) or not SEMVER_RE.match(version):
        fail(errors, "data.character_version must be semver (for example 1.0.0)")

    example = data.get("mes_example")
    if isinstance(example, str) and ("{{user}}" not in example or "{{char}}" not in example):
        fail(errors, "data.mes_example must show both {{user}} and {{char}} turns")

    greetings = data.get("alternate_greetings")
    if not isinstance(greetings, list):
        fail(errors, "data.alternate_greetings must be an array")
    else:
        if not 2 <= len(greetings) <= 8:
            fail(errors, f"alternate_greetings has {len(greetings)} items; expected 2..8")
        for index, greeting in enumerate(greetings):
            require_text(errors, greeting, f"alternate_greetings[{index}]", 20, 2000)

    require_string_list(errors, data.get("tags"), "data.tags", 3, 12, TAG_RE)
    entry_count = validate_character_book(errors, data.get("character_book"))

    extensions = data.get("extensions")
    namespace = extensions.get("miyora") if isinstance(extensions, dict) else None
    if not isinstance(namespace, dict):
        fail(errors, "data.extensions.miyora must be an object")
        return errors, ""

    character_id = path.name[: -len(".card.json")]
    if namespace.get("characterId") != character_id:
        fail(errors, f"extensions.miyora.characterId must match filename ({character_id!r})")
    catalog_character = characters.get(character_id)
    if catalog_character is None:
        fail(errors, f"catalog has no character with id {character_id!r}")
        return errors, character_id

    if data.get("name") != catalog_character.get("name"):
        fail(errors, "data.name does not match catalog name")
    if namespace.get("catalogTitle") != catalog_character.get("title"):
        fail(errors, "extensions.miyora.catalogTitle does not match catalog title")
    if namespace.get("personality") != catalog_character.get("personality"):
        fail(errors, "extensions.miyora.personality does not match catalog personality")
    if namespace.get("specialty") != catalog_character.get("specialty"):
        fail(errors, "extensions.miyora.specialty does not match catalog specialty")
    if namespace.get("catchphrase") != catalog_character.get("catchphrase"):
        fail(errors, "extensions.miyora.catchphrase does not match catalog catchphrase")

    palette = namespace.get("palette")
    if not isinstance(palette, dict):
        fail(errors, "extensions.miyora.palette must be an object")
    else:
        for key in PALETTE_KEYS:
            color = palette.get(key)
            if not isinstance(color, str) or not HEX_RE.match(color):
                fail(errors, f"extensions.miyora.palette.{key} must be a #rrggbb color")
        if palette != catalog_character.get("palette"):
            fail(errors, "extensions.miyora.palette does not match the catalog palette")

    card_tags = data.get("tags") if isinstance(data.get("tags"), list) else []
    missing_tags = sorted(set(catalog_character.get("tags", [])) - set(card_tags))
    if missing_tags:
        fail(errors, f"card tags are missing catalog tags: {missing_tags}")

    require_string_list(errors, namespace.get("rigSignature"), "extensions.miyora.rigSignature", 1, 12)
    voice = namespace.get("voice")
    if not isinstance(voice, dict):
        fail(errors, "extensions.miyora.voice must be an object")
    else:
        for field in ("pronouns", "register", "metaphorBudget", "addressUserAs", "catchphraseUsage"):
            if not isinstance(voice.get(field), str) or not voice[field].strip():
                fail(errors, f"extensions.miyora.voice.{field} must be a non-empty string")

    validate_render_prompt(errors, namespace.get("renderPrompt"))

    provenance = namespace.get("provenance")
    if not isinstance(provenance, dict):
        fail(errors, "extensions.miyora.provenance must be an object")
    else:
        for field in ("creator", "license", "source"):
            if not isinstance(provenance.get(field), str) or not provenance[field].strip():
                fail(errors, f"extensions.miyora.provenance.{field} must be a non-empty string")
        if provenance.get("license") != catalog.get("license"):
            fail(errors, "extensions.miyora.provenance.license does not match the catalog license")

    return errors, f"{character_id}: {entry_count} lore entries, {len(greetings) if isinstance(greetings, list) else 0} greetings"


def main() -> int:
    try:
        catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"error: cannot read catalog: {exc}", file=sys.stderr)
        return 1
    paths = sorted(CARDS_DIR.glob("*.card.json"))
    if not paths:
        print(f"error: no character cards found under {CARDS_DIR}", file=sys.stderr)
        return 1

    failed = 0
    for path in paths:
        errors, summary = validate_card(path, catalog)
        if errors:
            failed += 1
            print(f"FAIL {path.name}", file=sys.stderr)
            for error in errors:
                print(f"  - {error}", file=sys.stderr)
        else:
            print(f"ok   {path.name} ({summary})")

    if failed:
        print(f"\n{failed} of {len(paths)} character cards failed validation", file=sys.stderr)
        return 1
    print(f"\nall {len(paths)} character cards passed validation")
    return 0


if __name__ == "__main__":
    sys.exit(main())
