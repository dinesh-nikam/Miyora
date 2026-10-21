// TypeScript mirror of the Rust `agentpet-avatar` / `agentpet-core` avatar
// contracts. Rust validates every spec; the UI only renders and edits them.

export const BODY_TYPES = ["humanoid", "beast", "robot", "slime", "bird"] as const;
export const EAR_TYPES = ["none", "human", "elf", "cat", "fox", "bunny", "bear", "floppy", "horns", "antenna", "fin", "leaf"] as const;
export const TAIL_TYPES = ["none", "cat", "fox", "bunny", "dragon", "cable", "feather", "flame", "leaf", "star"] as const;
export const WING_TYPES = ["none", "feather", "bat", "fairy", "jet"] as const;
export const HAIR_STYLES = ["none", "short", "bob", "long", "ponytail", "twintails", "spiky", "bun", "mohawk", "curly", "hime", "side_tail"] as const;
export const EYE_STYLES = ["round", "sharp", "sleepy", "sparkle", "visor", "mono"] as const;
export const OUTFITS = ["hoodie", "kimono", "armor", "suit", "robe", "jacket", "dress", "overalls", "labcoat", "tunic", "fur", "plating", "hakama", "sailor", "coat"] as const;
export const PATTERNS = ["none", "dots", "stripes", "waves", "petals", "stars", "diamonds"] as const;
export const ACCESSORIES = [
  "glasses",
  "headphones",
  "scarf",
  "beanie",
  "witch_hat",
  "crown",
  "halo",
  "cape",
  "backpack",
  "headband",
  "bow",
  "flower",
  "goggles",
  "bell",
  "katana",
  "wand",
  "staff",
  "kitsune_mask",
  "shell",
  "gem",
] as const;
export const PALETTE_KEYS = ["skin", "hair", "eyes", "primary", "secondary", "accent"] as const;

export type BodyType = (typeof BODY_TYPES)[number];
export type Ears = (typeof EAR_TYPES)[number];
export type Tail = (typeof TAIL_TYPES)[number];
export type Wings = (typeof WING_TYPES)[number];
export type HairStyle = (typeof HAIR_STYLES)[number];
export type EyeStyle = (typeof EYE_STYLES)[number];
export type Outfit = (typeof OUTFITS)[number];
export type Accessory = (typeof ACCESSORIES)[number];
export type Pattern = (typeof PATTERNS)[number];
export type PaletteKey = (typeof PALETTE_KEYS)[number];

/** Limits mirrored from `crates/avatar/src/spec.rs`. */
export const SPEC_LIMITS = {
  maxAccessories: 5,
  maxNameChars: 32,
  height: [0.8, 1.2],
  headScale: [0.85, 1.35],
  build: [0.8, 1.3],
  limbLength: [0.85, 1.15],
  energy: [0.5, 1.6],
} as const;

export type Palette = Record<PaletteKey, string>;

export interface Proportions {
  height: number;
  headScale: number;
  build: number;
  limbLength: number;
}

export interface CharacterSpec {
  schemaVersion: number;
  id: string;
  name: string;
  title: string;
  description: string;
  body: BodyType;
  ears: Ears;
  tail: Tail;
  wings: Wings;
  hair: HairStyle;
  eyes: EyeStyle;
  outfit: Outfit;
  accessories: Accessory[];
  /** Original fabric pattern; older specs omit it (treated as "none"). */
  pattern?: Pattern;
  palette: Palette;
  /** Optional color the hair fades to at the tips. */
  hairTip?: string;
  proportions: Proportions;
  energy: number;
  tags: string[];
}

export type AvatarSource = "catalog" | "custom";
export type AvatarKind = "procedural" | "image_sculpt" | "vrm";

export interface ResolvedAvatar {
  source: AvatarSource;
  avatarId: string;
  kind: AvatarKind;
  name: string;
  spec: CharacterSpec | null;
  customized: boolean;
  modelSha256: string | null;
  metadata: Record<string, unknown> | null;
}

export interface AvatarCatalogEntry {
  spec: CharacterSpec;
  customized: boolean;
}

export interface CustomAvatarSummary {
  avatarId: string;
  kind: AvatarKind;
  name: string;
  spec: CharacterSpec | null;
  modelSha256: string | null;
  triangleCount: number | null;
  metadata: Record<string, unknown> | null;
  sourceAvatarId: string | null;
  createdAt: string;
}

/** Real progression data the stage visualizes. */
export interface AvatarHud {
  level: number;
  totalXp: number;
  xpIntoLevel: number;
  xpToNextLevel: number;
}

/** What the stage renders: a procedural spec or a stored model file. */
export type StageSubject =
  | { kind: "procedural"; spec: CharacterSpec }
  | { kind: "image_sculpt" | "vrm"; avatarId: string; modelSha256: string; name: string };

export function subjectFromAvatar(avatar: ResolvedAvatar | null): StageSubject | null {
  if (!avatar) return null;
  if (avatar.kind === "procedural") {
    return avatar.spec ? { kind: "procedural", spec: avatar.spec } : null;
  }
  return avatar.modelSha256
    ? { kind: avatar.kind, avatarId: avatar.avatarId, modelSha256: avatar.modelSha256, name: avatar.name }
    : null;
}

export function subjectKey(subject: StageSubject | null): string {
  if (!subject) return "none";
  return subject.kind === "procedural" ? `spec:${JSON.stringify(subject.spec)}` : `model:${subject.modelSha256}`;
}
