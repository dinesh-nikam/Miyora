// Design harness for Yorune, the astral cartographer. Not shipped.
// Renders the real bundled spec through the production AvatarStage.
import type { CSSProperties } from "react";
import { createRoot } from "react-dom/client";
import cardJson from "../../../crates/avatar/catalog/cards/miyora.yorune.card.json";
import catalogJson from "../../../crates/avatar/catalog/characters.json";
import { AvatarStage } from "../src/avatar/AvatarStage";
import type { CharacterSpec } from "../src/avatar/types";

interface CardEntry {
  id: number;
  name: string;
  keys: string[];
  content: string;
  position: string;
  enabled: boolean;
  insertion_order: number;
}

interface CharacterCardV2 {
  spec: string;
  spec_version: string;
  data: {
    name: string;
    description: string;
    personality: string;
    scenario: string;
    first_mes: string;
    mes_example: string;
    creator_notes: string;
    system_prompt: string;
    post_history_instructions: string;
    alternate_greetings: string[];
    character_book: {
      name: string;
      description: string;
      scan_depth: number;
      token_budget: number;
      entries: CardEntry[];
    };
    tags: string[];
    creator: string;
    character_version: string;
    extensions: {
      miyora: {
        rigSignature: string[];
        voice: {
          pronouns: string;
          register: string;
          metaphorBudget: string;
          addressUserAs: string;
          catchphraseUsage: string;
        };
        renderPrompt: {
          format: string;
          subject: string;
          style: string[];
          camera: Record<string, string>;
          lighting: Record<string, string>;
          details: string[];
          negativePrompt: string[];
        };
        provenance: { creator: string; license: string; source: string };
      };
    };
  };
}

const card = cardJson as CharacterCardV2;
const characters = (catalogJson as { characters: CharacterSpec[] }).characters;
const found = characters.find((character) => character.id === "miyora.yorune");
if (!found) throw new Error("Yorune is missing from the bundled catalog");
const spec: CharacterSpec = found;
const personality = spec.companionPersonality;
const dialogue = personality?.dialogue ?? {};
const portrait = new URLSearchParams(window.location.search).get("portrait") === "1";
const cardData = card.data;
const renderPrompt = cardData.extensions.miyora.renderPrompt;
const voice = cardData.extensions.miyora.voice;
const cardMeta: Array<[string, string]> = [
  ["Spec", `${card.spec} · ${card.spec_version}`],
  ["Card version", cardData.character_version],
  ["Creator", cardData.extensions.miyora.provenance.creator],
  ["License", cardData.extensions.miyora.provenance.license],
  ["Address", voice.addressUserAs],
  ["Pronouns", voice.pronouns],
  ["Metaphor budget", voice.metaphorBudget],
  ["Lore budget", `${cardData.character_book.token_budget} tokens · depth ${cardData.character_book.scan_depth}`],
];

const PALETTE_LABELS: Array<[keyof CharacterSpec["palette"], string]> = [
  ["skin", "Skin"],
  ["hair", "Hair"],
  ["eyes", "Eyes"],
  ["primary", "Coat lining"],
  ["secondary", "Midnight coat"],
  ["accent", "Astral glow"],
];

const STATS: Array<[string, string]> = [
  ["Body", spec.body],
  ["Species", `Fox-eared ${spec.category}`],
  ["Wings", spec.wings],
  ["Hair", `${spec.hair} · ${spec.hairTip ?? "no tip fade"}`],
  ["Eyes", spec.eyes],
  ["Outfit", `${spec.outfit} · ${spec.pattern} pattern`],
  ["Height", `${spec.proportions.height.toFixed(2)}×`],
  ["Build", `${spec.proportions.build.toFixed(2)}×`],
  ["Head scale", `${spec.proportions.headScale.toFixed(2)}×`],
  ["Energy", spec.energy.toFixed(2)],
];

const panel: CSSProperties = {
  border: "1px solid rgba(233, 213, 255, 0.14)",
  borderRadius: 22,
  background: "linear-gradient(160deg, rgba(22, 33, 62, 0.72), rgba(9, 13, 32, 0.86))",
  boxShadow: "0 24px 80px rgba(2, 4, 14, 0.55)",
};

const label: CSSProperties = {
  fontSize: 11,
  letterSpacing: "0.18em",
  textTransform: "uppercase",
  color: "#a78bfa",
  marginBottom: 12,
};

const prose: CSSProperties = {
  fontSize: 13,
  lineHeight: 1.75,
  color: "#cbd5e1",
  whiteSpace: "pre-wrap",
};

