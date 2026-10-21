import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { AudioManager, playChime } from "../CompanionRenderer";

class MockGainNode {
  gain = {
    setValueAtTime: vi.fn(),
    exponentialRampToValueAtTime: vi.fn(),
  };
  connect = vi.fn();
}

class MockOscillatorNode {
  type = "sine";
  frequency = {
    setValueAtTime: vi.fn(),
    exponentialRampToValueAtTime: vi.fn(),
  };
  connect = vi.fn();
  start = vi.fn();
  stop = vi.fn();
}

class MockAudioContext {
  state: "running" | "suspended" | "closed" = "running";
  currentTime = 0;
  destination = {};

  createGain = vi.fn(() => new MockGainNode());
  createOscillator = vi.fn(() => new MockOscillatorNode());
  resume = vi.fn().mockResolvedValue(undefined);
  close = vi.fn().mockImplementation(() => {
    this.state = "closed";
    return Promise.resolve();
  });
}

describe("AudioManager", () => {
  beforeEach(() => {
    AudioManager.resetInstance();
    localStorage.clear();
    // @ts-expect-error Mocking browser AudioContext
    window.AudioContext = MockAudioContext;
  });

  afterEach(() => {
    AudioManager.resetInstance();
  });

  it("maintains a single instance across multiple calls", () => {
    const instance1 = AudioManager.getInstance();
    const instance2 = AudioManager.getInstance();
    expect(instance1).toBe(instance2);
  });

  it("lazily creates AudioContext only when requested", () => {
    const manager = AudioManager.getInstance();
    const ctx = manager.getContext();
    expect(ctx).toBeInstanceOf(MockAudioContext);
  });

  it("resumes context if suspended", () => {
    const manager = AudioManager.getInstance();
    const ctx = manager.getContext() as unknown as MockAudioContext;
    ctx.state = "suspended";

    const resumedCtx = manager.getContext() as unknown as MockAudioContext;
    expect(resumedCtx.resume).toHaveBeenCalled();
  });

  it("disposes active AudioContext and clears instance reference", async () => {
    const manager = AudioManager.getInstance();
    const ctx = manager.getContext() as unknown as MockAudioContext;
    expect(ctx.state).toBe("running");

    manager.dispose();
    expect(ctx.close).toHaveBeenCalled();

    // After dispose, next getContext creates a fresh context
    const nextCtx = manager.getContext();
    expect(nextCtx).not.toBe(ctx);
  });

  it("does not play chime when sound is muted in localStorage", () => {
    localStorage.setItem("agentpet_sound_muted", "true");
    const manager = AudioManager.getInstance();
    const spy = vi.spyOn(manager, "getContext");

    playChime("pet");
    expect(spy).not.toHaveBeenCalled();
  });

  it("plays chime and creates oscillator/gain nodes when unmuted", () => {
    const manager = AudioManager.getInstance();
    playChime("pet");
    const ctx = manager.getContext() as unknown as MockAudioContext;
    expect(ctx.createOscillator).toHaveBeenCalled();
    expect(ctx.createGain).toHaveBeenCalled();
  });
});
