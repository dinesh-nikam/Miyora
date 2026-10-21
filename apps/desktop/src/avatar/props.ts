// Stage props around the avatar.
//
// Props make state and progress readable at a glance: a laptop while the
// agent works, thought bubbles while it thinks, a glyph for attention or
// errors, confetti for completions, and a pedestal ring whose arc is the real
// XP progress toward the next level.

import * as THREE from "three";
import type { AvatarHud } from "./types";
import type { PropFlags } from "./motion";

const PEDESTAL_RADIUS = 0.38;
const XP_RING_RADIUS = 0.42;
const CONFETTI_COUNT = 70;
const CONFETTI_GRAVITY = 1.6;
const CONFETTI_COLORS = ["#f472b6", "#facc15", "#38bdf8", "#4ade80", "#a78bfa", "#fb923c"];
const SLEEP_Z_COUNT = 3;
const SCREEN_LINES = 9;
const SCREEN_REDRAW_INTERVAL = 0.12;

export interface PropLayout {
  headTopY: number;
  deskY: number;
}

interface TextSprite {
  sprite: THREE.Sprite;
  canvas: HTMLCanvasElement;
  texture: THREE.CanvasTexture;
  text: string;
  color: string;
}

function createTextSprite(width: number, height: number, worldHeight: number): TextSprite {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  const material = new THREE.SpriteMaterial({ map: texture, transparent: true, depthWrite: false });
  const sprite = new THREE.Sprite(material);
  sprite.scale.set((worldHeight * width) / height, worldHeight, 1);
  return { sprite, canvas, texture, text: "", color: "" };
}

function drawText(target: TextSprite, text: string, color: string, badge: boolean): void {
  if (target.text === text && target.color === color) return;
  target.text = text;
  target.color = color;
  const context = target.canvas.getContext("2d");
  if (!context) return;
  const { width, height } = target.canvas;
  context.clearRect(0, 0, width, height);
  if (badge) {
    context.fillStyle = "rgba(10, 15, 25, 0.78)";
    const radius = height / 2;
    context.beginPath();
    context.moveTo(radius, 0);
    context.arcTo(width, 0, width, height, radius);
    context.arcTo(width, height, 0, height, radius);
    context.arcTo(0, height, 0, 0, radius);
    context.arcTo(0, 0, width, 0, radius);
    context.closePath();
    context.fill();
  }
  context.font = `800 ${Math.round(height * 0.62)}px "Segoe UI", system-ui, sans-serif`;
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.lineWidth = height * 0.08;
  context.strokeStyle = "rgba(0, 0, 0, 0.55)";
  context.strokeText(text, width / 2, height / 2 + 2);
  context.fillStyle = color;
  context.fillText(text, width / 2, height / 2 + 2);
  target.texture.needsUpdate = true;
}

export class StageProps {
  readonly group = new THREE.Group();
  /** Screen-facing overlays that must not turn when the user orbits the stage. */
  readonly hudGroup = new THREE.Group();
  private readonly accentColor = new THREE.Color();
  private readonly ringMaterial: THREE.MeshBasicMaterial;
  private xpRing: THREE.Mesh | null = null;
  private xpFraction = -1;
  private readonly levelLabel = createTextSprite(512, 96, 0.075);
  private readonly glyph = createTextSprite(512, 160, 0.13);
  private readonly laptop = new THREE.Group();
  private readonly screenCanvas = document.createElement("canvas");
  private readonly screenTexture: THREE.CanvasTexture;
  private screenClock = 0;
  private screenScroll = 0;
  private readonly thought = new THREE.Group();
  private readonly sleepZ: TextSprite[] = [];
  private readonly food = new THREE.Group();
  private readonly aura: THREE.Mesh;
  private readonly confetti: THREE.Points;
  private readonly confettiVelocity = new Float32Array(CONFETTI_COUNT * 3);
  private confettiActive = false;
  private layout: PropLayout = { headTopY: 0.9, deskY: 0.35 };

