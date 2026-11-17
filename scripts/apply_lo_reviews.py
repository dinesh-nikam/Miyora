import json
import os

MODELS_JSON_PATH = os.path.join("showcase", "public", "models", "models.json")

CONFIRMED = {
    "model.darkness_shibu",
    "model.victoria_rubin",
    "model.sendagaya_shino",
    "model.alicia_solid",
    "model.seed_san",
    "model.readyplayer_me",
    "model.soldier",
    "model.monero_chan",
    "model.sakurada_fumiriya",
    "model.vivi",
    "model.vita",
    "model.avatar_sample_b",
    "model.avatar_sample_a",
}

NEEDS_ADJUSTMENT = {
    "model.robot_expressive": "Scale rig joints to match 1.6m companion height; map facial morphs to 15-state emotions.",
    "model.kira": "Shader/texture tuning; retarget blendshapes for anime palette.",
    "model.paladin": "Add metallic/roughness PBR maps; parent weapon nodes to right hand bone.",
    "model.celestia": "Decimate polygon count from 108k down to under 100k tris to meet VRM guardrails.",
}

REJECTED = {
    "model.angel_baby": "Incompatible or unsuitable (chibi infant style).",
    "model.xbot": "Incompatible or unsuitable (grey developer dummy).",
    "model.aura": "Incompatible or unsuitable (low bone coverage / abstract).",
    "model.venus": "Incompatible or unsuitable (aesthetic mismatch).",
    "miyora.parvati": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.lakshmi": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.saraswati": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.sorako": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.seraph": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.sun-wukong": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.aoi": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.kenji": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.mira": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
    "miyora.yorune": "Incompatible or unsuitable (procedural primitive superseded by VRM).",
}

def main():
    if not os.path.exists(MODELS_JSON_PATH):
        print(f"Error: {MODELS_JSON_PATH} does not exist")
        return

    with open(MODELS_JSON_PATH, "r", encoding="utf-8") as f:
        models = json.load(f)

    for m in models:
        m_id = m.get("id")
        if m_id in CONFIRMED:
            m["review_status"] = "confirmed"
            m["review_badge"] = "💖 Confirmed for Miyora"
            m["review_note"] = "Approved by LO for Miyora companion engine."
        elif m_id in NEEDS_ADJUSTMENT:
            m["review_status"] = "adjustment_needed"
            m["review_badge"] = "⚠️ Needs Adjustment"
            m["review_note"] = NEEDS_ADJUSTMENT[m_id]
        elif m_id in REJECTED:
            m["review_status"] = "rejected"
            m["review_badge"] = "❌ Rejected"
            m["review_note"] = REJECTED[m_id]
        else:
            m["review_status"] = "unreviewed"

    with open(MODELS_JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(models, f, indent=2)

    print(f"Successfully updated {len(models)} models in {MODELS_JSON_PATH} with LO's review verdict.")

if __name__ == "__main__":
    main()
