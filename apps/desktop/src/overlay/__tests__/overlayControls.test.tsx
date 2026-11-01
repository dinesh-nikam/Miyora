import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { CompanionRenderer } from "../../components/CompanionRenderer";
import { PetOverlayFrame } from "../PetOverlayFrame";
import { CompanionControlsPanel } from "../../settings/CompanionControlsPanel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => undefined) }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ startDragging: vi.fn(async () => undefined) }) }));

const invokeMock = vi.mocked(invoke);

afterEach(() => {
  cleanup();
  invokeMock.mockReset();
});

describe("desktop overlay", () => {
  it("renders only the pet: no buttons, inputs, speech, or capsule", () => {
    invokeMock.mockResolvedValue(undefined);
    const { container } = render(
      <PetOverlayFrame>
        <CompanionRenderer state="idle" animation={null} mode="2d" interactive={false} bare />
      </PetOverlayFrame>,
    );

    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(screen.queryAllByRole("textbox")).toHaveLength(0);
    expect(container.querySelector(".companion-speech-bubble")).toBeNull();
    expect(container.querySelector(".companion-unified-capsule")).toBeNull();
  });
});

describe("companion controls panel", () => {
  it("explains a shortcut conflict and offers tray and button fallbacks", async () => {
    invokeMock.mockImplementation(async (command) =>
      command === "get_companion_shortcuts"
        ? [
            { action: "show_hide", status: { state: "registered", accelerator: "Ctrl+Alt+P" } },
            {
              action: "pause",
              status: { state: "conflict", accelerator: "Ctrl+Alt+Space", message: "Ctrl+Alt+Space is already in use by another application" },
            },
            { action: "reposition", status: { state: "unbound" } },
          ]
        : undefined,
    );

    render(<CompanionControlsPanel />);

    expect(await screen.findByText(/already in use by another application\. This action is unbound/)).toBeTruthy();
    expect(screen.getByText("Active: Ctrl+Alt+P")).toBeTruthy();
    expect(screen.getByLabelText("Pause/Resume")).toBeTruthy();
    // Every overlay action stays reachable without a shortcut.
    for (const label of ["Show/Hide pet", "Pause/Resume", "Reposition mode"]) {
      expect(screen.getByRole("button", { name: label })).toBeTruthy();
    }
  });

  it("runs a companion action through Rust when its button is pressed", async () => {
    invokeMock.mockImplementation(async (command) => (command === "get_companion_shortcuts" ? [] : undefined));
    render(<CompanionControlsPanel />);

    screen.getByRole("button", { name: "Reposition mode" }).click();

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_companion_action", { action: "reposition" }));
  });
});
