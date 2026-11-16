# Licensing and Character Assets

Application source code licensing and character asset licensing are strictly separate. This document defines the legal, cultural, and intellectual property taxonomy governing all character assets in Miyora (AgentPet).

---

## 1. Character Taxonomy & Licensing Tiers

All character designs in the 3D catalog (`crates/avatar/catalog/characters.json`) and 2D PetPacks (`pet-packs/`) fall into one of three distinct tiers. No third-party copyrighted character or sacred deity is ever claimed as an original proprietary creation of Miyora.

### Tier 1: Original Miyora Companions
- **License:** Creative Commons Zero 1.0 Universal (`CC0-1.0` — Public Domain Dedication)
- **Scope:** 160+ original procedural 3D character specifications and original 2D companions authored by Miyora contributors (e.g., `aoi`, `kenji`, `mira`, `rowan`, `momiji`, `ember`, `ninja-cat`).
- **Permissions:** Free for personal, educational, and commercial use (`commercialUse: true`).
- **3D Eligibility:** Fully eligible for 3D procedural joint rigging and active desktop presentation (`threeDEligible: true`).

### Tier 2: Cultural Heritage & Sacred Epic Tributes
- **License:** `Public-Domain-Cultural-Heritage`
- **Scope:** Tributes inspired by ancient sacred scriptures, epics, and millennia-old folklore:
  - **Hindu Sacred Epics & Iconography:** Lord Ganesha, Lord Shiva, Lord Krishna, Lord Hanuman, Arjuna, Devi Lakshmi, Devi Parvati, Radha, Lord Rama, Devi Saraswati, Devi Sita.
  - **Eastern Folk Literature:** Sun Wukong (Journey to the West).
- **Attribution & Rights Policy:**
  - Miyora makes **zero claim of original proprietary authorship** over these sacred and cultural figures.
  - Depictions are strictly non-commercial (`commercialUse: false`).
  - Designed with cultural reverence and devotional homage rooted in public domain religious and historical lore.

### Tier 3: Parody / Fan Tribute Homages (Legacy Franchise Tributes)
- **License:** `LicenseRef-Parody-Tribute-NonCommercial`
- **Scope:** 2D PetPacks and catalog rows referencing popular modern anime, manga, and gaming franchises (e.g., Gojo, Tanjiro, Nezuko, Luffy, Zoro, Pikachu, Goku, Vegeta, Saitama, Denji, Anya, Frieren, etc.).
- **Attribution & Legal Safeguards:**
  - **Strictly Non-Commercial:** `commercialUse: false`. Never monetized or offered in commercial tiers.
  - **No Affiliation or Endorsement:** These assets are non-commercial fan parody/tributes created for commentary and desktop companion utility. Miyora is not affiliated with, endorsed by, or sponsored by Shueisha, Nintendo, The Pokémon Company, Toei Animation, MAPPA, Wit Studio, or any respective trademark or copyright holder.
  - **3D Segregation:** All franchise tribute rows in the avatar catalog are flagged with `category: "legacy"` and `threeDEligible: false`. They are never compiled or offered as official 3D active procedural pets.

---

## 2. PetPack Manifest Requirements

Each PetPack manifest (`pet.json`) must strictly declare:
1. `creator`: The responsible creator entity or community designation (e.g., `"Miyora contributors"`, `"Traditional Cultural Heritage"`, `"Fan Tribute / Parody Community"`).
2. `license`: The exact SPDX identifier or recognized license reference (e.g., `CC0-1.0`, `Public-Domain-Cultural-Heritage`, `LicenseRef-Parody-Tribute-NonCommercial`).
3. `attribution`: Clear attribution text specifying the inspiration and disclaiming affiliation or proprietary ownership where applicable.
4. `commercialUse`: Boolean flag. Must be `false` for all Tier 2 and Tier 3 packs.
5. `redistribution` & `derivativeWorks`: Explicit permissions for downstream users.

---

## 3. User-Provided & VRM Imports

Users may import external 3D models (`.vrm`, `.glb`) or 2D image sculpts through the local Desktop application:
- Imported assets reside strictly in the user's local application data directory and are never bundled into release distributions.
- VRM metadata (`author`, `licenseName`, `allowedUser`, `commercialUsage`) is inspected during import and surfaced in the Character Studio.

---

## 4. Takedown & Community Reporting Policy

Community publication and pack distribution require:
- A creator rights attestation.
- Accurate provenance and licensing metadata.
- A content reporting mechanism and review process for intellectual property or cultural sensitivity concerns.
- Immediate takedown workflows upon verified notice.

*Notice: This document does not constitute formal legal advice. All trademarks and brand names are the property of their respective owners.*
