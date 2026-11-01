import { useCallback, useEffect, useRef, useState } from "react";
import { AvatarStage } from "../avatar/AvatarStage";
import { subjectFromAvatar, type AvatarHud, type PetPresentation, type ResolvedAvatar } from "../avatar/types";
import {
  currentCompanionPreferences,
  saveCompanionPreferences,
  useCompanionPreferences,
  type CompanionVibe,
} from "../settings/companionPreferences";

export type PetState =
  | "idle"
  | "working"
  | "thinking"
  | "waiting"
  | "success"
  | "error"
  | "celebrating"
  | "sleeping"
  | "hungry"
  | "eating"
  | "level_up"
  | "achievement"
  | "offline"
  | "paused"
  | "attention_required";

export interface SpriteFrame {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SelectedPetAnimation {
  petName: string;
  animationId: string;
  fps: number;
  loopAnimation: boolean;
  frames: SpriteFrame[];
  imageDataUrl: string;
}

export interface QuotaWindow {
  usedPercentage?: number;
  resetsAt?: string;
}

export interface CompanionRendererProps {
  state: PetState;
  animation: SelectedPetAnimation | null;
  /** 3D avatar resolved by the Rust core; rendered in 3D mode. */
  avatar?: ResolvedAvatar | null;
  /** Real progression shown on the 3D stage. */
  hud?: AvatarHud | null;
  quotaWindow?: QuotaWindow | null;
  quotaProvenance?: string;
  size?: number;
  interactive?: boolean;
  /** Pet only, for the desktop overlay: no speech, popovers, reminders, or controls. */
  bare?: boolean;
  /** Rust-resolved animation clip for the 3D stage. */
  presentation?: PetPresentation | null;
  mode?: "3d" | "2d";
  onModeToggle?: (mode: "3d" | "2d") => void;
  onDoubleClick?: () => void;
  onNextPet?: () => void;
  onWakePet?: () => void;
  onError?: (err: string) => void;
}

interface FloatingParticle {
  id: number;
  x: number;
  y: number;
  symbol: string;
  isTreat?: boolean;
}

export type ChimeType = "pet" | "success" | "bubble" | "care" | "eat" | "wake";

export class AudioManager {
  private static instance: AudioManager | null = null;
  private ctx: AudioContext | null = null;

  public static getInstance(): AudioManager {
    if (!AudioManager.instance) {
      AudioManager.instance = new AudioManager();
    }
    return AudioManager.instance;
  }

  public getContext(): AudioContext | null {
    if (typeof window === "undefined") return null;
    if (!this.ctx) {
      const AudioContextClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (AudioContextClass) {
        this.ctx = new AudioContextClass();
      }
    }
    if (this.ctx && this.ctx.state === "suspended") {
      this.ctx.resume().catch(() => {});
    }
    return this.ctx;
  }

  public playChime(type: ChimeType): void {
    try {
      if (currentCompanionPreferences().soundMuted) return;

      const ctx = this.getContext();
      if (!ctx) return;

      if (type === "pet") {
        // Warm major arpeggio
        const notes = [523.25, 659.25, 783.99]; // C5, E5, G5
        notes.forEach((freq, idx) => {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          osc.type = "sine";
          osc.frequency.setValueAtTime(freq, ctx.currentTime + idx * 0.07);
          gain.gain.setValueAtTime(0.08, ctx.currentTime + idx * 0.07);
          gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + idx * 0.07 + 0.28);
          osc.connect(gain);
          gain.connect(ctx.destination);
          osc.start(ctx.currentTime + idx * 0.07);
          osc.stop(ctx.currentTime + idx * 0.07 + 0.3);
        });
      } else if (type === "eat") {
        // Satisfying munch chime
        const freqs = [350, 480, 600];
        freqs.forEach((freq, idx) => {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          osc.type = "triangle";
          osc.frequency.setValueAtTime(freq, ctx.currentTime + idx * 0.08);
          gain.gain.setValueAtTime(0.12, ctx.currentTime + idx * 0.08);
          gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + idx * 0.08 + 0.16);
          osc.connect(gain);
          gain.connect(ctx.destination);
          osc.start(ctx.currentTime + idx * 0.08);
          osc.stop(ctx.currentTime + idx * 0.08 + 0.18);
        });
      } else if (type === "wake") {
        // Bright wake-up fanfare
        const notes = [440, 554.37, 659.25, 880]; // A4, C#5, E5, A5
        notes.forEach((freq, idx) => {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          osc.type = "sine";
          osc.frequency.setValueAtTime(freq, ctx.currentTime + idx * 0.08);
          gain.gain.setValueAtTime(0.1, ctx.currentTime + idx * 0.08);
          gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + idx * 0.08 + 0.35);
          osc.connect(gain);
          gain.connect(ctx.destination);
          osc.start(ctx.currentTime + idx * 0.08);
          osc.stop(ctx.currentTime + idx * 0.08 + 0.38);
        });
      } else if (type === "success") {
        // Victorious sparkling sweep
        const notes = [659.25, 783.99, 987.77, 1318.51]; // E5, G5, B5, E6
        notes.forEach((freq, idx) => {
          const osc = ctx.createOscillator();
          const gain = ctx.createGain();
          osc.type = "triangle";
          osc.frequency.setValueAtTime(freq, ctx.currentTime + idx * 0.09);
          gain.gain.setValueAtTime(0.1, ctx.currentTime + idx * 0.09);
          gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + idx * 0.09 + 0.4);
          osc.connect(gain);
          gain.connect(ctx.destination);
          osc.start(ctx.currentTime + idx * 0.09);
          osc.stop(ctx.currentTime + idx * 0.09 + 0.42);
        });
      } else if (type === "care") {
        // Gentle reminder chime
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = "sine";
        osc.frequency.setValueAtTime(440, ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(554.37, ctx.currentTime + 0.15);
        gain.gain.setValueAtTime(0.08, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.45);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start();
        osc.stop(ctx.currentTime + 0.48);
      } else if (type === "bubble") {
        // Soft pop
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = "sine";
        osc.frequency.setValueAtTime(850, ctx.currentTime);
        osc.frequency.exponentialRampToValueAtTime(320, ctx.currentTime + 0.05);
        gain.gain.setValueAtTime(0.05, ctx.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.06);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start();
        osc.stop(ctx.currentTime + 0.07);
      }
    } catch {
      // Audio context may be restricted before initial gesture
    }
  }

  public dispose(): void {
    if (this.ctx && this.ctx.state !== "closed") {
      this.ctx.close().catch(() => {});
      this.ctx = null;
    }
  }

  public static resetInstance(): void {
    if (AudioManager.instance) {
      AudioManager.instance.dispose();
      AudioManager.instance = null;
    }
  }
}

// Client-side Web Audio API Sound Synthesizer (delegates to singleton)
export function playChime(type: ChimeType) {
  AudioManager.getInstance().playChime(type);
}