function Swatch({ label, color }: { label: string; color: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <span style={{ width: 26, height: 26, borderRadius: 8, background: color, border: "1px solid rgba(255,255,255,0.18)" }} />
      <span style={{ display: "flex", flexDirection: "column", lineHeight: 1.25 }}>
        <span style={{ fontSize: 10, letterSpacing: "0.1em", textTransform: "uppercase", color: "#94a3b8" }}>{label}</span>
        <span style={{ fontSize: 11, color: "#e2e8f0", fontFamily: "Geist Mono, Consolas, monospace" }}>{color}</span>
      </span>
    </div>
  );
}

function YorunePortrait() {
  return (
    <div style={{ minHeight: "100vh", display: "grid", placeItems: "center", padding: 40 }}>
      <div style={{ display: "grid", justifyItems: "center", gap: 18 }}>
        <AvatarStage subject={{ kind: "procedural", spec }} state="idle" size={880} interactive={false} />
        <div style={{ textAlign: "center" }}>
          <div style={{ fontSize: 40, fontWeight: 800, letterSpacing: "-0.02em", color: "#f8fafc" }}>{spec.name}</div>
          <div style={{ fontSize: 13, letterSpacing: "0.22em", textTransform: "uppercase", color: "#c4b5fd" }}>{spec.title}</div>
        </div>
      </div>
    </div>
  );
}

