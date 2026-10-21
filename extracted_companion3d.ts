import { useEffect, useRef } from "react";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRMLoaderPlugin, VRM } from "@pixiv/three-vrm";

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

interface Companion3DProps {
  state: PetState;
  vrmUrl?: string | null;
  interactive?: boolean;
  size?: number;
  onModelLoaded?: (title: string) => void;
  onError?: (err: string) => void;
}

export function Companion3D({
  state,
  vrmUrl,
  interactive = true,
  size = 220,
  onModelLoaded,
  onError,
}: Companion3DProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const stateRef = useRef<PetState>(state);
  stateRef.current = state;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // 1. Scene setup with transparent alpha
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 100);
    camera.position.set(0, 0.8, 2.4);
    camera.lookAt(0, 0.45, 0);

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

    // 2. Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 1.4);
    scene.add(ambientLight);

    const dirLight = new THREE.DirectionalLight(0x70c0ff, 2.0);
    dirLight.position.set(2, 4, 3);
    scene.add(dirLight);

    const rimLight = new THREE.DirectionalLight(0xff70c0, 1.5);
    rimLight.position.set(-2, 2, -2);
    scene.add(rimLight);

    const bottomGlow = new THREE.PointLight(0x00ffcc, 1.2, 5);
    bottomGlow.position.set(0, -0.2, 0.8);
    scene.add(bottomGlow);

    // 3. Procedural Cyber 3D Companion Avatar & Desk Rig
    const companionGroup = new THREE.Group();
    scene.add(companionGroup);

    // Character Group
    const charGroup = new THREE.Group();
    companionGroup.add(charGroup);

    // Materials
    const skinMat = new THREE.MeshStandardMaterial({
      color: 0xffdfd0,
      roughness: 0.5,
    });
    const hairMat = new THREE.MeshStandardMaterial({
      color: 0x22384f,
      roughness: 0.3,
      metalness: 0.2,
    });
    const hoodieMat = new THREE.MeshStandardMaterial({
      color: 0x0f2b3e,
      roughness: 0.4,
    });
    const cyberTealMat = new THREE.MeshStandardMaterial({
      color: 0x00f0ff,
      emissive: 0x00a0cc,
      emissiveIntensity: 0.8,
      roughness: 0.2,
    });
    const cyberGoldMat = new THREE.MeshStandardMaterial({
      color: 0xffb700,
      emissive: 0xff8800,
      emissiveIntensity: 0.7,
      roughness: 0.2,
    });
    const deskMat = new THREE.MeshStandardMaterial({
      color: 0x0b131e,
      roughness: 0.3,
      metalness: 0.8,
    });
    const holoScreenMat = new THREE.MeshBasicMaterial({
      color: 0x00ffff,
      transparent: true,
      opacity: 0.75,
      side: THREE.DoubleSide,
    });

    // Body (Tech Hoodie)
    const torsoGeo = new THREE.CylinderGeometry(0.2, 0.16, 0.45, 16);
    const torso = new THREE.Mesh(torsoGeo, hoodieMat);
    torso.position.y = 0.35;
    charGroup.add(torso);

    // Hoodie Circuit Line
    const stripeGeo = new THREE.BoxGeometry(0.04, 0.42, 0.21);
    const stripe = new THREE.Mesh(stripeGeo, cyberTealMat);
    stripe.position.set(0, 0.35, 0.08);
    charGroup.add(stripe);

    // Head Group
    const headGroup = new THREE.Group();
    headGroup.position.set(0, 0.68, 0);
    charGroup.add(headGroup);

    const headGeo = new THREE.SphereGeometry(0.22, 24, 24);
    const head = new THREE.Mesh(headGeo, skinMat);
    headGroup.add(head);

    // Hair / Ponytail
    const hairGeo = new THREE.SphereGeometry(0.23, 16, 16, 0, Math.PI * 2, 0, Math.PI * 0.6);
    const hair = new THREE.Mesh(hairGeo, hairMat);
    hair.position.set(0, 0.04, -0.02);
    headGroup.add(hair);

    const ponytailGeo = new THREE.ConeGeometry(0.12, 0.45, 12);
    const ponytail = new THREE.Mesh(ponytailGeo, hairMat);
    ponytail.position.set(0, 0.15, -0.22);
    ponytail.rotation.x = -Math.PI * 0.75;
    headGroup.add(ponytail);

    // Cyber Visor / AR Glasses
    const visorGeo = new THREE.BoxGeometry(0.28, 0.07, 0.12);
    const visor = new THREE.Mesh(visorGeo, cyberTealMat);
    visor.position.set(0, 0.02, 0.18);
    headGroup.add(visor);

    // Cyber Horns / Antennae
    const hornGeo = new THREE.ConeGeometry(0.03, 0.16, 8);
    const leftHorn = new THREE.Mesh(hornGeo, cyberTealMat);
    leftHorn.position.set(-0.16, 0.2, 0);
    leftHorn.rotation.z = 0.4;
    headGroup.add(leftHorn);

    const rightHorn = new THREE.Mesh(hornGeo, cyberTealMat);
    rightHorn.position.set(0.16, 0.2, 0);
    rightHorn.rotation.z = -0.4;
    headGroup.add(rightHorn);

    // Arms & Hands
    const armGeo = new THREE.CylinderGeometry(0.045, 0.04, 0.3, 12);
    const leftArmGroup = new THREE.Group();
    leftArmGroup.position.set(-0.24, 0.48, 0);
    charGroup.add(leftArmGroup);
    const leftArm = new THREE.Mesh(armGeo, hoodieMat);
    leftArm.position.y = -0.15;
    leftArmGroup.add(leftArm);

    const rightArmGroup = new THREE.Group();
    rightArmGroup.position.set(0.24, 0.48, 0);
    charGroup.add(rightArmGroup);
    const rightArm = new THREE.Mesh(armGeo, hoodieMat);
    rightArm.position.y = -0.15;
    rightArmGroup.add(rightArm);

    // Hands
    const handGeo = new THREE.SphereGeometry(0.045, 12, 12);
    const leftHand = new THREE.Mesh(handGeo, skinMat);
    leftHand.position.set(0, -0.3, 0);
    leftArmGroup.add(leftHand);

    const rightHand = new THREE.Mesh(handGeo, skinMat);
    rightHand.position.set(0, -0.3, 0);
    rightArmGroup.add(rightHand);

    // 4. Holographic Coding Workstation (Desk + Glowing Laptop)
    const deskGroup = new THREE.Group();
    deskGroup.position.set(0, 0.15, 0.35);
    companionGroup.add(deskGroup);

    // Floating Cyber Desk Base
    const deskTop = new THREE.Mesh(new THREE.BoxGeometry(0.7, 0.03, 0.4), deskMat);
    deskGroup.add(deskTop);

    const deskEdge = new THREE.Mesh(new THREE.BoxGeometry(0.72, 0.015, 0.42), cyberTealMat);
    deskGroup.add(deskEdge);

    // Laptop Base / Glowing Keyboard
    const laptopBase = new THREE.Mesh(new THREE.BoxGeometry(0.3, 0.02, 0.2), deskMat);
    laptopBase.position.set(0, 0.02, 0.05);
    deskGroup.add(laptopBase);

    const keyboardGlow = new THREE.Mesh(new THREE.BoxGeometry(0.26, 0.005, 0.14), cyberTealMat);
    keyboardGlow.position.set(0, 0.035, 0.05);
    deskGroup.add(keyboardGlow);

    // Floating Hologram Screen
    const screenGroup = new THREE.Group();
    screenGroup.position.set(0, 0.04, -0.05);
    deskGroup.add(screenGroup);

    const holoScreen = new THREE.Mesh(new THREE.PlaneGeometry(0.34, 0.22), holoScreenMat);
    holoScreen.position.set(0, 0.12, 0);
    holoScreen.rotation.x = -0.15;
    screenGroup.add(holoScreen);

    // Code lines on Hologram screen
    const lineMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
    for (let i = 0; i < 4; i++) {
      const codeLine = new THREE.Mesh(
        new THREE.PlaneGeometry(0.12 + (i % 2) * 0.1, 0.012),
        lineMat
      );
      codeLine.position.set(-0.04, 0.17 - i * 0.035, 0.002);
      codeLine.rotation.x = -0.15;
      screenGroup.add(codeLine);
    }

    // 5. Particle System (Celebration Stars / Typing Sparks / Alert Red Glyphs)
    const particleCount = 40;
    const particleGeo = new THREE.BufferGeometry();
    const particlePositions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i++) {
      particlePositions[i] = (Math.random() - 0.5) * 1.5;
    }
    particleGeo.setAttribute("position", new THREE.BufferAttribute(particlePositions, 3));
    const particleMat = new THREE.PointsMaterial({
      size: 0.04,
      color: 0x00f0ff,
      transparent: true,
      opacity: 0.8,
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    companionGroup.add(particles);

    // 6. External VRM Loader (If a custom VRM is provided)
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
            charGroup.visible = false; // Hide procedural model in favor of VRM
            companionGroup.add(vrm.scene);
            if (onModelLoaded && vrm.meta?.name) {
              onModelLoaded(vrm.meta.name);
            }
          }
        },
        undefined,
        (err) => {
          if (onError) onError(`VRM load failed: ${err}`);
        }
      );
    }

    // 7. Mouse tracking for interactive head follow
    let mouseX = 0;
    let mouseY = 0;
    let isDragging = false;
    let prevMouseX = 0;

    const onMouseMove = (e: MouseEvent) => {
      const rect = container.getBoundingClientRect();
      const x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      const y = -(((e.clientY - rect.top) / rect.height) * 2 - 1);
      mouseX = x;
      mouseY = y;

      if (isDragging && interactive) {
        const delta = e.clientX - prevMouseX;
        companionGroup.rotation.y += delta * 0.02;
        prevMouseX = e.clientX;
      }
    };

    const onMouseDown = (e: MouseEvent) => {
      isDragging = true;
      prevMouseX = e.clientX;
    };

    const onMouseUp = () => {
      isDragging = false;
    };

    if (interactive) {
      container.addEventListener("mousemove", onMouseMove);
      container.addEventListener("mousedown", onMouseDown);
      window.addEventListener("mouseup", onMouseUp);
    }

    // 8. Animation Render Loop
    let clock = new THREE.Clock();
    let animId = 0;

    const animate = () => {
      animId = requestAnimationFrame(animate);
      const delta = clock.getDelta();
      const time = clock.getElapsedTime();
      const currentState = stateRef.current;

      // Update VRM spring bones if loaded
      if (currentVrm) {
        currentVrm.update(delta);
      }

      // Smooth head cursor tracking (only when not in sleeping state)
      if (currentState !== "sleeping") {
        headGroup.rotation.y = THREE.MathUtils.lerp(headGroup.rotation.y, mouseX * 0.5, 0.1);
        headGroup.rotation.x = THREE.MathUtils.lerp(headGroup.rotation.x, -mouseY * 0.3, 0.1);
      }

      // State-specific procedural animation controller
      switch (currentState) {
        case "working":
          // Typing motion on 3D keyboard
          deskGroup.visible = true;
          visor.material = cyberTealMat;
          bottomGlow.color.setHex(0x00f0ff);
          holoScreenMat.color.setHex(0x00ffff);
          particleMat.color.setHex(0x00f0ff);

          // Fast alternating arm typing
          leftArmGroup.rotation.x = -1.1 + Math.sin(time * 18) * 0.15;
          rightArmGroup.rotation.x = -1.1 + Math.cos(time * 18) * 0.15;
          leftArmGroup.rotation.z = 0.3;
          rightArmGroup.rotation.z = -0.3;

          // Head slight nod toward code screen
          headGroup.position.y = 0.68 + Math.sin(time * 6) * 0.01;
          headGroup.rotation.x = 0.18 + Math.sin(time * 12) * 0.03;
          torso.rotation.x = 0.08;

          // Hologram glow pulse
          holoScreenMat.opacity = 0.75 + Math.sin(time * 8) * 0.15;
          break;

        case "thinking":
          // Hand on chin, looking up thoughtfully
          deskGroup.visible = true;
          visor.material = cyberGoldMat;
          bottomGlow.color.setHex(0xffaa00);
          holoScreenMat.color.setHex(0xffbb00);
          particleMat.color.setHex(0xffcc00);

          rightArmGroup.rotation.x = -1.6;
          rightArmGroup.rotation.z = -0.5;
          leftArmGroup.rotation.x = -0.3;
          leftArmGroup.rotation.z = 0.2;

          headGroup.rotation.x = -0.2 + Math.sin(time * 2) * 0.08;
          headGroup.rotation.z = 0.15;
          torso.rotation.x = -0.05;
          break;

        case "waiting":
        case "attention_required":
          // Foot tap / idle hand waving
          deskGroup.visible = true;
          visor.material = cyberGoldMat;
          bottomGlow.color.setHex(0xffaa00);

          leftArmGroup.rotation.x = -0.2;
          rightArmGroup.rotation.x = -0.2 + Math.sin(time * 3) * 0.1;
          rightArmGroup.rotation.z = -0.3;
          headGroup.rotation.y = Math.sin(time * 2) * 0.2;
          break;

        case "success":
        case "celebrating":
        case "level_up":
          // Double fist-pump victory jump
          deskGroup.visible = false;
          visor.material = cyberTealMat;
          bottomGlow.color.setHex(0x00ff88);
          particleMat.color.setHex(0xffdf00);

          leftArmGroup.rotation.x = -2.2 + Math.sin(time * 10) * 0.3;
          rightArmGroup.rotation.x = -2.2 + Math.sin(time * 10 + 0.5) * 0.3;
          leftArmGroup.rotation.z = 0.6;
          rightArmGroup.rotation.z = -0.6;

          // Bouncing jump
          charGroup.position.y = Math.abs(Math.sin(time * 8)) * 0.25;
          charGroup.rotation.y = Math.sin(time * 4) * 0.3;
          break;

        case "error":
          // Red alert glitching
          deskGroup.visible = true;
          bottomGlow.color.setHex(0xff0044);
          holoScreenMat.color.setHex(0xff0044);
          particleMat.color.setHex(0xff0044);

          leftArmGroup.rotation.x = -1.8;
          rightArmGroup.rotation.x = -1.8;
          leftArmGroup.rotation.z = 0.8;
          rightArmGroup.rotation.z = -0.8;

          // Shaking head
          headGroup.rotation.y = Math.sin(time * 24) * 0.25;
          headGroup.position.y = 0.64;
          break;

        case "sleeping":
          // Slumped resting
          deskGroup.visible = true;
          bottomGlow.color.setHex(0x334466);
          holoScreenMat.opacity = 0.2;
          particleMat.color.setHex(0x5588cc);

          leftArmGroup.rotation.x = -1.1;
          rightArmGroup.rotation.x = -1.1;
          torso.rotation.x = 0.3;
          headGroup.rotation.x = 0.4;
          headGroup.position.y = 0.6;
          charGroup.position.y = -0.05 + Math.sin(time * 1.5) * 0.015;
          break;

        case "idle":
        default:
          // Breathing & floating idle sway
          deskGroup.visible = true;
          visor.material = cyberTealMat;
          bottomGlow.color.setHex(0x00f0ff);
          holoScreenMat.color.setHex(0x00ffff);
          particleMat.color.setHex(0x00f0ff);

          const breathe = Math.sin(time * 2.2);
          charGroup.position.y = breathe * 0.03;
          torso.scale.y = 1 + breathe * 0.02;
          headGroup.position.y = 0.68 + breathe * 0.02;

          leftArmGroup.rotation.x = -0.4 + breathe * 0.05;
          rightArmGroup.rotation.x = -0.4 + breathe * 0.05;
          leftArmGroup.rotation.z = 0.2;
          rightArmGroup.rotation.z = -0.2;
          break;
      }

      // Rotate particle cloud gently
      particles.rotation.y = time * 0.2;

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
      renderer.dispose();
      container.innerHTML = "";
    };
  }, [interactive, onError, onModelLoaded, size, vrmUrl]);

  return (
    <div
      ref={containerRef}
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