export function CompanionRenderer({
  state,
  animation,
  avatar = null,
  hud = null,
  size = 200,
  interactive = true,
  bare = false,
  presentation = null,
  mode,
  onModeToggle,
  onDoubleClick,
  onNextPet,
  onWakePet,
  onError,
}: CompanionRendererProps) {
  const preferences = useCompanionPreferences();
  const renderMode = mode ?? preferences.renderMode;

  useEffect(() => {
    return () => {
      AudioManager.getInstance().dispose();
    };
  }, []);

  const handleToggleRenderMode = (e: React.MouseEvent) => {
    e.stopPropagation();
    const nextMode = renderMode === "3d" ? "2d" : "3d";
    // A parent that controls `mode` owns persistence; otherwise save here.
    if (onModeToggle) {
      onModeToggle(nextMode);
    } else {
      void saveCompanionPreferences({ renderMode: nextMode });
    }
  };

  const [frameIndex, setFrameIndex] = useState(0);
  const [imageLoaded, setImageLoaded] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Human Touch & Cuteness States
  const [particles, setParticles] = useState<FloatingParticle[]>([]);
  const [isSquishing, setIsSquishing] = useState(false);
  const [isEating, setIsEating] = useState(false);
  const [isBlushing, setIsBlushing] = useState(false);
  const [activeSpeech, setActiveSpeech] = useState<string>("");
  const [speechVisible, setSpeechVisible] = useState(true);
  // Session-only display value: never persisted and never ledger XP.
  const [bondXp, setBondXp] = useState(15);
  const affection = preferences.affection;
  const [wellnessAlert, setWellnessAlert] = useState<string | null>(null);

  // User In-Place Customizations (persisted by Rust)
  const customNickname = preferences.nickname;
  const customAccessory = preferences.accessory;
  const customAura = preferences.aura;
  const customVibe = preferences.vibe;
  const [isCustomizerOpen, setIsCustomizerOpen] = useState(false);
  const [pointerOffset, setPointerOffset] = useState({ x: 0, y: 0 });
  const stageBoxRef = useRef<HTMLDivElement | null>(null);

  const awardBondXp = (amount: number) => {
    setBondXp((prev) => prev + amount);
  };

  const raiseAffection = (amount: number) => {
    void saveCompanionPreferences({ affection: Math.min(100, currentCompanionPreferences().affection + amount) });
  };

  const spec = avatar?.spec ?? null;
  const petName = avatar?.name ?? spec?.name ?? animation?.petName ?? "Companion";
  const nameLower = petName.toLowerCase();
  const specCatchphrase = spec?.catchphrase;
  const specSpecialty = spec?.specialty;
  const specPersonality = spec?.personality;

  const isWukong = nameLower.includes("wukong") || nameLower.includes("monkey");
  const isNezuko = nameLower.includes("nezuko");
  const isArjuna = nameLower.includes("arjuna");
  const isAoi = nameLower.includes("aoi");
  const isNinjaCat = nameLower.includes("ninja-cat") || (nameLower.includes("ninja") && nameLower.includes("cat"));
  const isGojo = nameLower.includes("gojo");
  const isZoro = nameLower.includes("zoro");
  const isLuffy = nameLower.includes("luffy");
  const isTanjiro = nameLower.includes("tanjiro");
  const isSaitama = nameLower.includes("saitama");
  const isFrieren = nameLower.includes("frieren");
  const isAnya = nameLower.includes("anya");
  const isDenji = nameLower.includes("denji");
  const isPower = nameLower.includes("power");
  const isPikachu = nameLower.includes("pikachu");
  const isKrishna = nameLower.includes("krishna");
  const isHanuman = nameLower.includes("hanuman");
  const isShiva = nameLower.includes("shiva");
  const isKenji = nameLower.includes("kenji");
  const isMira = nameLower.includes("mira");
  const isBolt = nameLower.includes("bolt") || nameLower.includes("seven") || spec?.body === "robot";

  // Character metadata & favorites
  const companionInfo = isWukong
    ? { icon: "☁️", themeColor: "#ffb703", glow: "radial-gradient(circle, #ffb703 0%, rgba(230, 57, 70, 0.45) 60%, transparent 80%)", treatName: "Peach", treatEmoji: "🍑" }
    : isNezuko
    ? { icon: "🌸", themeColor: "#ff69b4", glow: "radial-gradient(circle, #ff6b8b 0%, rgba(255, 105, 180, 0.45) 60%, transparent 80%)", treatName: "Tea", treatEmoji: "🍵" }
    : isGojo
    ? { icon: "♾️", themeColor: "#38bdf8", glow: "radial-gradient(circle, #38bdf8 0%, rgba(56, 189, 248, 0.45) 60%, transparent 80%)", treatName: "Kikufuku", treatEmoji: "🍡" }
    : isZoro
    ? { icon: "⚔️", themeColor: "#22c55e", glow: "radial-gradient(circle, #22c55e 0%, rgba(34, 197, 94, 0.45) 60%, transparent 80%)", treatName: "Onigiri", treatEmoji: "🍙" }
    : isLuffy
    ? { icon: "🍖", themeColor: "#ef4444", glow: "radial-gradient(circle, #ef4444 0%, rgba(239, 68, 68, 0.45) 60%, transparent 80%)", treatName: "Meat", treatEmoji: "🍖" }
    : isTanjiro
    ? { icon: "🌊", themeColor: "#10b981", glow: "radial-gradient(circle, #10b981 0%, rgba(16, 185, 129, 0.45) 60%, transparent 80%)", treatName: "Rice Ball", treatEmoji: "🍙" }
    : isSaitama
    ? { icon: "🥊", themeColor: "#eab308", glow: "radial-gradient(circle, #eab308 0%, rgba(234, 179, 8, 0.45) 60%, transparent 80%)", treatName: "Udon", treatEmoji: "🍜" }
    : isFrieren
    ? { icon: "🪄", themeColor: "#a7f3d0", glow: "radial-gradient(circle, #a7f3d0 0%, rgba(167, 243, 208, 0.45) 60%, transparent 80%)", treatName: "Grimoire", treatEmoji: "📖" }
    : isAnya
    ? { icon: "🥜", themeColor: "#f472b6", glow: "radial-gradient(circle, #f472b6 0%, rgba(244, 114, 182, 0.45) 60%, transparent 80%)", treatName: "Peanuts", treatEmoji: "🥜" }
    : isDenji
    ? { icon: "🪚", themeColor: "#f59e0b", glow: "radial-gradient(circle, #f59e0b 0%, rgba(245, 158, 11, 0.45) 60%, transparent 80%)", treatName: "Toast", treatEmoji: "🍞" }
    : isPower
    ? { icon: "😈", themeColor: "#f43f5e", glow: "radial-gradient(circle, #f43f5e 0%, rgba(244, 63, 94, 0.45) 60%, transparent 80%)", treatName: "Blood", treatEmoji: "🩸" }
    : isPikachu
    ? { icon: "⚡", themeColor: "#facc15", glow: "radial-gradient(circle, #facc15 0%, rgba(250, 204, 21, 0.45) 60%, transparent 80%)", treatName: "Berry", treatEmoji: "🍓" }
    : isKrishna
    ? { icon: "🪈", themeColor: "#38bdf8", glow: "radial-gradient(circle, #38bdf8 0%, rgba(255, 215, 0, 0.45) 60%, transparent 80%)", treatName: "Makhan", treatEmoji: "🍯" }
    : isHanuman
    ? { icon: "🔥", themeColor: "#ea580c", glow: "radial-gradient(circle, #ea580c 0%, rgba(234, 88, 12, 0.45) 60%, transparent 80%)", treatName: "Fruit", treatEmoji: "🥭" }
    : isShiva
    ? { icon: "🔱", themeColor: "#0284c7", glow: "radial-gradient(circle, #0284c7 0%, rgba(2, 132, 199, 0.45) 60%, transparent 80%)", treatName: "Belpatra", treatEmoji: "🌿" }
    : isArjuna
    ? { icon: "🏹", themeColor: "#ff9933", glow: "radial-gradient(circle, #ff9933 0%, rgba(255, 215, 0, 0.45) 60%, transparent 80%)", treatName: "Honey", treatEmoji: "🍯" }
    : isNinjaCat
    ? { icon: "🐾", themeColor: "#00f0ff", glow: "radial-gradient(circle, #00f0ff 0%, rgba(13, 148, 136, 0.45) 60%, transparent 80%)", treatName: "Fish", treatEmoji: "🐟" }
    : isAoi
    ? { icon: "🎧", themeColor: "#38bdf8", glow: "radial-gradient(circle, #38bdf8 0%, rgba(14, 165, 233, 0.45) 60%, transparent 80%)", treatName: "Coffee", treatEmoji: "☕" }
    : isKenji
    ? { icon: "👓", themeColor: "#818cf8", glow: "radial-gradient(circle, #818cf8 0%, rgba(99, 102, 241, 0.45) 60%, transparent 80%)", treatName: "Espresso", treatEmoji: "☕" }
    : isMira
    ? { icon: "✨", themeColor: "#c084fc", glow: "radial-gradient(circle, #c084fc 0%, rgba(192, 132, 252, 0.45) 60%, transparent 80%)", treatName: "Starcandy", treatEmoji: "🍬" }
    : isBolt
    ? { icon: "🤖", themeColor: "#06b6d4", glow: "radial-gradient(circle, #06b6d4 0%, rgba(6, 182, 212, 0.45) 60%, transparent 80%)", treatName: "Battery", treatEmoji: "🔋" }
    : spec?.body === "beast"
    ? { icon: "🐾", themeColor: spec?.palette.accent ?? "#f59e0b", glow: `radial-gradient(circle, ${spec?.palette.accent ?? "#f59e0b"} 0%, rgba(245, 158, 11, 0.45) 60%, transparent 80%)`, treatName: "Treat", treatEmoji: "🦴" }
    : spec?.body === "slime"
    ? { icon: "💧", themeColor: spec?.palette.accent ?? "#34d399", glow: `radial-gradient(circle, ${spec?.palette.accent ?? "#34d399"} 0%, rgba(52, 211, 153, 0.45) 60%, transparent 80%)`, treatName: "Syrup", treatEmoji: "🍯" }
    : {
        icon: "✨",
        themeColor: spec?.palette.accent ?? "#a855f7",
        glow: `radial-gradient(circle, ${spec?.palette.accent ?? "#a855f7"} 0%, rgba(168, 85, 247, 0.45) 60%, transparent 80%)`,
        treatName: "Cupcake",
        treatEmoji: "🧁",
      };

  // Bond Level
  const bondLevel = Math.max(1, Math.floor(bondXp / 15) + 1);
  const bondTitle =
    bondLevel >= 10
      ? "Legendary Partner"
      : bondLevel >= 6
      ? "Devoted Companion"
      : bondLevel >= 3
      ? "Close Friend"
      : "Coding Buddy";

  const displayName = customNickname.trim() || (renderMode === "3d" && avatar ? avatar.name : petName);
  const activeThemeColor = customAura || companionInfo.themeColor;
  const activeGlow = customAura
    ? `radial-gradient(circle, ${customAura} 0%, rgba(255, 105, 180, 0.45) 60%, transparent 80%)`
    : companionInfo.glow;

  // Authentic Human & Character Dialogue
  const getSpeechForState = useCallback(
    (currentState: PetState, trigger: "auto" | "pet" | "feed" | "wake"): string => {
      // 1. Feeding Response
      if (trigger === "feed") {
        if (isWukong) return "Mmm! Celestial peach! Power level rising, Chief! 🍑✨";
        if (isNezuko) return "Mmm-ph! (Nezuko sips warm tea happily!) (◕‿◕)🍵";
        if (isArjuna) return "Saffron nectar! My archery focus is refreshed. 🍯🏹";
        if (isNinjaCat) return "*crunch crunch* Purrrrr! Delicious fish! 🐟🐾";
        if (isGojo) return "Sweet Kikufuku mochi! Cursed energy at maximum. 🍡✨";
        if (isZoro) return "Onigiri consumed. Swords are sharpened. 🍙⚔️";
        if (isLuffy) return "MEEEAT! Hahaha, let's ship the whole feature! 🍖🎉";
        if (isAnya) return "Peanuts! Anya's mind-reading powers boosted! 🥜✨";
        if (isFrieren) return "A peaceful sweet before diving into old grimoires. 📖☕";
        if (isPikachu) return "Pika-pi! (Cheeks charged with clean energy!) 🍓⚡";
        if (isKrishna) return "Sacred makhan! Divine harmony flows through your code. 🍯🪈";
        if (spec) return `Mmm, ${companionInfo.treatName}! ${petName} is energized for the next sprint, Chief! ⚡`;
        return "Mmm, warm coffee! Ready for the next sprint, Chief! ☕⚡";
      }

      // 2. Wake-up Response
      if (trigger === "wake") {
        if (isWukong) return "Hahaha! The Great Sage awakens! Point me to the bugs! ⚡☁️";
        if (isNezuko) return "Mmm-ph! (Rubs her eyes and smiles warmly!) 🌸✨";
        if (isArjuna) return "Awake and vigilant. The Gandiva bow stands ready! 🏹";
        if (isNinjaCat) return "*stretches stealthily* Meow! Ready for deployment! 🐾";
        if (isGojo) return "Back from the void. Ready to show them what real code looks like. ♾️";
        if (isZoro) return "Yawn... Which file needs cutting down, Chief? ⚔️";
        if (isAnya) return "Anya woke up! Is it time to test? (◕‿◕)🥜";
        if (isFrieren) return "Awake. Another century, another stable release. 🪄";
        if (specCatchphrase) return `${petName} awakens: "${specCatchphrase}" ⚡`;
        return "Systems online! Let's conquer this codebase together! ⚡";
      }

      // 3. Petting / Affection Response
      if (trigger === "pet") {
        if (isWukong) return "Hahaha! The Monkey King is energized! Celestial blessing for your keyboard! 🍑✨";
        if (isNezuko) return "Mmm-ph! (Nezuko smiles warmly and leans into your hand) 🌸💖";
        if (isArjuna) return "Honor and gratitude, Chief. Together our aim is true! 🏹✨";
        if (isNinjaCat) return "*loud purring* Purrrrr... blessed by the shinobi cat! 🐾💖";
        if (isGojo) return "Infinity won't stop a good high-five from you, Chief! ♾️✨";
        if (isZoro) return "Hmph... don't slack off. Let's get these commits in. ⚔️";
        if (isAnya) return "Waku waku! Chief gave Anya headpats! (◕‿◕)♡";
        if (isFrieren) return "Headpats? Himmel used to do that. It is pleasant. 🪄";
        if (isPikachu) return "Pikaaa-chuuu! (Warm happy sparks!) ⚡💖";
        if (spec) return `${petName} (${specPersonality ?? "happy"}): "Thanks for the support, Chief! Let's build!" ✨`;
        return "Aw, thanks for the pats, Chief! We're an unbeatable coding duo! 🎧💖";
      }

      // 4. Automatic Contextual State Dialogue
      if (isWukong) {
        switch (currentState) {
          case "working": return "My Gold-Banded Cudgel will crush these merge conflicts! ⚡";
          case "thinking": return "Pondering the Dao of clean code... 🤔";
          case "success": return "HA! Another feature vanquished! The heavens rejoice! 🎉";
          case "error": return "A minor bug demon! Stand back, Chief, we'll smash it! 💪";
          case "sleeping": return "Zzz... napping on my Somersault Cloud... ☁️💤";
          default: return "Riding my Somersault Cloud. Ready when you are, Chief! ☁️";
        }
      }

      if (isNezuko) {
        switch (currentState) {
          case "working": return "Mmm! (Guarding your focus while you code!) ⚡🌸";
          case "thinking": return "Mmm? (Tilts head curiously at the algorithm) 🤔";
          case "success": return "Mmm-ph!! (Jumps with joy!) All tests passed! 🎉";
          case "error": return "Mmm... (Hands you a lucky charm. We'll fix it!) 💪";
          case "sleeping": return "Zzz... resting peacefully in her cozy box... 🌙💤";
          default: return "Keeping watch by your side. Hydrate when ready, Chief! 🍵";
        }
      }

      if (isGojo) {
        switch (currentState) {
          case "working": return "Unlimited Void deployed. Compiling at the speed of light. ♾️⚡";
          case "thinking": return "Six Eyes analyzing every branch and memory allocation... 🤔";
          case "success": return "Domain Expansion: Clean Architecture! Zero errors. 🚀";
          case "error": return "A cursed exception appeared. I'll exorcise it in one pass. 🔍";
          case "sleeping": return "Reversing cursed energy... even the strongest needs rest. 🌙";
          default: return "Don't worry, Chief. I'm the strongest companion you've got. ♾️✨";
        }
      }

      if (isZoro) {
        switch (currentState) {
          case "working": return "Three-sword syntax check. Slicing through technical debt! ⚔️";
          case "thinking": return "I'm not lost... just inspecting an unexplored submodule. 🤔";
          case "success": return "Nothing happened... just three clean commits. 🗡️🎉";
          case "error": return "A stubborn blocker. We cut right through it again! 💪";
          case "sleeping": return "Zzz... sleeping on deck with a hand on the hilt... 🌙";
          default: return "Swords sharpened. Give me the next pull request, Chief. ⚔️";
        }
      }

      if (isTanjiro) {
        switch (currentState) {
          case "working": return "Total Concentration Breathing! Following the thread to clean code! 🌊⚡";
          case "thinking": return "I can smell the seam in this logic bug... almost there! 🤔";
          case "success": return "Sun Breathing: Clear Blue Sky! All test suites green! ☀️🎉";
          case "error": return "Never give up! We'll keep refining until the tests pass! 💪";
          case "sleeping": return "Zzz... resting by the mountain hearth... 🌙";
          default: return "Swordsman of the Sun at your service, Chief! ☀️";
        }
      }

      if (isSaitama) {
        switch (currentState) {
          case "working": return "Consecutive Normal Edits... getting it done quickly. 🥊";
          case "thinking": return "Is there a supermarket sale today? Oh right, the bug. 🤔";
          case "success": return "One punch. One test. All green. Let's get groceries. 🏆";
          case "error": return "Hmm. An unhandled exception. Looks weak. 🔍";
          case "sleeping": return "Zzz... dozing on the tatami mat... 🌙";
          default: return "Just a companion for fun. What needs fixing, Chief? 🥊";
        }
      }

      if (isFrieren) {
        switch (currentState) {
          case "working": return "Transmuting ancient modules into elegant modern functions... 🪄";
          case "thinking": return "Pondering the spell for automatic memory deallocation... 🤔";
          case "success": return "Zoltraak pierced the bug cleanly. 100 years of stability assured. ✨";
          case "error": return "A mimic chest? No, just an unhandled Promise rejection. 🔍";
          case "sleeping": return "Zzz... wake me up in thirty years... or when the build finishes. 🌙";
          default: return "Take your time, Chief. Good code, like journeying, cannot be rushed. 📖";
        }
      }

      if (isAnya) {
        switch (currentState) {
          case "working": return "Anya is doing super secret spy coding! 🥜⚡";
          case "thinking": return "Anya reads your mind: you want clean code! 🤔✨";
          case "success": return "WAKU WAKU!! All green! Give Anya a Stella star! ⭐🎉";
          case "error": return "SHOCK! The compiler made an angry face! 😱";
          case "sleeping": return "Zzz... dreaming of giant peanuts... 🌙🥜";
          default: return "Anya is ready! Waku waku codebase time! (◕‿◕)✨";
        }
      }

      if (isKrishna) {
        switch (currentState) {
          case "working": return "Performing code karma with focus and devotion. 🪈⚡";
          case "thinking": return "The cosmic flute guides the flow of every abstraction... 🤔";
          case "success": return "Victory without attachment! The architecture sings in harmony! 🪈🎉";
          case "error": return "Every obstacle is an invitation to deeper understanding, Chief. 🧘";
          case "sleeping": return "Resting in cosmic stillness beneath the kadamba tree... 🌙";
          default: return "Act with steadfast purpose, Chief. The code will flourish. 🪈✨";
        }
      }

      if (isArjuna) {
        switch (currentState) {
          case "working": return "Arrows of logic flying true to the target! ⚡🏹";
          case "thinking": return "Surveying the battlefield of code... finding the clean route. 🤔";
          case "success": return "Bullseye! Every test struck with precision! 🎯";
          case "error": return "Adjusting stance! We'll strike the root cause together! 🛡️";
          case "sleeping": return "Zzz... resting upon his chariot beneath the stars... 🌙";
          default: return "Gandiva bow strung. Focus unbroken, Chief. 🏹";
        }
      }

      if (isNinjaCat) {
        switch (currentState) {
          case "working": return "*silent paws typing at 200 WPM* 🐾⚡";
          case "thinking": return "*ears twitching* Sensing an async race condition... 🤔";
          case "success": return "Target silenced! Build green and stealthy! 🐾🎉";
          case "error": return "Hiss! Bug detected in perimeter! Ambushing... ⚔️";
          case "sleeping": return "Zzz... curled up in a neat little cat ball... 🌙🐾";
          default: return "*purrs* All systems quiet and secure, Chief. 🐾";
        }
      }

      // Catalog procedural spec with bespoke catchphrase & specialty
      if (specCatchphrase) {
        switch (currentState) {
          case "working":
            return specSpecialty
              ? `${petName} deploying ${specSpecialty}: "${specCatchphrase}" ⚡`
              : `${petName}: "${specCatchphrase}" ⚡`;
          case "thinking":
            return `${petName} (${specPersonality ?? "focused"} focus) analyzing the architecture... 🤔`;
          case "success":
            return `${petName}: "Verification complete! ${specCatchphrase}" 🚀🎉`;
          case "error":
            return `${petName}: "Compiler warning detected. Engaging ${specSpecialty ?? "triage"}." 🔍`;
          case "sleeping":
            return `Zzz... ${petName} is resting. Codebase is secure. 🌙`;
          case "waiting":
          case "attention_required":
            return `${petName} needs your input, Chief! 📋`;
          default:
            return `${petName}: "${specCatchphrase}" ✨`;
        }
      }

      // Default Aoi fallback
      switch (currentState) {
        case "working":
          return "In the zone! Writing clean, beautiful code! ⚡";
        case "thinking":
          return "Tracing the call graph... let's check edge cases. 🤔";
        case "success":
          return "Clean build, zero warnings! High five, Chief! 🚀";
        case "error":
          return "Compiler caught an issue — let's inspect line 1! 🔍";
        case "sleeping":
          return "Power-nap mode active... Zzz... ping me anytime! 🌙";
        case "waiting":
        case "attention_required":
          return "Input needed! Take your time, Chief. 📋";
        default:
          return "Hey Chief! Ready to build something awesome? 🎧✨";
      }
    },
    [
      isAnya,
      isAoi,
      isArjuna,
      isBolt,
      isDenji,
      isFrieren,
      isGojo,
      isKrishna,
      isLuffy,
      isNezuko,
      isNinjaCat,
      isPikachu,
      isSaitama,
      isTanjiro,
      isWukong,
      isZoro,
      petName,
      spec,
      specCatchphrase,
      specPersonality,
      specSpecialty,
    ]
  );

  // Sync speech on state changes
  useEffect(() => {
    const text = getSpeechForState(state, "auto");
    setActiveSpeech(text);
    if (state === "success" || state === "celebrating" || state === "level_up") {
      playChime("success");
    }
  }, [state, getSpeechForState]);

  // Periodic wellness check-ins
  useEffect(() => {
    if (bare) return;
    const timer = setInterval(() => {
      const reminders = [
        "💧 Water Break: Time for a refreshing sip, Chief!",
        "🧘 Posture Check: Roll your shoulders and take a breath.",
        "👁️ 20-20-20 Rule: Look 20ft away for 20s to rest your eyes.",
      ];
      const reminder = reminders[Math.floor(Math.random() * reminders.length)];
      setWellnessAlert(reminder);
      playChime("care");
    }, 180000);
    return () => clearInterval(timer);
  }, [bare]);

  // User Customization Persistence
  const saveNickname = (name: string) => {
    void saveCompanionPreferences({ nickname: name });
  };

  const saveAccessory = (acc: string) => {
    void saveCompanionPreferences({ accessory: acc });
    playChime("bubble");
  };

  const saveAura = (color: string) => {
    void saveCompanionPreferences({ aura: color });
    playChime("bubble");
  };

  const saveVibe = (vibe: CompanionVibe) => {
    void saveCompanionPreferences({ vibe });
    playChime("bubble");
  };

  // Living Gaze Tracking
  const handleStagePointerMove = (e: React.PointerEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const nx = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    const ny = ((e.clientY - rect.top) / rect.height) * 2 - 1;
    setPointerOffset({ x: nx, y: ny });
  };

  const handleStagePointerLeave = () => {
    setPointerOffset({ x: 0, y: 0 });
  };

  // Heart-to-Heart Human Touch Check-in
  const handleHeartToHeart = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    playChime("care");
    setIsBlushing(true);
    setTimeout(() => setIsBlushing(false), 2400);

    const warmSpeeches: Record<"sweet" | "playful" | "loyal" | "calm", string[]> = {
      sweet: [
        "Chief, remember to breathe deeply. I'm always cheering for you! 🌸",
        "Have a sip of water, okay? Your health comes first! 💧",
        "Even tough bugs dissolve with patience. You got this, friend! ✨",
        "Your posture looks a bit tense — drop your shoulders and relax! 🌿",
        "I'm right here beside you. You're never coding alone! 💕",
      ],
      playful: [
        "High-paw, Chief! We're crushing these bugs today! 🐾",
        "Whoa, super clean code! Can we celebrate with a snack? 🍰",
        "Refactoring time? Let's make it look magical! 🪄",
        "You're my absolute favorite developer in the world! 🚀",
        "Zooming through tasks! You're on fire today! ⚡",
      ],
      loyal: [
        "Standing guard over your workspace, Chief. Ready for anything! 🛡️",
        "Every error is just another clue. I trust your judgment completely! ⚡",
        "Focus level: maximum. I'm right here holding the line with you! 🦾",
        "We're making solid, reliable progress. Keep marching forward! 🌟",
      ],
      calm: [
        "Let your thoughts settle like clear water. The logic will flow. 🍵",
        "Step by step, line by line. Great craft takes gentle time. 🌿",
        "A peaceful mind writes the most elegant architecture. 🌸",
        "Take a breath of fresh air. Everything is under control. ✨",
      ],
    };

    const deck = warmSpeeches[customVibe] || warmSpeeches.sweet;
    const choice = deck[Math.floor(Math.random() * deck.length)];
    setActiveSpeech(choice);
    setSpeechVisible(true);
  };

  // Petting Action (Warm Touch & Heart Fountain)
  const handlePet = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setIsSquishing(true);
    setTimeout(() => setIsSquishing(false), 260);
    setIsBlushing(true);
    setTimeout(() => setIsBlushing(false), 2200);

    playChime("pet");

    // Spawn floating heart particle fountain
    const rect = stageBoxRef.current?.getBoundingClientRect();
    const cx = rect ? rect.width / 2 : size / 2;
    const cy = rect ? rect.height / 2 : size / 2;
    const symbols = ["💖", "✨", "🌸", "💕", "🐾", "⭐"];

    for (let i = 0; i < 3; i++) {
      const id = Date.now() + Math.random() + i;
      const x = cx + (Math.random() * 50 - 25);
      const y = cy + (Math.random() * 30 - 15);
      const symbol = symbols[Math.floor(Math.random() * symbols.length)];
      setTimeout(() => {
        setParticles((prev) => [...prev, { id, x, y, symbol }]);
        setTimeout(() => {
          setParticles((prev) => prev.filter((p) => p.id !== id));
        }, 1200);
      }, i * 90);
    }

    // Increase Bond & Affection
    awardBondXp(1);
    raiseAffection(3);

    const petLines = [
      "Aww, thank you! I love pets! (˶ᵔ ᵕ ᵔ˶)💕",
      "*Happy tail wiggles and purrs* 💖",
      "You're the sweetest, Chief! (◕‿◕✿)",
      "Yay! That tickles so good! ✨",
      "Hehe, that gives me +100 coding energy! 🌸",
    ];
    setActiveSpeech(petLines[Math.floor(Math.random() * petLines.length)]);
    setSpeechVisible(true);
  };

  // Feed Snack Action
  const handleFeed = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setIsEating(true);
    setTimeout(() => setIsEating(false), 480);
    setIsBlushing(true);
    setTimeout(() => setIsBlushing(false), 2200);

    playChime("eat");

    // Spawn treat drop particle
    const rect = stageBoxRef.current?.getBoundingClientRect();
    const cx = rect ? rect.width / 2 : size / 2;
    const cy = rect ? rect.height / 2 - 20 : size / 2 - 20;

    const id = Date.now() + Math.random();
    setParticles((prev) => [
      ...prev,
      { id, x: cx, y: cy, symbol: companionInfo.treatEmoji, isTreat: true },
    ]);
    setTimeout(() => {
      setParticles((prev) => prev.filter((p) => p.id !== id));
    }, 1200);

    // Increase Bond & Affection
    awardBondXp(2);
    raiseAffection(5);

    const feedLines = [
      `*Nom nom nom* Delicious ${companionInfo.treatName}! Thank you, Chief! 😋`,
      `So yummy! My heart and energy are happy! 🌟`,
      `Mmm, ${companionInfo.treatName} gives me super developer focus! 💖`,
    ];
    setActiveSpeech(feedLines[Math.floor(Math.random() * feedLines.length)]);
    setSpeechVisible(true);
  };

  // Wake Up Action
  const handleWake = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    playChime("wake");

    setIsSquishing(true);
    setTimeout(() => setIsSquishing(false), 240);

    const response = getSpeechForState(state, "wake");
    setActiveSpeech(response);
    setSpeechVisible(true);

    if (onWakePet) {
      onWakePet();
    }
  };

  // Manual Thought Cycle
  const handleCycleThought = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    playChime("bubble");
    const next = getSpeechForState(state, "pet");
    setActiveSpeech(next);
    setSpeechVisible(true);
  };

  // Load Image and reset frames
  useEffect(() => {
    setLoadError(null);
    setImageLoaded(false);
    imgRef.current = null;

    if (!animation?.imageDataUrl) {
      return;
    }
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.src = animation.imageDataUrl;
    img.onload = () => {
      imgRef.current = img;
      setImageLoaded(true);
      setLoadError(null);
    };
    img.onerror = () => {
      const err = `Failed to decode sprite asset for ${animation.petName}`;
      setLoadError(err);
      if (onError) onError(err);
    };
    return () => {
      img.onload = null;
      img.onerror = null;
    };
  }, [animation?.imageDataUrl, animation?.petName, onError]);

  // Frame timing loop
  useEffect(() => {
    if (!animation || animation.frames.length <= 1) {
      setFrameIndex(0);
      return;
    }
    const intervalMs = Math.max(16, Math.floor(1000 / (animation.fps || 1)));
    const timer = setInterval(() => {
      setFrameIndex((prev) => {
        if (prev + 1 >= animation.frames.length) {
          return animation.loopAnimation ? 0 : prev;
        }
        return prev + 1;
      });
    }, intervalMs);
    return () => clearInterval(timer);
  }, [animation]);

  // Canvas paint
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !imageLoaded || !imgRef.current || !animation) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.imageSmoothingEnabled = false;

    const currentFrame = animation.frames[frameIndex] || animation.frames[0];
    if (currentFrame) {
      ctx.drawImage(
        imgRef.current,
        currentFrame.x,
        currentFrame.y,
        currentFrame.width,
        currentFrame.height,
        0,
        0,
        canvas.width,
        canvas.height
      );
    }
  }, [animation, frameIndex, imageLoaded]);



  return (
    <div
      className={`companion-renderer-container ${interactive ? "interactive" : ""} state-${state}`}
      onDoubleClick={onDoubleClick}
      style={{
        position: "relative",
        width: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "flex-end",
        userSelect: "none",
        boxSizing: "border-box",
      }}
    >
      {/* 1. Sleek In-Bounds Floating Speech Bubble with Speech Tail (No square box!) */}
      {!bare && speechVisible && activeSpeech && (
        <div
          className="companion-speech-bubble"
          onClick={handleCycleThought}
          title="Click to hear another thought"
          style={{
            position: "relative",
            width: "calc(100% - 16px)",
            maxWidth: "248px",
            background: "rgba(15, 23, 42, 0.94)",
            border: `1px solid ${activeThemeColor}55`,
            borderRadius: "18px",
            padding: "7px 12px",
            color: "#f8fafc",
            fontSize: "10.5px",
            lineHeight: "1.38",
            fontWeight: 500,
            boxShadow: `0 6px 20px ${activeThemeColor}28`,
            backdropFilter: "blur(12px)",
            zIndex: 10,
            display: "flex",
            alignItems: "center",
            gap: "7px",
            marginBottom: "8px",
            animation: "bubbleFloatIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1)",
            cursor: "pointer",
          }}
        >
          <span style={{ fontSize: "13px", flexShrink: 0 }}>{companionInfo.icon}</span>
          <span style={{ flex: 1, letterSpacing: "0.2px" }}>{activeSpeech}</span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setSpeechVisible(false);
            }}
            title="Dismiss speech"
            style={{
              background: "rgba(255, 255, 255, 0.08)",
              border: "none",
              borderRadius: "50%",
              color: "#94a3b8",
              cursor: "pointer",
              fontSize: "10px",
              width: "16px",
              height: "16px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              flexShrink: 0,
            }}
          >
            ×
          </button>
          {/* Manga-style downward speech bubble tail */}
          <div
            style={{
              position: "absolute",
              bottom: "-6px",
              left: "50%",
              transform: "translateX(-50%)",
              width: 0,
              height: 0,
              borderLeft: "6px solid transparent",
              borderRight: "6px solid transparent",
              borderTop: `6px solid ${activeThemeColor}66`,
            }}
          />
        </div>
      )}

      {/* Wellness Human Care Popover */}
      {!bare && wellnessAlert && (
        <div
          onClick={(e) => {
            e.stopPropagation();
            setWellnessAlert(null);
            awardBondXp(5);
            playChime("pet");
          }}
          title="Click to acknowledge: +5 Bond XP!"
          style={{
            position: "absolute",
            top: "2px",
            left: "50%",
            transform: "translateX(-50%)",
            background: "linear-gradient(135deg, #10b981 0%, #059669 100%)",
            color: "#ffffff",
            padding: "5px 12px",
            borderRadius: "20px",
            fontSize: "9.5px",
            fontWeight: 700,
            letterSpacing: "0.3px",
            boxShadow: "0 6px 18px rgba(16, 185, 129, 0.4)",
            zIndex: 20,
            whiteSpace: "nowrap",
            cursor: "pointer",
            animation: "pulse 1.5s infinite ease-in-out",
          }}
        >
          {wellnessAlert} · <span style={{ textDecoration: "underline" }}>Done! (+5 XP)</span>
        </div>
      )}

      {/* 2. Living Companion Character Stage with Pointer Tilt & Organic Floating */}
      <div
        ref={stageBoxRef}
        onClick={interactive ? handlePet : undefined}
        onPointerMove={interactive ? handleStagePointerMove : undefined}
        onPointerLeave={interactive ? handleStagePointerLeave : undefined}
        aria-label={interactive ? "Companion stage — click to pet and bond" : undefined}
        role={interactive ? "button" : undefined}
        tabIndex={interactive ? 0 : -1}
        onKeyDown={(e) => { if (interactive && (e.key === 'Enter' || e.key === ' ')) handlePet(); }}
        style={{
          position: "relative",
          width: size,
          height: size,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: interactive ? "pointer" : "default",
          transform: interactive
            ? `perspective(600px) rotateY(${pointerOffset.x * 7}deg) rotateX(${-pointerOffset.y * 5}deg)`
            : undefined,
          transition: "transform 0.15s cubic-bezier(0.2, 0, 0.2, 1)",
        }}
      >
        {/* Dynamic Character Aura Glow */}
        <div
          className="companion-aura"
          style={{
            position: "absolute",
            inset: "15px",
            borderRadius: "50%",
            pointerEvents: "none",
            filter: "blur(20px)",
            opacity: state === "idle" ? 0.4 : state === "sleeping" ? 0.18 : 0.75,
            background: activeGlow,
            transition: "all 0.6s ease-in-out",
          }}
        />

        {/* Dynamic Ground Shadow */}
        <div
          style={{
            position: "absolute",
            bottom: "6px",
            width: `${size * 0.55}px`,
            height: "10px",
            borderRadius: "50%",
            background: "radial-gradient(ellipse at center, rgba(0, 0, 0, 0.65) 0%, transparent 75%)",
            pointerEvents: "none",
            animation: "companionShadow 3.8s ease-in-out infinite",
          }}
        />

        {/* Cheerful Blushing Cheeks Overlay */}
        {isBlushing && (
          <div
            style={{
              position: "absolute",
              top: "43%",
              left: "50%",
              transform: "translateX(-50%)",
              width: `${size * 0.48}px`,
              display: "flex",
              justifyContent: "space-between",
              pointerEvents: "none",
              zIndex: 7,
              animation: "blushPulse 1.8s ease-in-out infinite",
            }}
          >
            <div style={{ width: `${size * 0.1}px`, height: `${size * 0.05}px`, borderRadius: "50%", background: "radial-gradient(circle, rgba(255, 105, 180, 0.95) 0%, rgba(255, 182, 193, 0.4) 60%, transparent 100%)", filter: "blur(2px)" }} />
            <div style={{ width: `${size * 0.1}px`, height: `${size * 0.05}px`, borderRadius: "50%", background: "radial-gradient(circle, rgba(255, 105, 180, 0.95) 0%, rgba(255, 182, 193, 0.4) 60%, transparent 100%)", filter: "blur(2px)" }} />
          </div>
        )}

        {/* Cute Accessories Layer */}
        {customAccessory === "ribbon" && (
          <div style={{ position: "absolute", top: "14%", left: "50%", transform: "translateX(-50%)", fontSize: `${size * 0.16}px`, zIndex: 8, pointerEvents: "none", filter: "drop-shadow(0 2px 5px rgba(0,0,0,0.4))" }}>
            🎀
          </div>
        )}
        {customAccessory === "flower" && (
          <div style={{ position: "absolute", top: "15%", right: "26%", fontSize: `${size * 0.16}px`, zIndex: 8, pointerEvents: "none", filter: "drop-shadow(0 2px 5px rgba(0,0,0,0.4))" }}>
            🌸
          </div>
        )}
        {customAccessory === "crown" && (
          <div style={{ position: "absolute", top: "4%", left: "50%", transform: "translateX(-50%)", fontSize: `${size * 0.18}px`, zIndex: 8, pointerEvents: "none", animation: "crownFloat 2.4s ease-in-out infinite", filter: "drop-shadow(0 4px 10px rgba(250,204,21,0.6))" }}>
            👑
          </div>
        )}
        {customAccessory === "glasses" && (
          <div style={{ position: "absolute", top: "34%", left: "50%", transform: "translateX(-50%)", fontSize: `${size * 0.18}px`, zIndex: 8, pointerEvents: "none", filter: "drop-shadow(0 2px 6px rgba(0,0,0,0.5))" }}>
            👓
          </div>
        )}
        {customAccessory === "halo" && (
          <div
            style={{
              position: "absolute",
              top: "6%",
              left: "50%",
              width: `${size * 0.32}px`,
              height: `${size * 0.1}px`,
              borderRadius: "50%",
              border: "3px solid #fde047",
              zIndex: 8,
              pointerEvents: "none",
              animation: "haloGlow 2.5s ease-in-out infinite",
            }}
          />
        )}
        {customAccessory === "bell" && (
          <div style={{ position: "absolute", top: "55%", left: "50%", transform: "translateX(-50%)", fontSize: `${size * 0.14}px`, zIndex: 8, pointerEvents: "none", filter: "drop-shadow(0 2px 4px rgba(0,0,0,0.4))" }}>
            🔔
          </div>
        )}

        {/* Floating Particles (Hearts & Treats) */}
        {particles.map((p) => (
          <div
            key={p.id}
            style={{
              position: "absolute",
              left: p.x,
              top: p.y,
              fontSize: p.isTreat ? "24px" : "18px",
              pointerEvents: "none",
              zIndex: 30,
              animation: p.isTreat
                ? "dropTreat 1.1s cubic-bezier(0.2, 0.8, 0.2, 1) forwards"
                : "floatHeart 1.1s cubic-bezier(0.2, 0.8, 0.2, 1) forwards",
            }}
          >
            {p.symbol}
          </div>
        ))}

        {/* Main Character: 3D Articulated Avatar (Default) or 2D Pixel Sprite */}
        {renderMode === "3d" ? (
          <div
            style={{
              position: "relative",
              zIndex: 2,
              filter: isSquishing ? "brightness(1.2)" : isEating ? "brightness(1.1)" : "none",
              transform: isSquishing ? "scale(1.08, 0.92)" : isEating ? "scale(0.96, 1.04)" : undefined,
              transition: "transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1)",
            }}
          >
            <AvatarStage
              subject={subjectFromAvatar(avatar)}
              state={state}
              presentation={presentation}
              hud={hud}
              size={size}
              interactive={interactive}
              onDoubleClick={onDoubleClick}
              onError={onError}
            />
          </div>
        ) : loadError ? (
          <div
            className="companion-pack-error"
            style={{
              width: size,
              height: size,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              background: "rgba(30, 10, 15, 0.9)",
              border: "2px dashed #ff4757",
              borderRadius: "16px",
              color: "#ff6b81",
              padding: "16px",
              textAlign: "center",
              boxSizing: "border-box",
              position: "relative",
              zIndex: 2,
            }}
          >
            <span style={{ fontSize: "20px", marginBottom: "6px" }}>⚠️</span>
            <strong style={{ fontSize: "11px", letterSpacing: "1px" }}>COMPANION LOAD ERROR</strong>
            <p style={{ fontSize: "10px", margin: "6px 0 0 0", color: "#ccc" }}>{loadError}</p>
          </div>
        ) : animation ? (
          <canvas
            ref={canvasRef}
            width={size}
            height={size}
            style={{
              width: size,
              height: size,
              imageRendering: "pixelated",
              position: "relative",
              zIndex: 2,
              filter: isSquishing ? "brightness(1.15)" : isEating ? "brightness(1.1)" : "none",
              transform: isSquishing
                ? "scale(1.09, 0.91)"
                : isEating
                ? "scale(0.95, 1.05)"
                : state === "sleeping"
                ? "translateY(3px) scale(0.98)"
                : undefined,
              animation:
                isSquishing || isEating
                  ? "none"
                  : state === "sleeping"
                  ? "companionSleep 4s ease-in-out infinite"
                  : "companionBreathe 3.8s ease-in-out infinite",
              transition: "transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1)",
            }}
          />
        ) : (
          <div
            style={{
              width: size,
              height: size,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              color: "#94a3b8",
              fontSize: "11px",
              gap: "6px",
            }}
          >
            <span style={{ fontSize: "24px" }}>🌸</span>
            <span>Welcoming companion...</span>
          </div>
        )}
      </div>

      {/* 3. Living AgentPet Unified Bottom Capsule & Customizer (Rounded & Organic!) */}
      {!bare && (
      <div
        className="companion-unified-capsule"
        style={{
          width: "calc(100% - 12px)",
          background: "rgba(15, 23, 42, 0.9)",
          border: `1px solid ${activeThemeColor}44`,
          borderRadius: "20px",
          padding: "6px 10px",
          display: "flex",
          flexDirection: "column",
          gap: "5px",
          zIndex: 8,
          backdropFilter: "blur(14px)",
          boxShadow: `0 8px 24px rgba(0, 0, 0, 0.35), 0 0 0 1px ${activeThemeColor}22`,
          boxSizing: "border-box",
        }}
      >
        {/* Row 1: Identity, Affection Heart Meter, Mode & Style Toggle */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: "9.5px",
            color: "#e2e8f0",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "5px", fontWeight: 700 }}>
            <span>{companionInfo.icon}</span>
            <span style={{ color: activeThemeColor, fontWeight: 800 }}>{displayName}</span>
            {/* Cute Affection Heart Meter */}
            <span
              title={`Affection: ${affection}% — pet and treat to increase!`}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "2px",
                fontSize: "8.5px",
                background: "rgba(255, 105, 180, 0.15)",
                border: "1px solid rgba(255, 105, 180, 0.3)",
                borderRadius: "999px",
                padding: "1px 5px",
                color: "#ff69b4",
                fontWeight: 800,
              }}
            >
              ❤️ {affection}%
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "5px" }}>
            {/* Quick Customizer Toggle Button */}
            <button
              type="button"
              onClick={() => setIsCustomizerOpen((prev) => !prev)}
              title="Customize pet nickname, accessory, aura and personality"
              style={{
                fontSize: "8.5px",
                fontWeight: 800,
                color: isCustomizerOpen ? "#facc15" : "#cbd5e1",
                background: isCustomizerOpen ? "rgba(250, 204, 21, 0.2)" : "rgba(255, 255, 255, 0.08)",
                border: `1px solid ${isCustomizerOpen ? "#facc15" : "rgba(255, 255, 255, 0.15)"}`,
                borderRadius: "999px",
                padding: "2px 7px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "2px",
              }}
            >
              <span>🎨 Style</span>
            </button>
            <button
              type="button"
              onClick={handleToggleRenderMode}
              title={`Switch to ${renderMode === "3d" ? "2D Pixel Sprite" : "3D Living Avatar"} mode`}
              style={{
                fontSize: "8.5px",
                fontWeight: 800,
                color: renderMode === "3d" ? "#38bdf8" : "#94a3b8",
                background: renderMode === "3d" ? "rgba(56, 189, 248, 0.2)" : "rgba(255, 255, 255, 0.08)",
                border: `1px solid ${renderMode === "3d" ? "#38bdf8" : "rgba(255, 255, 255, 0.15)"}`,
                borderRadius: "999px",
                padding: "2px 6px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "2px",
              }}
            >
              <span>{renderMode === "3d" ? "🎮 3D" : "🖼️ 2D"}</span>
            </button>
            <span
              style={{
                fontSize: "8.5px",
                fontWeight: 700,
                color: state === "working" ? "#38bdf8" : state === "error" ? "#ff4757" : "#a855f7",
                background: "rgba(255, 255, 255, 0.06)",
                padding: "1px 6px",
                borderRadius: "999px",
              }}
            >
              {state.toUpperCase()}
            </span>
            <span style={{ color: "#94a3b8", fontSize: "8.5px" }}>
              LVL {bondLevel} {bondTitle}
            </span>
          </div>
        </div>

        {/* In-Place Pet Customizer Drawer (Slides open on clicking Style) */}
        {isCustomizerOpen && (
          <div
            style={{
              background: "rgba(2, 6, 23, 0.8)",
              border: `1px solid ${activeThemeColor}33`,
              borderRadius: "14px",
              padding: "7px 8px",
              display: "flex",
              flexDirection: "column",
              gap: "6px",
              marginTop: "3px",
              animation: "bubbleFloatIn 0.2s ease-out",
            }}
          >
            {/* Nickname Editor */}
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span style={{ fontSize: "9px", color: "#94a3b8", minWidth: "55px", fontWeight: 700 }}>Nickname:</span>
              <input
                type="text"
                placeholder={avatar?.name ?? petName}
                value={customNickname}
                onChange={(e) => saveNickname(e.target.value)}
                maxLength={18}
                style={{
                  flex: 1,
                  background: "rgba(15, 23, 42, 0.9)",
                  border: "1px solid rgba(148, 163, 184, 0.3)",
                  borderRadius: "8px",
                  color: "#f8fafc",
                  fontSize: "9.5px",
                  padding: "3px 7px",
                  outline: "none",
                }}
              />
            </div>

            {/* Accessory Selector */}
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span style={{ fontSize: "9px", color: "#94a3b8", minWidth: "55px", fontWeight: 700 }}>Accessory:</span>
              <div style={{ display: "flex", gap: "4px", flexWrap: "wrap", flex: 1 }}>
                {[
                  { id: "none", label: "❌" },
                  { id: "ribbon", label: "🎀" },
                  { id: "flower", label: "🌸" },
                  { id: "crown", label: "👑" },
                  { id: "glasses", label: "👓" },
                  { id: "halo", label: "😇" },
                  { id: "bell", label: "🔔" },
                ].map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => saveAccessory(item.id)}
                    style={{
                      background: customAccessory === item.id ? "rgba(56, 189, 248, 0.3)" : "rgba(255, 255, 255, 0.07)",
                      border: `1px solid ${customAccessory === item.id ? "#38bdf8" : "rgba(255, 255, 255, 0.12)"}`,
                      borderRadius: "6px",
                      padding: "2px 5px",
                      fontSize: "11px",
                      cursor: "pointer",
                    }}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Aura Glow Color Presets */}
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span style={{ fontSize: "9px", color: "#94a3b8", minWidth: "55px", fontWeight: 700 }}>Aura Glow:</span>
              <div style={{ display: "flex", gap: "5px", alignItems: "center" }}>
                {[
                  { color: "#ff69b4", label: "Sakura" },
                  { color: "#38bdf8", label: "Sky" },
                  { color: "#facc15", label: "Honey" },
                  { color: "#c084fc", label: "Violet" },
                  { color: "#34d399", label: "Mint" },
                ].map((item) => (
                  <button
                    key={item.color}
                    type="button"
                    onClick={() => saveAura(item.color)}
                    title={item.label}
                    style={{
                      width: "14px",
                      height: "14px",
                      borderRadius: "50%",
                      backgroundColor: item.color,
                      border: customAura === item.color ? "2px solid #ffffff" : "1px solid rgba(0,0,0,0.4)",
                      boxShadow: customAura === item.color ? `0 0 8px ${item.color}` : "none",
                      cursor: "pointer",
                      padding: 0,
                    }}
                  />
                ))}
              </div>
            </div>

            {/* Personality Vibe */}
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <span style={{ fontSize: "9px", color: "#94a3b8", minWidth: "55px", fontWeight: 700 }}>Vibe:</span>
              <div style={{ display: "flex", gap: "4px" }}>
                {[
                  { id: "sweet", label: "💖 Sweet" },
                  { id: "playful", label: "⚡ Playful" },
                  { id: "loyal", label: "🛡️ Loyal" },
                  { id: "calm", label: "🌿 Calm" },
                ].map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => saveVibe(item.id as "sweet" | "playful" | "loyal" | "calm")}
                    style={{
                      background: customVibe === item.id ? "rgba(168, 85, 247, 0.25)" : "rgba(255, 255, 255, 0.06)",
                      border: `1px solid ${customVibe === item.id ? "#c084fc" : "rgba(255, 255, 255, 0.1)"}`,
                      borderRadius: "6px",
                      padding: "2px 6px",
                      fontSize: "8.5px",
                      fontWeight: 700,
                      color: customVibe === item.id ? "#f3e8ff" : "#94a3b8",
                      cursor: "pointer",
                    }}
                  >
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Row 2: Tactile Living Controls (Pet, Feed, Heart-to-Heart, Wake) */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "4px",
            marginTop: "1px",
          }}
        >
          <button
            type="button"
            onClick={handlePet}
            title="Pet companion (Purrs, blush, +Affection, +1 XP)"
            style={{
              flex: 1,
              background: "rgba(255, 105, 180, 0.15)",
              border: "1px solid rgba(255, 105, 180, 0.35)",
              borderRadius: "14px",
              color: "#ff80bf",
              padding: "4px 6px",
              fontSize: "9.5px",
              fontWeight: 700,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "3px",
              transition: "all 0.15s ease",
            }}
          >
            <span>💖</span>
            <span>Pet</span>
          </button>

          <button
            type="button"
            onClick={handleFeed}
            title={`Feed ${companionInfo.treatName} (+Affection, +2 XP)`}
            style={{
              flex: 1,
              background: "rgba(250, 204, 21, 0.15)",
              border: "1px solid rgba(250, 204, 21, 0.35)",
              borderRadius: "14px",
              color: "#fde047",
              padding: "4px 6px",
              fontSize: "9.5px",
              fontWeight: 700,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "3px",
              transition: "all 0.15s ease",
            }}
          >
            <span>{companionInfo.treatEmoji}</span>
            <span>{companionInfo.treatName}</span>
          </button>

          <button
            type="button"
            onClick={handleHeartToHeart}
            title="Heart-to-Heart: hear warm, encouraging words"
            style={{
              flex: 1,
              background: "rgba(56, 189, 248, 0.15)",
              border: "1px solid rgba(56, 189, 248, 0.35)",
              borderRadius: "14px",
              color: "#7dd3fc",
              padding: "4px 6px",
              fontSize: "9.5px",
              fontWeight: 700,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "3px",
              transition: "all 0.15s ease",
            }}
          >
            <span>💬</span>
            <span>Talk</span>
          </button>

          {state === "sleeping" ? (
            <button
              type="button"
              onClick={handleWake}
              title="Gently wake up companion"
              style={{
                flex: 1,
                background: "linear-gradient(135deg, #f59e0b 0%, #d97706 100%)",
                border: "none",
                borderRadius: "14px",
                color: "#ffffff",
                padding: "4px 6px",
                fontSize: "9.5px",
                fontWeight: 700,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: "3px",
              }}
            >
              <span>⏰</span>
              <span>Wake</span>
            </button>
          ) : (
            <button
              type="button"
              onClick={handleCycleThought}
              title="Hear companion's current thought"
              style={{
                background: "rgba(255, 255, 255, 0.08)",
                border: "1px solid rgba(255, 255, 255, 0.15)",
                borderRadius: "14px",
                color: "#cbd5e1",
                padding: "4px 8px",
                fontSize: "9.5px",
                fontWeight: 600,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: "2px",
              }}
            >
              <span>💭</span>
            </button>
          )}

          {onNextPet && (
            <button
              type="button"
              onClick={onNextPet}
              title="Cycle to next companion pack"
              style={{
                background: "rgba(255, 255, 255, 0.08)",
                border: "1px solid rgba(255, 255, 255, 0.15)",
                borderRadius: "14px",
                color: "#38bdf8",
                padding: "4px 8px",
                fontSize: "9.5px",
                fontWeight: 700,
                cursor: "pointer",
              }}
            >
              ⇄
            </button>
          )}
        </div>
      </div>
      )}

      {/* Global CSS keyframes for organic companion movements */}
      <style>{`
        @keyframes companionBreathe {
          0% {
            transform: translateY(0px) scale(1, 1);
          }
          50% {
            transform: translateY(-4px) scale(1.015, 0.985);
          }
          100% {
            transform: translateY(0px) scale(1, 1);
          }
        }
        @keyframes companionSleep {
          0% {
            transform: translateY(2px) scale(0.99, 0.97);
          }
          50% {
            transform: translateY(4px) scale(0.97, 0.99);
          }
          100% {
            transform: translateY(2px) scale(0.99, 0.97);
          }
        }
        @keyframes companionShadow {
          0% {
            transform: scale(1);
            opacity: 0.45;
          }
          50% {
            transform: scale(0.85);
            opacity: 0.25;
          }
          100% {
            transform: scale(1);
            opacity: 0.45;
          }
        }
        @keyframes floatHeart {
          0% {
            opacity: 1;
            transform: translate(-50%, -50%) scale(0.5) translateY(0);
          }
          50% {
            opacity: 1;
            transform: translate(-50%, -50%) scale(1.2) translateY(-20px);
          }
          100% {
            opacity: 0;
            transform: translate(-50%, -50%) scale(0.9) translateY(-45px);
          }
        }
        @keyframes dropTreat {
          0% {
            opacity: 1;
            transform: translate(-50%, -80%) scale(1.2);
          }
          60% {
            opacity: 1;
            transform: translate(-50%, 0px) scale(1);
          }
          100% {
            opacity: 0;
            transform: translate(-50%, 10px) scale(0.7);
          }
        }
        @keyframes bubbleFloatIn {
          0% {
            opacity: 0;
            transform: translateY(4px) scale(0.97);
          }
          100% {
            opacity: 1;
            transform: translateY(0px) scale(1);
          }
        }
      `}</style>
    </div>
  );
}
