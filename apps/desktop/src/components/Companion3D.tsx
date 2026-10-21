import { useEffect, useRef } from "react";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRMLoaderPlugin, VRM, VRMUtils } from "@pixiv/three-vrm";
import type { PetState } from "./CompanionRenderer";

export type { PetState };

export function disposeMaterial(material: THREE.Material): void {
  for (const key of Object.keys(material)) {
    const value = (material as unknown as Record<string, unknown>)[key];
    if (value && typeof value === "object" && "isTexture" in value && (value as THREE.Texture).isTexture) {
      (value as THREE.Texture).dispose();
    }
  }
  material.dispose();
}

export function disposeScene(scene: THREE.Scene, renderer: THREE.WebGLRenderer, vrm: VRM | null = null): void {
  if (vrm) {
    try {
      VRMUtils.deepDispose(vrm.scene);
    } catch {
      // ignore
    }
  }

  scene.traverse((object) => {
    if ((object as THREE.Mesh).isMesh || (object as THREE.Points).isPoints) {
      const mesh = object as THREE.Mesh;
      if (mesh.geometry) {
        mesh.geometry.dispose();
      }
      if (mesh.material) {
        if (Array.isArray(mesh.material)) {
          mesh.material.forEach((mat) => disposeMaterial(mat));
        } else {
          disposeMaterial(mesh.material);
        }
      }
    }
  });

  renderer.dispose();
  try {
    renderer.forceContextLoss();
  } catch {
    // context may already be lost
  }
}

export interface Companion3DProps {
  state: PetState;
  petName?: string;
  imageUrl?: string | null;
  vrmUrl?: string | null;
  size?: number;
  interactive?: boolean;
  onDoubleClick?: () => void;
  onError?: (err: string) => void;
}