  constructor(accent: string) {
    this.accentColor.set(accent);

    // Pedestal and contact shadow.
    const pedestal = new THREE.Mesh(
      new THREE.CylinderGeometry(PEDESTAL_RADIUS, PEDESTAL_RADIUS * 1.06, 0.04, 48),
      new THREE.MeshStandardMaterial({ color: 0x0b1220, roughness: 0.35, metalness: 0.7 }),
    );
    pedestal.position.y = -0.02;
    this.group.add(pedestal);
    const shadowCanvas = document.createElement("canvas");
    shadowCanvas.width = 128;
    shadowCanvas.height = 128;
    const shadowContext = shadowCanvas.getContext("2d");
    if (shadowContext) {
      const gradient = shadowContext.createRadialGradient(64, 64, 4, 64, 64, 64);
      gradient.addColorStop(0, "rgba(0,0,0,0.55)");
      gradient.addColorStop(1, "rgba(0,0,0,0)");
      shadowContext.fillStyle = gradient;
      shadowContext.fillRect(0, 0, 128, 128);
    }
    const shadow = new THREE.Mesh(
      new THREE.PlaneGeometry(0.5, 0.5),
      new THREE.MeshBasicMaterial({ map: new THREE.CanvasTexture(shadowCanvas), transparent: true, depthWrite: false }),
    );
    shadow.rotation.x = -Math.PI / 2;
    shadow.position.y = 0.002;
    this.group.add(shadow);

    // XP progress ring: faint full track plus a bright arc.
    const track = new THREE.Mesh(
      new THREE.TorusGeometry(XP_RING_RADIUS, 0.006, 6, 96),
      new THREE.MeshBasicMaterial({ color: 0x334155, transparent: true, opacity: 0.6 }),
    );
    track.rotation.x = Math.PI / 2;
    track.position.y = 0.004;
    this.group.add(track);
    this.ringMaterial = new THREE.MeshBasicMaterial({ color: this.accentColor });
    this.levelLabel.sprite.position.set(0, 0.05, XP_RING_RADIUS + 0.03);
    this.levelLabel.sprite.visible = false;
    this.hudGroup.add(this.levelLabel.sprite);

    // Laptop and floating desk.
    this.screenCanvas.width = 256;
    this.screenCanvas.height = 160;
    this.screenTexture = new THREE.CanvasTexture(this.screenCanvas);
    this.screenTexture.colorSpace = THREE.SRGBColorSpace;
    const metal = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.6 });
    const desk = new THREE.Mesh(new THREE.BoxGeometry(0.42, 0.018, 0.2), new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.5 }));
    this.laptop.add(desk);
    const base = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.012, 0.13), metal);
    base.position.set(0, 0.015, 0.0);
    this.laptop.add(base);
    const keys = new THREE.Mesh(new THREE.PlaneGeometry(0.17, 0.08), new THREE.MeshBasicMaterial({ color: this.accentColor, transparent: true, opacity: 0.55 }));
    keys.rotation.x = -Math.PI / 2;
    keys.position.set(0, 0.0215, 0.01);
    this.laptop.add(keys);
    const lid = new THREE.Group();
    lid.position.set(0, 0.02, -0.062);
    lid.rotation.x = -0.32;
    const lidBody = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.13, 0.008), metal);
    lidBody.position.y = 0.065;
    lid.add(lidBody);
    const screen = new THREE.Mesh(new THREE.PlaneGeometry(0.185, 0.115), new THREE.MeshBasicMaterial({ map: this.screenTexture }));
    screen.position.set(0, 0.065, -0.0045);
    screen.rotation.y = Math.PI;
    lid.add(screen);
    lid.rotation.y = 0;
    this.laptop.add(lid);
    this.laptop.visible = false;
    this.group.add(this.laptop);

    // Thought bubbles.
    const bubbleMaterial = new THREE.MeshBasicMaterial({ color: 0xf8fafc, transparent: true, opacity: 0.9 });
    [0.018, 0.028, 0.045].forEach((radius, index) => {
      const bubble = new THREE.Mesh(new THREE.SphereGeometry(radius, 16, 12), bubbleMaterial);
      bubble.position.set(0.08 + index * 0.05, index * 0.06, 0.05);
      this.thought.add(bubble);
    });
    this.thought.visible = false;
    this.group.add(this.thought);

    // Floating glyph and sleep Zs.
    this.glyph.sprite.visible = false;
    this.group.add(this.glyph.sprite);
    for (let i = 0; i < SLEEP_Z_COUNT; i += 1) {
      const z = createTextSprite(128, 128, 0.07 + i * 0.02);
      drawText(z, "Z", "#c7d2fe", false);
      z.sprite.visible = false;
      this.sleepZ.push(z);
      this.group.add(z.sprite);
    }

    // Food bowl.
    const bowl = new THREE.Mesh(
      new THREE.CylinderGeometry(0.06, 0.04, 0.035, 24, 1, true),
      new THREE.MeshStandardMaterial({ color: 0xf8fafc, roughness: 0.3, side: THREE.DoubleSide }),
    );
    this.food.add(bowl);
    const foodMaterial = new THREE.MeshStandardMaterial({ color: 0xf59e0b, roughness: 0.6 });
    for (let i = 0; i < 5; i += 1) {
      const morsel = new THREE.Mesh(new THREE.SphereGeometry(0.018, 10, 8), foodMaterial);
      morsel.position.set(Math.cos(i * 1.3) * 0.025, 0.015, Math.sin(i * 1.3) * 0.025);
      this.food.add(morsel);
    }
    this.food.position.set(0.24, 0.02, 0.2);
    this.food.visible = false;
    this.group.add(this.food);

    // Level-up aura.
    this.aura = new THREE.Mesh(
      new THREE.TorusGeometry(0.3, 0.012, 8, 64),
      new THREE.MeshBasicMaterial({ color: 0xfde047, transparent: true, opacity: 0.8 }),
    );
    this.aura.rotation.x = Math.PI / 2;
    this.aura.visible = false;
    this.group.add(this.aura);

    // Confetti.
    const positions = new Float32Array(CONFETTI_COUNT * 3);
    const colors = new Float32Array(CONFETTI_COUNT * 3);
    const color = new THREE.Color();
    for (let i = 0; i < CONFETTI_COUNT; i += 1) {
      color.set(CONFETTI_COLORS[i % CONFETTI_COLORS.length]);
      colors.set([color.r, color.g, color.b], i * 3);
      positions[i * 3 + 1] = -10;
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    this.confetti = new THREE.Points(geometry, new THREE.PointsMaterial({ size: 0.022, vertexColors: true }));
    this.confetti.frustumCulled = false;
    this.group.add(this.confetti);
  }

  setLayout(layout: PropLayout): void {
    this.layout = layout;
    this.laptop.position.set(0, layout.deskY - 0.03, 0.24);
    this.thought.position.set(0.1, layout.headTopY + 0.02, 0);
    this.glyph.sprite.position.set(0, layout.headTopY + 0.12, 0);
    this.aura.position.y = 0;
  }

  setAccent(accent: string): void {
    this.accentColor.set(accent);
    this.ringMaterial.color.copy(this.accentColor);
  }

  /** Shows real progression: arc = XP into level / XP needed for the next level. */
  setHud(hud: AvatarHud | null): void {
    if (!hud) {
      this.levelLabel.sprite.visible = false;
      this.updateRing(0);
      return;
    }
    const span = hud.xpIntoLevel + hud.xpToNextLevel;
    const fraction = span > 0 ? Math.min(1, Math.max(0, hud.xpIntoLevel / span)) : 0;
    this.updateRing(fraction);
    drawText(this.levelLabel, `LV ${hud.level}  ·  ${hud.xpIntoLevel}/${span} XP`, "#e2e8f0", true);
    this.levelLabel.sprite.visible = true;
  }

  private updateRing(fraction: number): void {
    if (Math.abs(fraction - this.xpFraction) < 0.002) return;
    this.xpFraction = fraction;
    if (this.xpRing) {
      this.group.remove(this.xpRing);
      this.xpRing.geometry.dispose();
      this.xpRing = null;
    }
    if (fraction <= 0) return;
    this.xpRing = new THREE.Mesh(new THREE.TorusGeometry(XP_RING_RADIUS, 0.012, 8, 96, Math.PI * 2 * fraction), this.ringMaterial);
    this.xpRing.rotation.set(Math.PI / 2, 0, Math.PI / 2);
    this.xpRing.position.y = 0.006;
    this.group.add(this.xpRing);
  }

  update(flags: PropFlags, dt: number, time: number): void {
    this.laptop.visible = flags.laptop;
    if (flags.laptop) this.drawScreen(dt);

    this.thought.visible = flags.thought;
    if (flags.thought) {
      this.thought.children.forEach((bubble, index) => {
        bubble.position.y = index * 0.06 + 0.01 * Math.sin(time * 2 + index);
      });
    }

    if (flags.glyph) {
      drawText(this.glyph, flags.glyph, flags.glyphColor, flags.glyph.length > 2);
      this.glyph.sprite.visible = true;
      this.glyph.sprite.position.y = this.layout.headTopY + 0.12 + 0.015 * Math.sin(time * 3);
    } else {
      this.glyph.sprite.visible = false;
    }

    this.sleepZ.forEach((z, index) => {
      z.sprite.visible = flags.sleepZ;
      if (!flags.sleepZ) return;
      const phase = (time * 0.35 + index / SLEEP_Z_COUNT) % 1;
      z.sprite.position.set(0.1 + phase * 0.12, this.layout.headTopY - 0.05 + phase * 0.25, 0.02);
      (z.sprite.material as THREE.SpriteMaterial).opacity = Math.sin(phase * Math.PI);
    });

    this.food.visible = flags.food;

    const aura = flags.glyph === "LEVEL UP";
    this.aura.visible = aura;
    if (aura) {
      const phase = (time * 0.7) % 1;
      this.aura.position.y = phase * (this.layout.headTopY + 0.1);
      (this.aura.material as THREE.MeshBasicMaterial).opacity = 0.85 * (1 - phase);
    }

    this.updateConfetti(flags.confetti, dt);
  }

  private drawScreen(dt: number): void {
    this.screenClock += dt;
    if (this.screenClock < SCREEN_REDRAW_INTERVAL) return;
    this.screenClock = 0;
    this.screenScroll += 1;
    const context = this.screenCanvas.getContext("2d");
    if (!context) return;
    const { width, height } = this.screenCanvas;
    context.fillStyle = "#020617";
    context.fillRect(0, 0, width, height);
    const lineHeight = height / (SCREEN_LINES + 1);
    for (let i = 0; i < SCREEN_LINES; i += 1) {
      const seed = (i + this.screenScroll) * 2654435761;
      const indent = ((seed >>> 3) % 4) * 14;
      const length = 40 + ((seed >>> 7) % 150);
      context.fillStyle = i === SCREEN_LINES - 1 ? `#${this.accentColor.getHexString()}` : ["#38bdf8", "#a5b4fc", "#4ade80", "#e2e8f0"][(seed >>> 11) % 4];
      context.fillRect(14 + indent, lineHeight * (i + 0.6), Math.min(length, width - 28 - indent), lineHeight * 0.45);
    }
    this.screenTexture.needsUpdate = true;
  }

  private updateConfetti(active: boolean, dt: number): void {
    const attribute = this.confetti.geometry.getAttribute("position") as THREE.BufferAttribute;
    const positions = attribute.array as Float32Array;
    const top = this.layout.headTopY + 0.25;
    if (active && !this.confettiActive) {
      for (let i = 0; i < CONFETTI_COUNT; i += 1) this.spawnConfetti(positions, i, top, true);
    }
    this.confettiActive = active;
    let anyVisible = false;
    for (let i = 0; i < CONFETTI_COUNT; i += 1) {
      const y = i * 3 + 1;
      if (positions[y] < -1) continue;
      this.confettiVelocity[y] -= CONFETTI_GRAVITY * dt;
      positions[i * 3] += this.confettiVelocity[i * 3] * dt;
      positions[y] += this.confettiVelocity[y] * dt;
      positions[i * 3 + 2] += this.confettiVelocity[i * 3 + 2] * dt;
      if (positions[y] < 0) {
        if (active) this.spawnConfetti(positions, i, top, false);
        else positions[y] = -10;
      }
      anyVisible = true;
    }
    this.confetti.visible = anyVisible;
    attribute.needsUpdate = true;
  }

  private spawnConfetti(positions: Float32Array, index: number, top: number, burst: boolean): void {
    const angle = Math.random() * Math.PI * 2;
    const radius = Math.random() * 0.15;
    positions.set([Math.cos(angle) * radius, top - (burst ? Math.random() * 0.3 : 0), Math.sin(angle) * radius], index * 3);
    this.confettiVelocity.set(
      [Math.cos(angle) * (0.15 + Math.random() * 0.35), burst ? 0.6 + Math.random() * 0.6 : Math.random() * 0.3, Math.sin(angle) * (0.15 + Math.random() * 0.35)],
      index * 3,
    );
  }
}
