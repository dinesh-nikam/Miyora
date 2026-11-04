import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { CommandDeck, quotaSummary, type ProviderReport } from "../CommandDeck";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => []) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => undefined) }));

afterEach(cleanup);

const usage = { costProvenance: "N/A", tokensProvenance: "N/A" };

const codex: ProviderReport = {
  providerId: "codex",
  displayName: "OpenAI Codex CLI",
  detected: true,
  hookStatus: "installed",
  quota: { kind: "unavailable", reason: "not_exposed_locally", action: "copy_codex_status_command" },
  ...usage,
};

const claude: ProviderReport = {
  providerId: "claude_code",
  displayName: "Claude Code",
  detected: true,
  hookStatus: "installed",
  quota: {
    kind: "measured",
    windows: [{ window: "five_hour", remainingPercent: 62.5, resetAt: null }],
    observedAt: "2026-11-01T09:00:00Z",
    provenance: "LIVE",
  },
  ...usage,
};

function renderDeck(providers: ProviderReport[]) {
  render(
    <CommandDeck
      isOpen
      onClose={() => undefined}
      providers={providers}
      activePetName="Aoi"
      activeState="idle"
      isSimulated={false}
      onToggleSimulated={() => undefined}
      onSelectPet={() => undefined}
      installedPacks={[]}
      onVerifyLedger={() => undefined}
    />,
  );
}

describe("truthful quota status", () => {
  it("shows Codex as unavailable with /status guidance and no invented numbers", async () => {
    renderDeck([codex]);
    expect(await screen.findByText("Usage is not available locally.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy /status" })).toBeTruthy();
    expect(screen.queryByRole("meter")).toBeNull();
    expect(document.body.textContent).not.toMatch(/\d+%/);
    expect(await screen.findByText(/Your companion shares one gentle moment a day/i)).toBeTruthy();
  });

  it("shows a meter only for a measured provider value", async () => {
    renderDeck([claude]);
    expect((await screen.findByRole("meter")).getAttribute("aria-valuenow")).toBe("38");
    expect(await screen.findByText(/Your companion shares one gentle moment a day/i)).toBeTruthy();
  });

  it("summarizes every union variant safely", () => {
    expect(quotaSummary(claude.quota)).toBe("63% of five-hour limit left");
    expect(quotaSummary({ kind: "unavailable", reason: "stale", action: null })).toMatch(/out of date/);
    expect(quotaSummary({ kind: "unsupported", reason: "This provider does not expose usage locally." })).toMatch(/does not expose/);
  });
});