export function Companion3D({
  state,
  petName = "Nezuko",
  vrmUrl,
  size = 240,
  interactive = true,
  onDoubleClick,
  onError,
}: Companion3DProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const stateRef = useRef<PetState>(state);
  stateRef.current = state;

  const petNameLower = petName.toLowerCase();
  const isNezuko = petNameLower.includes("nezuko");
  const isWukong = petNameLower.includes("wukong") || petNameLower.includes("monkey");
  const isArjuna = petNameLower.includes("arjuna");
  const isCat = petNameLower.includes("cat") || petNameLower.includes("ninja");
  const isAoi = petNameLower.includes("aoi");

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // 1. Scene & Camera setup with transparent alpha
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 100);
    camera.position.set(0, 0.72, 2.35);
    camera.lookAt(0, 0.38, 0);

    const renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: "high-performance",
    });
    renderer.setSize(size, size);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    container.innerHTML = "";
    container.appendChild(renderer.domElement);

    // 2. Dynamic Studio Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 1.4);
    scene.add(ambientLight);

    const dirLight = new THREE.DirectionalLight(0xffffff, 1.8);
    dirLight.position.set(2, 4, 3);
    scene.add(dirLight);

    const rimColor = isNezuko
      ? 0xff70a6
      : isWukong
      ? 0xffb703
      : isArjuna
      ? 0x38bdf8
      : isCat
      ? 0x22c55e
      : 0x00f0ff;

    const rimLight = new THREE.DirectionalLight(rimColor, 1.6);
    rimLight.position.set(-2.5, 2.5, -2);
    scene.add(rimLight);

    const bottomGlow = new THREE.PointLight(rimColor, 1.3, 5);
    bottomGlow.position.set(0, -0.15, 0.7);
    scene.add(bottomGlow);

    // 3. Root Companion Group
    const companionGroup = new THREE.Group();
    scene.add(companionGroup);

    // Pedestal Base (Illuminated 3D Cyber Platform)
    const baseGeo = new THREE.CylinderGeometry(0.55, 0.62, 0.05, 32);
    const baseMat = new THREE.MeshStandardMaterial({
      color: 0x0a101d,
      roughness: 0.3,
      metalness: 0.8,
    });
    const baseMesh = new THREE.Mesh(baseGeo, baseMat);
    baseMesh.position.y = -0.05;
    companionGroup.add(baseMesh);

    // Cyber Rings around pedestal
    const ringGeo = new THREE.TorusGeometry(0.58, 0.015, 12, 32);
    const ringMat = new THREE.MeshStandardMaterial({
      color: rimColor,
      emissive: rimColor,
      emissiveIntensity: 0.85,
    });
    const ringMesh = new THREE.Mesh(ringGeo, ringMat);
    ringMesh.rotation.x = Math.PI / 2;
    ringMesh.position.y = -0.03;
    companionGroup.add(ringMesh);

    // 4. Character Master Group
    const charGroup = new THREE.Group();
    companionGroup.add(charGroup);

    // Color palettes per character
    const skinMat = new THREE.MeshStandardMaterial({
      color: isCat ? 0x2d3748 : 0xffe2d5,
      roughness: 0.55,
    });
    const eyeMat = new THREE.MeshStandardMaterial({
      color: isNezuko ? 0xd946ef : isWukong ? 0xf59e0b : isArjuna ? 0x2563eb : isCat ? 0x10b981 : 0x06b6d4,
      roughness: 0.2,
      emissive: isNezuko ? 0xa21caf : isWukong ? 0xd97706 : isArjuna ? 0x1d4ed8 : isCat ? 0x059669 : 0x0891b2,
      emissiveIntensity: 0.4,
    });
    const eyeHighlightMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
    const blushMat = new THREE.MeshBasicMaterial({
      color: 0xf43f5e,
      transparent: true,
      opacity: 0.55,
    });
    const mainOutfitMat = new THREE.MeshStandardMaterial({
      color: isNezuko
        ? 0x181116
        : isWukong
        ? 0xb91c1c
        : isArjuna
        ? 0x0f172a
        : isCat
        ? 0x111827
        : 0x0f2b3e,
      roughness: 0.4,
    });
    const accentMat = new THREE.MeshStandardMaterial({
      color: rimColor,
      emissive: rimColor,
      emissiveIntensity: 0.85,
      roughness: 0.2,
    });
    const hairMat = new THREE.MeshStandardMaterial({
      color: isNezuko
        ? 0x120c11
        : isWukong
        ? 0x92400e
        : isArjuna
        ? 0x1e293b
        : isCat
        ? 0x374151
        : 0x1e293b,
      roughness: 0.45,
    });
    const goldMat = new THREE.MeshStandardMaterial({
      color: 0xfbbf24,
      metalness: 0.9,
      roughness: 0.25,
      emissive: 0xd97706,
      emissiveIntensity: 0.35,
    });
    const deskMat = new THREE.MeshStandardMaterial({
      color: 0x0b131e,
      roughness: 0.3,
      metalness: 0.85,
    });
    const holoScreenMat = new THREE.MeshBasicMaterial({
      color: rimColor,
      transparent: true,
      opacity: 0.8,
      side: THREE.DoubleSide,
    });

    // 5. Torso & Sash
    const torsoGeo = new THREE.CylinderGeometry(0.18, 0.15, 0.42, 16);
    const torso = new THREE.Mesh(torsoGeo, mainOutfitMat);
    torso.position.y = 0.35;
    charGroup.add(torso);

    const beltGeo = new THREE.CylinderGeometry(0.185, 0.185, 0.08, 16);
    const belt = new THREE.Mesh(beltGeo, accentMat);
    belt.position.y = 0.32;
    charGroup.add(belt);

    // 6. Head Group
    const headGroup = new THREE.Group();
    headGroup.position.set(0, 0.68, 0);
    charGroup.add(headGroup);

    const headGeo = new THREE.SphereGeometry(0.22, 24, 24);
    const head = new THREE.Mesh(headGeo, skinMat);
    headGroup.add(head);

    // Hair Dome
    const hairGeo = new THREE.SphereGeometry(0.23, 16, 16, 0, Math.PI * 2, 0, Math.PI * 0.6);
    const hair = new THREE.Mesh(hairGeo, hairMat);
    hair.position.set(0, 0.04, -0.02);
    headGroup.add(hair);

    // Anime Eyes & Sparkles
    const eyeGeo = new THREE.SphereGeometry(0.038, 12, 12);
    const leftEye = new THREE.Mesh(eyeGeo, eyeMat);
    leftEye.position.set(-0.08, 0.02, 0.19);
    leftEye.scale.set(1, 1.25, 0.4);
    headGroup.add(leftEye);

    const rightEye = new THREE.Mesh(eyeGeo, eyeMat);
    rightEye.position.set(0.08, 0.02, 0.19);
    rightEye.scale.set(1, 1.25, 0.4);
    headGroup.add(rightEye);

    // Eye Sparkle Dots
    const sparkGeo = new THREE.SphereGeometry(0.012, 8, 8);
    const leftSpark = new THREE.Mesh(sparkGeo, eyeHighlightMat);
    leftSpark.position.set(-0.07, 0.04, 0.21);
    headGroup.add(leftSpark);

    const rightSpark = new THREE.Mesh(sparkGeo, eyeHighlightMat);
    rightSpark.position.set(0.09, 0.04, 0.21);
    headGroup.add(rightSpark);

    // Cheek Blushes
    const blushGeo = new THREE.PlaneGeometry(0.05, 0.025);
    const leftBlush = new THREE.Mesh(blushGeo, blushMat);
    leftBlush.position.set(-0.11, -0.04, 0.195);
    leftBlush.rotation.y = -0.3;
    headGroup.add(leftBlush);

    const rightBlush = new THREE.Mesh(blushGeo, blushMat);
    rightBlush.position.set(0.11, -0.04, 0.195);
    rightBlush.rotation.y = 0.3;
    headGroup.add(rightBlush);

    // 7. Character Specific Regalia & Props
    let tailGroup: THREE.Group | null = null;
    let staffMesh: THREE.Mesh | null = null;

    if (isNezuko) {
      // Bamboo Mouthpiece with cord ties
      const bambooGeo = new THREE.CylinderGeometry(0.03, 0.03, 0.22, 12);
      const bambooMat = new THREE.MeshStandardMaterial({
        color: 0x22c55e,
        roughness: 0.3,
      });
      const bamboo = new THREE.Mesh(bambooGeo, bambooMat);
      bamboo.rotation.z = Math.PI / 2;
      bamboo.position.set(0, -0.04, 0.2);
      headGroup.add(bamboo);

      // Red tying cord
      const cordGeo = new THREE.TorusGeometry(0.19, 0.008, 6, 20);
      const cordMat = new THREE.MeshStandardMaterial({ color: 0xef4444 });
      const cord = new THREE.Mesh(cordGeo, cordMat);
      cord.position.set(0, -0.04, 0.04);
      headGroup.add(cord);

      // Pink Hair Ribbon
      const ribbonGeo = new THREE.BoxGeometry(0.08, 0.08, 0.04);
      const ribbon = new THREE.Mesh(ribbonGeo, accentMat);
      ribbon.position.set(-0.16, 0.14, 0.12);
      headGroup.add(ribbon);

      // Long Hair Back Falls with orange tips
      const hairFallGeo = new THREE.CylinderGeometry(0.18, 0.26, 0.55, 12);
      const hairFall = new THREE.Mesh(hairFallGeo, hairMat);
      hairFall.position.set(0, -0.15, -0.12);
      hairFall.rotation.x = 0.2;
      headGroup.add(hairFall);
    } else if (isWukong) {
      // Golden Circlet (Fillet Headband)
      const circletGeo = new THREE.TorusGeometry(0.21, 0.02, 8, 24);
      const circlet = new THREE.Mesh(circletGeo, goldMat);
      circlet.rotation.x = Math.PI / 2 + 0.1;
      circlet.position.set(0, 0.06, 0);
      headGroup.add(circlet);

      // Crown Feathers
      const featherGeo = new THREE.ConeGeometry(0.03, 0.3, 8);
      const featherMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3 });
      const leftFeather = new THREE.Mesh(featherGeo, featherMat);
      leftFeather.position.set(-0.06, 0.32, -0.04);
      leftFeather.rotation.z = -0.3;
      headGroup.add(leftFeather);

      const rightFeather = new THREE.Mesh(featherGeo, featherMat);
      rightFeather.position.set(0.06, 0.32, -0.04);
      rightFeather.rotation.z = 0.3;
      headGroup.add(rightFeather);

      // Ruyi Jingu Bang Staff resting at desk
      const staffGeo = new THREE.CylinderGeometry(0.022, 0.022, 0.95, 12);
      const staffMat = new THREE.MeshStandardMaterial({
        color: 0x991b1b,
        metalness: 0.6,
        roughness: 0.3,
      });
      staffMesh = new THREE.Mesh(staffGeo, staffMat);
      staffMesh.position.set(0.38, 0.35, 0.15);
      staffMesh.rotation.z = -0.22;
      companionGroup.add(staffMesh);

      // Staff Gold Tips
      const tipGeo = new THREE.CylinderGeometry(0.026, 0.026, 0.16, 12);
      const topTip = new THREE.Mesh(tipGeo, goldMat);
      topTip.position.y = 0.4;
      staffMesh.add(topTip);

      const btmTip = new THREE.Mesh(tipGeo, goldMat);
      btmTip.position.y = -0.4;
      staffMesh.add(btmTip);

      // Monkey Tail
      tailGroup = new THREE.Group();
      tailGroup.position.set(0, 0.2, -0.15);
      charGroup.add(tailGroup);
      const tailGeo = new THREE.CylinderGeometry(0.025, 0.015, 0.35, 8);
      const tailMesh = new THREE.Mesh(tailGeo, hairMat);
      tailMesh.rotation.x = -1.2;
      tailMesh.position.set(0, 0.12, -0.12);
      tailGroup.add(tailMesh);
    } else if (isArjuna) {
      // Warrior Forehead Tilak
      const tilakGeo = new THREE.BoxGeometry(0.02, 0.06, 0.01);
      const tilakMat = new THREE.MeshBasicMaterial({ color: 0xef4444 });
      const tilak = new THREE.Mesh(tilakGeo, tilakMat);
      tilak.position.set(0, 0.1, 0.215);
      headGroup.add(tilak);

      // Gandiva Bow on back
      const bowGeo = new THREE.TorusGeometry(0.38, 0.018, 8, 24, Math.PI * 0.9);
      const bow = new THREE.Mesh(bowGeo, goldMat);
      bow.position.set(0, 0.45, -0.16);
      bow.rotation.z = 0.6;
      charGroup.add(bow);

      // Archer Shoulder Pauldron
      const pauldronGeo = new THREE.SphereGeometry(0.08, 12, 12, 0, Math.PI);
      const pauldron = new THREE.Mesh(pauldronGeo, goldMat);
      pauldron.position.set(-0.25, 0.48, 0);
      pauldron.rotation.x = -Math.PI / 2;
      charGroup.add(pauldron);
    } else if (isCat) {
      // Cat Ears with Pink Inner Ear
      const earGeo = new THREE.ConeGeometry(0.08, 0.15, 4);
      const earInnerGeo = new THREE.ConeGeometry(0.05, 0.11, 4);
      const earInnerMat = new THREE.MeshBasicMaterial({ color: 0xf43f5e });

      const leftEar = new THREE.Mesh(earGeo, hairMat);
      leftEar.position.set(-0.14, 0.24, 0);
      leftEar.rotation.z = -0.35;
      headGroup.add(leftEar);

      const leftInner = new THREE.Mesh(earInnerGeo, earInnerMat);
      leftInner.position.set(0, 0.01, 0.02);
      leftEar.add(leftInner);

      const rightEar = new THREE.Mesh(earGeo, hairMat);
      rightEar.position.set(0.14, 0.24, 0);
      rightEar.rotation.z = 0.35;
      headGroup.add(rightEar);

      const rightInner = new THREE.Mesh(earInnerGeo, earInnerMat);
      rightInner.position.set(0, 0.01, 0.02);
      rightEar.add(rightInner);

      // Whiskers
      const whiskerGeo = new THREE.CylinderGeometry(0.003, 0.003, 0.12);
      const whiskerMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
      const w1 = new THREE.Mesh(whiskerGeo, whiskerMat);
      w1.rotation.z = Math.PI / 2 + 0.15;
      w1.position.set(-0.15, -0.03, 0.16);
      headGroup.add(w1);

      const w2 = new THREE.Mesh(whiskerGeo, whiskerMat);
      w2.rotation.z = Math.PI / 2 - 0.15;
      w2.position.set(0.15, -0.03, 0.16);
      headGroup.add(w2);

      // Cat Tail
      tailGroup = new THREE.Group();
      tailGroup.position.set(0, 0.2, -0.15);
      charGroup.add(tailGroup);
      const tailGeo = new THREE.CylinderGeometry(0.022, 0.016, 0.38, 8);
      const tailMesh = new THREE.Mesh(tailGeo, hairMat);
      tailMesh.rotation.x = -1.1;
      tailMesh.position.set(0, 0.14, -0.14);
      tailGroup.add(tailMesh);
    } else {
      // Aoi / Cyber Streetwear: Visor & Headphones
      const visorGeo = new THREE.BoxGeometry(0.26, 0.06, 0.12);
      const visor = new THREE.Mesh(visorGeo, accentMat);
      visor.position.set(0, 0.02, 0.18);
      headGroup.add(visor);

      // Headphones
      const phoneGeo = new THREE.CylinderGeometry(0.07, 0.07, 0.04, 16);
      const leftPhone = new THREE.Mesh(phoneGeo, deskMat);
      leftPhone.rotation.z = Math.PI / 2;
      leftPhone.position.set(-0.22, 0.02, 0);
      headGroup.add(leftPhone);

      const rightPhone = new THREE.Mesh(phoneGeo, deskMat);
      rightPhone.rotation.z = Math.PI / 2;
      rightPhone.position.set(0.22, 0.02, 0);
      headGroup.add(rightPhone);
    }

    // 8. Articulated Arms & Hands
    const armGeo = new THREE.CylinderGeometry(0.045, 0.04, 0.3, 12);
    const leftArmGroup = new THREE.Group();
    leftArmGroup.position.set(-0.24, 0.48, 0);
    charGroup.add(leftArmGroup);
    const leftArm = new THREE.Mesh(armGeo, mainOutfitMat);
    leftArm.position.y = -0.15;
    leftArmGroup.add(leftArm);

    const rightArmGroup = new THREE.Group();
    rightArmGroup.position.set(0.24, 0.48, 0);
    charGroup.add(rightArmGroup);
    const rightArm = new THREE.Mesh(armGeo, mainOutfitMat);
    rightArm.position.y = -0.15;
    rightArmGroup.add(rightArm);

    const handGeo = new THREE.SphereGeometry(0.045, 12, 12);
    const leftHand = new THREE.Mesh(handGeo, skinMat);
    leftHand.position.set(0, -0.3, 0);
    leftArmGroup.add(leftHand);

    const rightHand = new THREE.Mesh(handGeo, skinMat);
    rightHand.position.set(0, -0.3, 0);
    rightArmGroup.add(rightHand);

    // 9. Holographic Coding Workstation (Desk + Glowing Laptop)
    const deskGroup = new THREE.Group();
    deskGroup.position.set(0, 0.16, 0.35);
    companionGroup.add(deskGroup);

    // Cyber Desk Base
    const deskTop = new THREE.Mesh(new THREE.BoxGeometry(0.7, 0.025, 0.36), deskMat);
    deskGroup.add(deskTop);

    const deskEdge = new THREE.Mesh(new THREE.BoxGeometry(0.72, 0.012, 0.38), accentMat);
    deskGroup.add(deskEdge);

    // Laptop Base / Glowing Keyboard
    const laptopBase = new THREE.Mesh(new THREE.BoxGeometry(0.28, 0.018, 0.18), deskMat);
    laptopBase.position.set(0, 0.02, 0.04);
    deskGroup.add(laptopBase);

    const keyboardGlow = new THREE.Mesh(new THREE.BoxGeometry(0.24, 0.005, 0.12), accentMat);
    keyboardGlow.position.set(0, 0.032, 0.04);
    deskGroup.add(keyboardGlow);

    // Floating Hologram Screen
    const screenGroup = new THREE.Group();
    screenGroup.position.set(0, 0.04, -0.05);
    deskGroup.add(screenGroup);

    const holoScreen = new THREE.Mesh(new THREE.PlaneGeometry(0.32, 0.2), holoScreenMat);
    holoScreen.position.set(0, 0.11, 0);
    holoScreen.rotation.x = -0.15;
    screenGroup.add(holoScreen);

    // Code lines on Hologram screen
    const lineMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
    for (let i = 0; i < 4; i++) {
      const codeLine = new THREE.Mesh(
        new THREE.PlaneGeometry(0.12 + (i % 2) * 0.08, 0.01),
        lineMat
      );
      codeLine.position.set(-0.03, 0.15 - i * 0.03, 0.002);
      codeLine.rotation.x = -0.15;
      screenGroup.add(codeLine);
    }

    // 10. Floating Sparkle Particles
    const particleCount = 35;
    const particleGeo = new THREE.BufferGeometry();
    const particlePositions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i++) {
      particlePositions[i] = (Math.random() - 0.5) * 1.5;
    }
    particleGeo.setAttribute("position", new THREE.BufferAttribute(particlePositions, 3));
    const particleMat = new THREE.PointsMaterial({
      size: 0.035,
      color: rimColor,
      transparent: true,
      opacity: 0.85,
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    companionGroup.add(particles);

    // 11. External VRM Loader if URL provided
    let currentVrm: VRM | null = null;
    if (vrmUrl) {
      const loader = new GLTFLoader();
      loader.register((parser) => new VRMLoaderPlugin(parser));
      loader.load(
        vrmUrl,
        (gltf) => {
          const vrm = gltf.userData.vrm as VRM;
          if (vrm) {
            currentVrm = vrm;
            vrm.scene.scale.set(0.7, 0.7, 0.7);
            vrm.scene.position.set(0, 0, 0);
            charGroup.visible = false;
            companionGroup.add(vrm.scene);
          }
        },
        undefined,
        (err) => {
          if (onError) onError(`VRM load failed: ${err}`);
        }
      );
    }

    // 12. Interactive Mouse Drag to Orbit in 3D
    let mouseX = 0;
    let mouseY = 0;
    let isDragging = false;
    let prevMouseX = 0;
    let prevMouseY = 0;

    const onMouseMove = (e: MouseEvent) => {
      const rect = container.getBoundingClientRect();
      const x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      const y = -(((e.clientY - rect.top) / rect.height) * 2 - 1);
      mouseX = x;
      mouseY = y;

      if (isDragging && interactive) {
        const deltaX = e.clientX - prevMouseX;
        const deltaY = e.clientY - prevMouseY;
        companionGroup.rotation.y += deltaX * 0.018;
        camera.position.y = Math.max(0.3, Math.min(1.4, camera.position.y + deltaY * 0.005));
        camera.lookAt(0, 0.38, 0);
        prevMouseX = e.clientX;
        prevMouseY = e.clientY;
      }
    };

    const onMouseDown = (e: MouseEvent) => {
      isDragging = true;
      prevMouseX = e.clientX;
      prevMouseY = e.clientY;
    };

    const onMouseUp = () => {
      isDragging = false;
    };

    if (interactive) {
      container.addEventListener("mousemove", onMouseMove);
      container.addEventListener("mousedown", onMouseDown);
      window.addEventListener("mouseup", onMouseUp);
    }

    // 13. High-Framerate Procedural Animation Loop
    const clock = new THREE.Clock();
    let animId = 0;

    const animate = () => {
      animId = requestAnimationFrame(animate);
      const delta = clock.getDelta();
      const time = clock.getElapsedTime();
      const currentState = stateRef.current;

      if (currentVrm) {
        currentVrm.update(delta);
      }

      // Smooth Head Tracking toward cursor when awake
      if (currentState !== "sleeping") {
        headGroup.rotation.y = THREE.MathUtils.lerp(headGroup.rotation.y, mouseX * 0.45, 0.08);
        headGroup.rotation.x = THREE.MathUtils.lerp(headGroup.rotation.x, -mouseY * 0.25, 0.08);
      }

      // Tail swish animation
      if (tailGroup) {
        tailGroup.rotation.y = Math.sin(time * 3) * 0.35;
      }

      // Staff subtle levitation
      if (staffMesh) {
        staffMesh.position.y = 0.35 + Math.sin(time * 2) * 0.015;
      }

      // State-specific procedural 3D motion
      switch (currentState) {
        case "working":
          // Realistic typing motion on glowing laptop keyboard
          deskGroup.visible = true;
          leftArmGroup.rotation.x = -1.1 + Math.sin(time * 18) * 0.15;
          rightArmGroup.rotation.x = -1.1 + Math.cos(time * 18) * 0.15;
          leftArmGroup.rotation.z = 0.3;
          rightArmGroup.rotation.z = -0.3;

          headGroup.position.y = 0.68 + Math.sin(time * 6) * 0.01;
          headGroup.rotation.x = 0.18 + Math.sin(time * 12) * 0.03;
          torso.rotation.x = 0.08;
          holoScreenMat.opacity = 0.75 + Math.sin(time * 8) * 0.15;
          break;

        case "thinking":
          // Thoughtful lean back with chin touch
          deskGroup.visible = true;
          rightArmGroup.rotation.x = -1.65;
          rightArmGroup.rotation.z = -0.45;
          leftArmGroup.rotation.x = -0.3;
          leftArmGroup.rotation.z = 0.2;

          headGroup.rotation.x = -0.15 + Math.sin(time * 2) * 0.08;
          headGroup.rotation.z = 0.12;
          torso.rotation.x = -0.05;
          break;

        case "waiting":
        case "attention_required":
          deskGroup.visible = true;
          leftArmGroup.rotation.x = -0.2;
          rightArmGroup.rotation.x = -0.2 + Math.sin(time * 3) * 0.1;
          rightArmGroup.rotation.z = -0.3;
          headGroup.rotation.y = Math.sin(time * 2) * 0.2;
          break;

        case "success":
        case "celebrating":
        case "level_up":
        case "achievement":
          // Joyous double fist pump & victory float
          deskGroup.visible = false;
          leftArmGroup.rotation.x = -2.7;
          rightArmGroup.rotation.x = -2.7;
          leftArmGroup.rotation.z = -0.5;
          rightArmGroup.rotation.z = 0.5;

          charGroup.position.y = Math.abs(Math.sin(time * 8)) * 0.25;
          charGroup.rotation.y = Math.sin(time * 4) * 0.3;
          break;

        case "error":
          deskGroup.visible = true;
          leftArmGroup.rotation.x = -1.8;
          rightArmGroup.rotation.x = -1.8;
          leftArmGroup.rotation.z = 0.8;
          rightArmGroup.rotation.z = -0.8;

          headGroup.rotation.y = Math.sin(time * 24) * 0.25;
          headGroup.position.y = 0.64;
          holoScreenMat.color.setHex(0xef4444);
          break;

        case "sleeping":
          // Peaceful slumber slumped on desk
          deskGroup.visible = true;
          holoScreenMat.opacity = 0.2;

          leftArmGroup.rotation.x = -1.1;
          rightArmGroup.rotation.x = -1.1;
          torso.rotation.x = 0.3;
          headGroup.rotation.x = 0.4;
          headGroup.position.y = 0.6;
          charGroup.position.y = -0.05 + Math.sin(time * 1.5) * 0.015;
          break;

        case "idle":
        default:
          // Organic breathing & floating idle sway
          deskGroup.visible = true;
          const breathe = Math.sin(time * 2.2);
          charGroup.position.y = breathe * 0.025;
          torso.scale.y = 1 + breathe * 0.02;
          headGroup.position.y = 0.68 + breathe * 0.02;

          leftArmGroup.rotation.x = -0.4 + breathe * 0.05;
          rightArmGroup.rotation.x = -0.4 + breathe * 0.05;
          leftArmGroup.rotation.z = 0.2;
          rightArmGroup.rotation.z = -0.2;
          break;
      }

      // Continuous subtle ambient dynamics
      particles.rotation.y = time * 0.12;
      ringMesh.rotation.z = time * 0.35;

      renderer.render(scene, camera);
    };

    animate();

    return () => {
      cancelAnimationFrame(animId);
      if (interactive) {
        container.removeEventListener("mousemove", onMouseMove);
        container.removeEventListener("mousedown", onMouseDown);
        window.removeEventListener("mouseup", onMouseUp);
      }
      disposeScene(scene, renderer, currentVrm);
      container.innerHTML = "";
    };
  }, [interactive, isAoi, isArjuna, isCat, isNezuko, isWukong, onError, size, vrmUrl]);

  return (
    <div
      ref={containerRef}
      onDoubleClick={onDoubleClick}
      title={interactive ? "3D Living Avatar · Drag mouse to spin 360° · Double-click for Command Deck" : undefined}
      style={{
        width: size,
        height: size,
        cursor: interactive ? "grab" : "default",
        userSelect: "none",
        position: "relative",
      }}
    />
  );
}