function YoruneDossier() {
  return (
    <div style={{ width: 1560, margin: "0 auto", padding: "36px 44px 44px" }}>
      <header style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 24, marginBottom: 24 }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: "0.28em", textTransform: "uppercase", color: "#a78bfa", marginBottom: 10 }}>
            Miyora · Original companion · No. 214 · Card v{cardData.character_version}
          </div>
          <h1 style={{ margin: 0, fontSize: 58, lineHeight: 0.95, letterSpacing: "-0.035em", color: "#f8fafc" }}>{spec.name}</h1>
          <div style={{ marginTop: 10, fontSize: 16, letterSpacing: "0.18em", textTransform: "uppercase", color: "#f5c542" }}>
            {spec.title}
          </div>
        </div>
        <div style={{ textAlign: "right", fontSize: 12, lineHeight: 1.7, color: "#94a3b8" }}>
          <div>{personality?.icon} {spec.personality} · {spec.specialty}</div>
          <div>CC0-1.0 · Original Miyora design</div>
          <div>{spec.id}</div>
        </div>
      </header>

      <main style={{ display: "grid", gridTemplateColumns: "minmax(520px, 620px) minmax(440px, 1fr)", gap: 28, alignItems: "start" }}>
        <section style={{ ...panel, padding: 18, display: "grid", justifyItems: "center", gap: 14 }}>
          <AvatarStage subject={{ kind: "procedural", spec }} state="idle" size={584} interactive={false} />
          <div style={{ width: "100%", display: "grid", gap: 3, fontSize: 11, color: "#94a3b8" }}>
            <span>Signature rig detail</span>
            <span style={{ color: "#e9d5ff" }}>orrery · crescent · stitched star seam</span>
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            {spec.tags.map((tag) => (
              <span
                key={tag}
                style={{
                  fontSize: 10,
                  letterSpacing: "0.12em",
                  textTransform: "uppercase",
                  color: "#ddd6fe",
                  background: "rgba(192, 132, 252, 0.14)",
                  border: "1px solid rgba(192, 132, 252, 0.35)",
                  borderRadius: 999,
                  padding: "4px 10px",
                }}
              >
                {tag}
              </span>
            ))}
          </div>
        </section>

        <section style={{ display: "grid", gap: 18 }}>
          <div style={{ ...panel, padding: "20px 22px" }}>
            <p style={{ margin: 0, fontSize: 15, lineHeight: 1.75, color: "#cbd5e1" }}>{spec.description}</p>
            <blockquote
              style={{
                margin: "16px 0 0",
                padding: "12px 16px",
                borderLeft: "2px solid #f5c542",
                background: "rgba(245, 197, 66, 0.06)",
                borderRadius: "0 10px 10px 0",
                fontSize: 14,
                lineHeight: 1.6,
                color: "#fef3c7",
              }}
            >
              “{spec.catchphrase}”
            </blockquote>
          </div>

          <div style={{ ...panel, padding: "20px 22px" }}>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "#a78bfa", marginBottom: 14 }}>
              Field notes
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: "10px 22px" }}>
              {STATS.map(([label, value]) => (
                <div key={label} style={{ display: "flex", justifyContent: "space-between", gap: 12, borderBottom: "1px solid rgba(148, 163, 184, 0.12)", paddingBottom: 6 }}>
                  <span style={{ fontSize: 11, color: "#94a3b8" }}>{label}</span>
                  <span style={{ fontSize: 12, color: "#e2e8f0", textAlign: "right" }}>{value}</span>
                </div>
              ))}
            </div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 7, marginTop: 16 }}>
              {spec.accessories.map((accessory) => (
                <span
                  key={accessory}
                  style={{
                    fontSize: 11,
                    color: "#fde68a",
                    background: "rgba(245, 197, 66, 0.1)",
                    border: "1px solid rgba(245, 197, 66, 0.28)",
                    borderRadius: 7,
                    padding: "4px 9px",
                  }}
                >
                  {accessory.replace(/_/g, " ")}
                </span>
              ))}
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: 12, marginTop: 16 }}>
              {PALETTE_LABELS.map(([key, label]) => (
                <Swatch key={key} label={label} color={spec.palette[key]} />
              ))}
            </div>
          </div>

          <div style={{ ...panel, padding: "20px 22px" }}>
            <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "#a78bfa", marginBottom: 12 }}>
              Companion voice
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 10 }}>
              {Object.entries(dialogue).map(([state, line]) => (
                <div key={state} style={{ background: "rgba(10, 16, 29, 0.72)", border: "1px solid rgba(148, 163, 184, 0.12)", borderRadius: 12, padding: "10px 12px" }}>
                  <div style={{ fontSize: 9, letterSpacing: "0.16em", textTransform: "uppercase", color: "#818cf8", marginBottom: 4 }}>{state}</div>
                  <div style={{ fontSize: 12, lineHeight: 1.5, color: "#cbd5e1" }}>{line}</div>
                </div>
              ))}
            </div>
            <div style={{ marginTop: 12, fontSize: 11, color: "#94a3b8", lineHeight: 1.6 }}>
              Treat: {personality?.treatEmoji} {personality?.treatName} · Feed: {personality?.feedReaction}
            </div>
          </div>
        </section>
      </main>

      <section style={{ marginTop: 30 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", gap: 20, marginBottom: 14 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
            <h2 style={{ margin: 0, fontSize: 22, letterSpacing: "-0.015em", color: "#f8fafc" }}>Character Card V2</h2>
            <span
              style={{
                fontSize: 10,
                fontWeight: 700,
                letterSpacing: "0.16em",
                textTransform: "uppercase",
                color: "#0f172a",
                background: "#f5c542",
                borderRadius: 999,
                padding: "4px 10px",
              }}
            >
              {card.spec}
            </span>
          </div>
          <span style={{ fontSize: 11, color: "#94a3b8" }}>
            {cardData.character_book.name} · {cardData.character_book.entries.length} lore entries · {cardData.alternate_greetings.length} greetings
          </span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1.35fr) minmax(0, 1fr)", gap: 18, alignItems: "start" }}>
          <div style={{ display: "grid", gap: 18 }}>
            <div style={{ ...panel, padding: "20px 22px" }}>
              <div style={label}>Scenario</div>
              <p style={{ ...prose, margin: "0 0 20px" }}>{cardData.scenario}</p>
              <div style={label}>Opening transmission</div>
              <p style={{ ...prose, margin: 0, color: "#e2e8f0" }}>{cardData.first_mes}</p>
            </div>

            <div style={{ ...panel, padding: "20px 22px" }}>
              <div style={label}>Example exchange</div>
              <p style={{ ...prose, margin: 0 }}>{cardData.mes_example}</p>
            </div>
          </div>

          <div style={{ display: "grid", gap: 18 }}>
            <div style={{ ...panel, padding: "20px 22px" }}>
              <div style={label}>Render prompt · {renderPrompt.format}</div>
              <p style={{ ...prose, margin: "0 0 12px", color: "#e2e8f0" }}>{renderPrompt.subject}</p>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 14 }}>
                {renderPrompt.style.map((style) => (
                  <span
                    key={style}
                    style={{
                      fontSize: 10,
                      color: "#ddd6fe",
                      background: "rgba(192, 132, 252, 0.14)",
                      border: "1px solid rgba(192, 132, 252, 0.35)",
                      borderRadius: 999,
                      padding: "3px 9px",
                    }}
                  >
                    {style}
                  </span>
                ))}
              </div>
              <div style={{ display: "grid", gap: 7, marginBottom: 14 }}>
                {(
                  [
                    ["Camera", renderPrompt.camera],
                    ["Lighting", renderPrompt.lighting],
                  ] as Array<[string, Record<string, string>]>
                ).map(([name, block]) => (
                  <div
                    key={name}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      gap: 14,
                      borderBottom: "1px solid rgba(148, 163, 184, 0.12)",
                      paddingBottom: 6,
                    }}
                  >
                    <span style={{ fontSize: 10, letterSpacing: "0.12em", textTransform: "uppercase", color: "#94a3b8" }}>{name}</span>
                    <span style={{ fontSize: 11, color: "#e2e8f0", textAlign: "right" }}>{Object.values(block).join(" · ")}</span>
                  </div>
                ))}
              </div>
              <div style={{ ...label, marginBottom: 8 }}>Signature details</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginBottom: 14 }}>
                {renderPrompt.details.map((detail) => (
                  <span
                    key={detail}
                    style={{
                      fontSize: 10,
                      color: "#fde68a",
                      background: "rgba(245, 197, 66, 0.1)",
                      border: "1px solid rgba(245, 197, 66, 0.28)",
                      borderRadius: 7,
                      padding: "3px 8px",
                    }}
                  >
                    {detail}
                  </span>
                ))}
              </div>
              <div style={{ fontSize: 10, lineHeight: 1.7, color: "#94a3b8" }}>
                Negative: <span style={{ color: "#cbd5e1" }}>{renderPrompt.negativePrompt.join(" · ")}</span>
              </div>
            </div>

            <div style={{ ...panel, padding: "20px 22px" }}>
              <div style={label}>Card metadata</div>
              <div style={{ display: "grid", gap: 7 }}>
                {cardMeta.map(([name, value]) => (
                  <div
                    key={name}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      gap: 12,
                      borderBottom: "1px solid rgba(148, 163, 184, 0.12)",
                      paddingBottom: 6,
                    }}
                  >
                    <span style={{ fontSize: 11, color: "#94a3b8" }}>{name}</span>
                    <span style={{ fontSize: 11, color: "#e2e8f0", textAlign: "right" }}>{value}</span>
                  </div>
                ))}
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 12 }}>
                {cardData.tags.map((tag) => (
                  <span
                    key={tag}
                    style={{
                      fontSize: 10,
                      letterSpacing: "0.08em",
                      color: "#ddd6fe",
                      background: "rgba(192, 132, 252, 0.12)",
                      border: "1px solid rgba(192, 132, 252, 0.3)",
                      borderRadius: 999,
                      padding: "3px 8px",
                    }}
                  >
                    {tag}
                  </span>
                ))}
              </div>
              <p style={{ margin: "12px 0 0", fontSize: 10.5, lineHeight: 1.65, color: "#94a3b8" }}>{cardData.creator_notes}</p>
            </div>
          </div>
        </div>

        <div style={{ ...panel, marginTop: 18, padding: "20px 22px" }}>
          <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", gap: 18, marginBottom: 14 }}>
            <div style={{ ...label, marginBottom: 0 }}>Lorebook · {cardData.character_book.name}</div>
            <span style={{ maxWidth: 720, fontSize: 10.5, lineHeight: 1.6, color: "#94a3b8", textAlign: "right" }}>
              {cardData.character_book.description}
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 12 }}>
            {cardData.character_book.entries.map((entry) => (
              <div
                key={entry.id}
                style={{
                  background: "rgba(10, 16, 29, 0.72)",
                  border: "1px solid rgba(148, 163, 184, 0.12)",
                  borderRadius: 14,
                  padding: "12px 13px",
                }}
              >
                <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", gap: 8 }}>
                  <span style={{ fontSize: 12.5, fontWeight: 600, color: "#f8fafc" }}>{entry.name}</span>
                  <span style={{ flexShrink: 0, fontSize: 9, letterSpacing: "0.08em", color: "#818cf8" }}>
                    #{entry.insertion_order}
                  </span>
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 7 }}>
                  {entry.keys.map((key) => (
                    <span
                      key={key}
                      style={{
                        fontSize: 9,
                        color: "#c4b5fd",
                        background: "rgba(129, 140, 248, 0.12)",
                        borderRadius: 5,
                        padding: "2px 5px",
                      }}
                    >
                      {key}
                    </span>
                  ))}
                </div>
                <p style={{ margin: "8px 0 0", fontSize: 11, lineHeight: 1.6, color: "#cbd5e1" }}>{entry.content}</p>
                <div style={{ marginTop: 8, fontSize: 9, letterSpacing: "0.08em", textTransform: "uppercase", color: "#64748b" }}>
                  {entry.position === "before_char" ? "before character" : "after character"} · {entry.enabled ? "enabled" : "disabled"}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <footer style={{ marginTop: 22, display: "flex", justifyContent: "space-between", gap: 20, fontSize: 11, color: "#64748b" }}>
        <span>Rendered from characters.json and miyora.yorune.card.json through rig.ts</span>
        <span>Validated by scripts/validate_character_cards.py</span>
      </footer>
    </div>
  );
}

createRoot(document.getElementById("root") as HTMLElement).render(portrait ? <YorunePortrait /> : <YoruneDossier />);
