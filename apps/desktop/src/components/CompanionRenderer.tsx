import { useCallback, useEffect, useRef, useState } from "react";
import { Companion3D } from "./Companion3D";

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
  quotaWindow?: QuotaWindow | null;
  quotaProvenance?: string;
  size?: number;
  interactive?: boolean;
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
      const isMuted = localStorage.getItem("agentpet_sound_muted") === "true";
      if (isMuted) return;

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
  size = 200,
  interactive = true,
  mode,
  onModeToggle,
  onDoubleClick,
  onNextPet,
  onWakePet,
  onError,
}: CompanionRendererProps) {
  const [renderMode, setRenderMode] = useState<"3d" | "2d">(() => {
    if (mode) return mode;
    try {
      const saved = localStorage.getItem("agentpet_render_mode");
      if (saved === "2d" || saved === "3d") return saved;
    } catch {
      // ignore
    }
    return "3d";
  });

  useEffect(() => {
    if (mode && mode !== renderMode) {
      setRenderMode(mode);
    }
  }, [mode]);

  useEffect(() => {
    return () => {
      AudioManager.getInstance().dispose();
    };
  }, []);

  const handleToggleRenderMode = (e: React.MouseEvent) => {
    e.stopPropagation();
    const nextMode = renderMode === "3d" ? "2d" : "3d";
    setRenderMode(nextMode);
    try {
      localStorage.setItem("agentpet_render_mode", nextMode);
    } catch {
      // ignore
    }
    if (onModeToggle) {
      onModeToggle(nextMode);
    }
  };

  const [frameIndex, setFrameIndex] = useState(0);
  const [imageLoaded, setImageLoaded] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Human Touch States
  const [particles, setParticles] = useState<FloatingParticle[]>([]);
  const [isSquishing, setIsSquishing] = useState(false);
  const [isEating, setIsEating] = useState(false);
  const [activeSpeech, setActiveSpeech] = useState<string>("");
  const [speechVisible, setSpeechVisible] = useState(true);
  const [bondXp, setBondXp] = useState<number>(() => {
    const saved = localStorage.getItem("agentpet_companion_bond_xp");
    return saved ? parseInt(saved, 10) || 15 : 15;
  });
  const [wellnessAlert, setWellnessAlert] = useState<string | null>(null);

  const petName = animation?.petName ?? "Companion";
  const nameLower = petName.toLowerCase();
  const isWukong = nameLower.includes("wukong") || nameLower.includes("monkey");
  const isNezuko = nameLower.includes("nezuko");
  const isArjuna = nameLower.includes("arjuna");
  const isAoi = nameLower.includes("aoi");
  const isNinjaCat = nameLower.includes("cat") || nameLower.includes("ninja");

  // Character metadata & favorites
  const companionInfo = isWukong
    ? {
        icon: "☁️",
        themeColor: "#ffb703",
        glow: "radial-gradient(circle, #ffb703 0%, rgba(230, 57, 70, 0.45) 60%, transparent 80%)",
        treatName: "Peach",
        treatEmoji: "🍑",
      }
    : isNezuko
    ? {
        icon: "🌸",
        themeColor: "#ff69b4",
        glow: "radial-gradient(circle, #ff6b8b 0%, rgba(255, 105, 180, 0.45) 60%, transparent 80%)",
        treatName: "Tea",
        treatEmoji: "🍵",
      }
    : isArjuna
    ? {
        icon: "🏹",
        themeColor: "#ff9933",
        glow: "radial-gradient(circle, #ff9933 0%, rgba(255, 215, 0, 0.45) 60%, transparent 80%)",
        treatName: "Honey",
        treatEmoji: "🍯",
      }
    : isNinjaCat
    ? {
        icon: "🐾",
        themeColor: "#00f0ff",
        glow: "radial-gradient(circle, #00f0ff 0%, rgba(13, 148, 136, 0.45) 60%, transparent 80%)",
        treatName: "Fish",
        treatEmoji: "🐟",
      }
    : isAoi
    ? {
        icon: "🎧",
        themeColor: "#38bdf8",
        glow: "radial-gradient(circle, #38bdf8 0%, rgba(14, 165, 233, 0.45) 60%, transparent 80%)",
        treatName: "Coffee",
        treatEmoji: "☕",
      }
    : {
        icon: "✨",
        themeColor: "#a855f7",
        glow: "radial-gradient(circle, #a855f7 0%, rgba(168, 85, 247, 0.45) 60%, transparent 80%)",
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

  // Authentic Human & Character Dialogue
  const getSpeechForState = useCallback(
    (currentState: PetState, trigger: "auto" | "pet" | "feed" | "wake"): string => {
      // 1. Feeding Response
      if (trigger === "feed") {
        if (isWukong) return "Mmm! Celestial peach! Power level rising, Chief! 🍑✨";
        if (isNezuko) return "Mmm-ph! (Nezuko sips the warm tea happily!) (◕‿◕)🍵";
        if (isArjuna) return "Saffron nectar! My archery focus is refreshed. 🍯🏹";
        if (isNinjaCat) return "*crunch crunch* Purrrrr! Delicious fish! 🐟🐾";
        return "Mmm, warm coffee! Ready for the next sprint, Chief! ☕⚡";
      }

      // 2. Wake-up Response
      if (trigger === "wake") {
        if (isWukong) return "Hahaha! The Great Sage awakens! Point me to the bugs! ⚡☁️";
        if (isNezuko) return "Mmm-ph! (Rubs her eyes and smiles warmly!) 🌸✨";
        if (isArjuna) return "Awake and vigilant. The Gandiva bow stands ready! 🏹";
        if (isNinjaCat) return "*stretches stealthily* Meow! Ready for deployment! 🐾";
        return "Systems online! Let's conquer this codebase together! ⚡";
      }

      // 3. Petting / Affection Response
      if (trigger === "pet") {
        if (isWukong) {
          const wukongPats = [
            "Hahaha! That tickles, Chief! The Monkey King is energized! 🍑✨",
            "My Somersault Cloud glides smoother with your support! ☁️",
            "A celestial blessing for your keyboard, Chief! 💖",
          ];
          return wukongPats[Math.floor(Math.random() * wukongPats.length)];
        }
        if (isNezuko) {
          const nezukoPats = [
            "Mmm-ph! (Nezuko smiles warmly and leans into your hand) ✨",
            "Nezuko gently pats your head back! (◕‿◕)♡",
            "A warm energy fills your workspace! Bond increased! 🌸💖",
          ];
          return nezukoPats[Math.floor(Math.random() * nezukoPats.length)];
        }
        if (isArjuna) {
          const arjunaPats = [
            "Honor and gratitude, Chief. Together our aim is true! 🏹",
            "Your leadership inspires the archer's soul. Let's build! ✨",
          ];
          return arjunaPats[Math.floor(Math.random() * arjunaPats.length)];
        }
        if (isNinjaCat) {
          const catPats = [
            "*loud purring* Purrrrr... blessed by the shinobi cat! 🐾💖",
            "*headbutts your cursor* Meow! You're doing great, Chief!",
          ];
          return catPats[Math.floor(Math.random() * catPats.length)];
        }
        const aoiPats = [
          "Aw, thanks for the pats, Chief! (◕‿◕)♡ In the zone!",
          "Bond level up! We're an unbeatable coding duo! 💖",
          "Chief, you're doing great today. Let's keep building! 🎧✨",
        ];
        return aoiPats[Math.floor(Math.random() * aoiPats.length)];
      }

      // 4. Automatic Contextual State Dialogue
      if (isWukong) {
        switch (currentState) {
          case "working":
            return "My Gold-Banded Cudgel will crush these merge conflicts! ⚡";
          case "thinking":
            return "Pondering the Dao of clean code... 🤔";
          case "success":
            return "HA! Another feature vanquished! The heavens rejoice! 🎉";
          case "error":
            return "A minor bug demon! Stand back, Chief, we'll smash it! 💪";
          case "sleeping":
            return "Zzz... napping on my Somersault Cloud... ☁️💤";
          case "waiting":
          case "attention_required":
            return "Awaiting your command, Chief! Staff is ready! 🐒";
          default:
            return "Riding my Somersault Cloud. Ready when you are, Chief! ☁️";
        }
      }

      if (isNezuko) {
        switch (currentState) {
          case "working":
            return "Mmm! (Guarding your focus while you code!) ⚡🌸";
          case "thinking":
            return "Mmm? (Tilts head curiously at the algorithm) 🤔";
          case "success":
            return "Mmm-ph!! (Jumps with joy!) All tests passed! 🎉";
          case "error":
            return "Mmm... (Hands you a lucky charm. We'll fix it!) 💪";
          case "sleeping":
            return "Zzz... resting peacefully in her cozy box... 🌙💤";
          case "waiting":
          case "attention_required":
            return "Mmm-ph! (Waiting for your command, Chief!) 🌸";
          default:
            return "Keeping watch by your side. Hydrate when ready, Chief! 🍵";
        }
      }

      if (isArjuna) {
        switch (currentState) {
          case "working":
            return "Arrows of logic flying true to the target! ⚡🏹";
          case "thinking":
            return "Surveying the battlefield of code... finding the clean route. 🤔";
          case "success":
            return "Bullseye! Every test struck with precision! 🎯";
          case "error":
            return "Adjusting stance! We'll strike the root cause together! 🛡️";
          case "sleeping":
            return "Zzz... resting upon his chariot beneath the stars... 🌙";
          default:
            return "Gandiva bow strung. Focus unbroken, Chief. 🏹";
        }
      }

      if (isNinjaCat) {
        switch (currentState) {
          case "working":
            return "*silent paws typing at 200 WPM* 🐾⚡";
          case "thinking":
            return "*ears twitching* Sensing an async race condition... 🤔";
          case "success":
            return "Target silenced! Build green and stealthy! 🐾🎉";
          case "error":
            return "Hiss! Bug detected in perimeter! Ambushing... ⚔️";
          case "sleeping":
            return "Zzz... curled up in a neat little cat ball... 🌙🐾";
          default:
            return "*purrs* All systems quiet and secure, Chief. 🐾";
        }
      }

      // Aoi / Default
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
    [isArjuna, isNezuko, isNinjaCat, isWukong]
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
  }, []);

  // Petting Action
  const handlePet = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setIsSquishing(true);
    setTimeout(() => setIsSquishing(false), 240);

    playChime("pet");

    // Spawn floating heart particle
    const rect = canvasRef.current?.getBoundingClientRect();
    const x = rect ? rect.width / 2 + (Math.random() * 40 - 20) : size / 2;
    const y = rect ? rect.height / 2 + (Math.random() * 30 - 15) : size / 2;
    const symbols = ["💖", "✨", "🌸", "⭐", "♡"];
    const symbol = symbols[Math.floor(Math.random() * symbols.length)];

    const id = Date.now() + Math.random();
    setParticles((prev) => [...prev, { id, x, y, symbol }]);
    setTimeout(() => {
      setParticles((prev) => prev.filter((p) => p.id !== id));
    }, 1200);

    // Increase Bond XP
    setBondXp((prev) => {
      const next = prev + 1;
      try {
        localStorage.setItem("agentpet_companion_bond_xp", String(next));
      } catch {
        // ignore
      }
      return next;
    });

    const response = getSpeechForState(state, "pet");
    setActiveSpeech(response);
    setSpeechVisible(true);
  };

  // Feed Snack Action
  const handleFeed = (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setIsEating(true);
    setTimeout(() => setIsEating(false), 450);

    playChime("eat");

    // Spawn treat drop particle
    const rect = canvasRef.current?.getBoundingClientRect();
    const x = rect ? rect.width / 2 : size / 2;
    const y = rect ? rect.height / 2 - 20 : size / 2 - 20;

    const id = Date.now() + Math.random();
    setParticles((prev) => [
      ...prev,
      { id, x, y, symbol: companionInfo.treatEmoji, isTreat: true },
    ]);
    setTimeout(() => {
      setParticles((prev) => prev.filter((p) => p.id !== id));
    }, 1200);

    // Increase Bond XP
    setBondXp((prev) => {
      const next = prev + 2;
      try {
        localStorage.setItem("agentpet_companion_bond_xp", String(next));
      } catch {
        // ignore
      }
      return next;
    });

    const response = getSpeechForState(state, "feed");
    setActiveSpeech(response);
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
    if (!animation?.imageDataUrl) {
      setImageLoaded(false);
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

  if (loadError) {
    return (
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
          borderRadius: "8px",
          color: "#ff6b81",
          padding: "16px",
          textAlign: "center",
          boxSizing: "border-box",
        }}
      >
        <span style={{ fontSize: "20px", marginBottom: "6px" }}>⚠️</span>
        <strong style={{ fontSize: "11px", letterSpacing: "1px" }}>COMPANION LOAD ERROR</strong>
        <p style={{ fontSize: "10px", margin: "6px 0 0 0", color: "#ccc" }}>{loadError}</p>
      </div>
    );
  }

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
      {/* 1. Sleek In-Bounds Speech Bubble with Speech Tail */}
      {speechVisible && activeSpeech && (
        <div
          className="companion-speech-bubble"
          onClick={handleCycleThought}
          title="Click to cycle thoughts"
          style={{
            position: "relative",
            width: "calc(100% - 16px)",
            maxWidth: "240px",
            background: "rgba(10, 15, 28, 0.94)",
            border: `1px solid ${companionInfo.themeColor}55`,
            borderRadius: "10px",
            padding: "6px 10px",
            color: "#f8fafc",
            fontSize: "10.5px",
            lineHeight: "1.35",
            fontWeight: 500,
            boxShadow: `0 4px 16px ${companionInfo.themeColor}25`,
            backdropFilter: "blur(10px)",
            zIndex: 10,
            display: "flex",
            alignItems: "center",
            gap: "6px",
            marginBottom: "6px",
            animation: "bubbleFloatIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1)",
            cursor: "pointer",
          }}
        >
          <span style={{ fontSize: "12px", flexShrink: 0 }}>{companionInfo.icon}</span>
          <span style={{ flex: 1, letterSpacing: "0.2px" }}>{activeSpeech}</span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setSpeechVisible(false);
            }}
            title="Dismiss speech"
            style={{
              background: "transparent",
              border: "none",
              color: "#94a3b8",
              cursor: "pointer",
              fontSize: "11px",
              padding: "0 2px",
            }}
          >
            ×
          </button>
        </div>
      )}

      {/* Wellness Human Care Popover */}
      {wellnessAlert && (
        <div
          onClick={(e) => {
            e.stopPropagation();
            setWellnessAlert(null);
            setBondXp((x) => x + 5);
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
            padding: "4px 10px",
            borderRadius: "16px",
            fontSize: "9.5px",
            fontWeight: 700,
            letterSpacing: "0.3px",
            boxShadow: "0 4px 12px rgba(16, 185, 129, 0.4)",
            zIndex: 20,
            whiteSpace: "nowrap",
            cursor: "pointer",
            animation: "pulse 1.5s infinite ease-in-out",
          }}
        >
          {wellnessAlert} · <span style={{ textDecoration: "underline" }}>Done! (+5 XP)</span>
        </div>
      )}

      {/* 2. Companion Character Stage */}
      <div
        onClick={handlePet}
        title="Click to pet & bond with companion"
        style={{
          position: "relative",
          width: size,
          height: size,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: interactive ? "pointer" : "default",
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
            opacity: state === "idle" ? 0.35 : state === "sleeping" ? 0.15 : 0.7,
            background: companionInfo.glow,
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
            <Companion3D
              state={state}
              petName={petName}
              size={size}
              interactive={interactive}
              onDoubleClick={onDoubleClick}
              onError={onError}
            />
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

      {/* 3. Unified Bottom Status & Action Capsule (Replaces colliding pills) */}
      <div
        className="companion-unified-capsule"
        style={{
          width: "calc(100% - 12px)",
          background: "rgba(10, 16, 30, 0.92)",
          border: `1px solid ${companionInfo.themeColor}44`,
          borderRadius: "12px",
          padding: "5px 8px",
          display: "flex",
          flexDirection: "column",
          gap: "4px",
          zIndex: 8,
          backdropFilter: "blur(10px)",
          boxShadow: "0 4px 16px rgba(0, 0, 0, 0.4)",
          boxSizing: "border-box",
        }}
      >
        {/* Row 1: Character identity, 3D/2D toggle, state, and bond level */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: "9.5px",
            color: "#e2e8f0",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "4px", fontWeight: 700 }}>
            <span>{companionInfo.icon}</span>
            <span style={{ color: companionInfo.themeColor }}>{petName}</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
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
                borderRadius: "4px",
                padding: "1px 6px",
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
                padding: "1px 5px",
                borderRadius: "4px",
              }}
            >
              {state.toUpperCase()}
            </span>
            <span style={{ color: "#94a3b8", fontSize: "8.5px" }}>
              LVL {bondLevel} {bondTitle}
            </span>
          </div>
        </div>

        {/* Row 2: Tactile Quick Action Controls */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "4px",
            marginTop: "2px",
          }}
        >
          <button
            type="button"
            onClick={handlePet}
            title="Pet companion (+1 Bond XP)"
            style={{
              flex: 1,
              background: "rgba(255, 255, 255, 0.08)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              borderRadius: "6px",
              color: "#ffffff",
              padding: "3px 4px",
              fontSize: "9.5px",
              fontWeight: 600,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "3px",
            }}
          >
            <span>👋</span>
            <span>Pet</span>
          </button>

          <button
            type="button"
            onClick={handleFeed}
            title={`Feed ${companionInfo.treatName} (+2 Bond XP)`}
            style={{
              flex: 1,
              background: "rgba(255, 255, 255, 0.08)",
              border: "1px solid rgba(255, 255, 255, 0.12)",
              borderRadius: "6px",
              color: "#ffffff",
              padding: "3px 4px",
              fontSize: "9.5px",
              fontWeight: 600,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "3px",
            }}
          >
            <span>{companionInfo.treatEmoji}</span>
            <span>{companionInfo.treatName}</span>
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
                borderRadius: "6px",
                color: "#ffffff",
                padding: "3px 4px",
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
              title="Hear companion's thought"
              style={{
                flex: 1,
                background: "rgba(255, 255, 255, 0.08)",
                border: "1px solid rgba(255, 255, 255, 0.12)",
                borderRadius: "6px",
                color: "#ffffff",
                padding: "3px 4px",
                fontSize: "9.5px",
                fontWeight: 600,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: "3px",
              }}
            >
              <span>💭</span>
              <span>Talk</span>
            </button>
          )}

          {onNextPet && (
            <button
              type="button"
              onClick={onNextPet}
              title="Cycle to next companion pack"
              style={{
                background: "rgba(255, 255, 255, 0.08)",
                border: "1px solid rgba(255, 255, 255, 0.12)",
                borderRadius: "6px",
                color: "#38bdf8",
                padding: "3px 7px",
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
