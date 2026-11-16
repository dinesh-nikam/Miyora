import type { ReactNode } from "react";
import type { PetPackSummary } from "../../types/app";
import { PanelErrorBanner } from "../common/PanelErrorBanner";

interface CompanionsTabProps {
  avatarError: string | null;
  companionsError: string | null;
  onRetryAvatar: () => void;
  onRetryCompanions: () => void;
  installedPacks: PetPackSummary[];
  selectedPetId: string | null;
  onSelectPet: (selectionId: string) => void;
  onImportPack: () => void;
  children: ReactNode;
}

export function CompanionsTab({
  avatarError,
  companionsError,
  onRetryAvatar,
  onRetryCompanions,
  installedPacks,
  selectedPetId,
  onSelectPet,
  onImportPack,
  children,
}: CompanionsTabProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "32px" }}>
      <PanelErrorBanner error={avatarError} onRetry={onRetryAvatar} />

      <div>
        <h2 className="display" style={{ fontSize: "clamp(32px, 5vw, 64px)", color: "#0a0a0a", margin: "0 0 8px 0" }}>
          PET <span className="highlight-magenta">LIBRARY</span>
        </h2>
        <p className="font-grotesk" style={{ fontSize: "15px", color: "#0a0a0a", fontWeight: 600, margin: "0 0 18px 0" }}>
          Browse rigged Miyora pets by category, search by name, species, tag, or personality, preview every state, and pick your desktop companion.
        </p>
        <div className="edge-thick shadow-pop-lg studio-outer" style={{ backgroundColor: "#070c18", padding: "20px" }}>
          {children}
        </div>
      </div>

      <PanelErrorBanner error={companionsError} onRetry={onRetryCompanions} />

      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", flexWrap: "wrap", gap: "16px" }}>
        <div>
          <h2 className="display" style={{ fontSize: "clamp(28px, 4.5vw, 52px)", color: "#0a0a0a", margin: 0 }}>
            2D SPRITE <span className="highlight-yellow">PACKS</span>
          </h2>
          <p className="font-grotesk" style={{ fontSize: "14px", color: "#0a0a0a", fontWeight: 600, margin: "6px 0 0 0" }}>
            Verified PetPacks with pure data-driven spritesheets, shown in 2D mode. Click to activate companion.
          </p>
        </div>
        <button
          onClick={onImportPack}
          className="font-grotesk shadow-pop shadow-pop-hover"
          style={{
            background: "#ff2db7",
            border: "3px solid #0a0a0a",
            color: "#ffffff",
            padding: "10px 20px",
            borderRadius: "999px",
            fontSize: "13px",
            fontWeight: 900,
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            cursor: "pointer",
          }}
        >
          + Import PetPack Archive
        </button>
      </div>

      {/* Pack Cards Grid (Selected Work Tile System) */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "24px" }}>
        {installedPacks.map((pack) => {
          const isSelected = selectedPetId
            ? selectedPetId === pack.selectionId || selectedPetId === pack.id
            : false;
          return (
            <div
              key={pack.selectionId}
              className="edge-thick shadow-pop shadow-pop-hover"
              style={{
                backgroundColor: "#ffffff",
                display: "flex",
                flexDirection: "column",
                overflow: "hidden",
              }}
            >
              {/* Abstract Image / Graphic Half */}
              <div
                style={{
                  width: "100%",
                  height: "170px",
                  backgroundColor: "#0a0a0a",
                  borderBottom: "4px solid #0a0a0a",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  position: "relative",
                  overflow: "hidden",
                }}
              >
                {/* Diagonal background accent badge */}
                {isSelected && (
                  <div
                    className="font-grotesk shadow-pop-sm"
                    style={{
                      position: "absolute",
                      top: "12px",
                      right: "12px",
                      backgroundColor: "#00d4ff",
                      color: "#0a0a0a",
                      fontSize: "11px",
                      fontWeight: 900,
                      padding: "4px 10px",
                      border: "2px solid #0a0a0a",
                      letterSpacing: "0.08em",
                      textTransform: "uppercase",
                      zIndex: 2,
                    }}
                  >
                    ACTIVE COMPANION
                  </div>
                )}

                {pack.previewDataUrl ? (
                  <img
                    src={pack.previewDataUrl}
                    alt={pack.name}
                    style={{
                      maxHeight: "100%",
                      maxWidth: "100%",
                      objectFit: "contain",
                      imageRendering: "pixelated",
                      filter: "drop-shadow(4px 4px 0 #00d4ff)",
                    }}
                  />
                ) : (
                  <span style={{ fontSize: "56px", filter: "drop-shadow(4px 4px 0 #ff2db7)" }}>
                    ⚡
                  </span>
                )}
              </div>

              {/* Content Half */}
              <div style={{ padding: "20px", display: "flex", flexDirection: "column", gap: "12px", flex: 1, justifyContent: "space-between" }}>
                <div>
                  <div className="font-grotesk" style={{ fontSize: "11px", fontWeight: 800, color: "#ff2db7", textTransform: "uppercase", letterSpacing: "0.15em" }}>
                    0{installedPacks.indexOf(pack) + 1} / PETPACK
                  </div>
                  <div className="display" style={{ fontSize: "24px", color: "#0a0a0a", marginTop: "4px" }}>
                    {pack.name}
                  </div>
                  {/* Structured metadata */}
                  <div
                    className="petpack-meta"
                    aria-label={`Version ${pack.version}, by ${pack.creator}, license ${pack.license}`}
                  >
                    <span className="petpack-meta__label">Version</span>
                    <span className="petpack-meta__value">v{pack.version}</span>
                    <span className="petpack-meta__label">Author</span>
                    <span className="petpack-meta__value" title={pack.creator}>{pack.creator}</span>
                    <span className="petpack-meta__label">License</span>
                    <span className="petpack-meta__value">
                      <span
                        className={`petpack-meta__license-badge ${
                          pack.license === "CC0-1.0"
                            ? "cc0"
                            : pack.license.startsWith("CC-BY")
                            ? "cc-by"
                            : "restricted"
                        }`}
                      >
                        {pack.license}
                      </span>
                    </span>
                  </div>
                </div>

                {/* Select Button */}
                <button
                  onClick={() => onSelectPet(pack.selectionId)}
                  disabled={isSelected}
                  className="font-grotesk shadow-pop-sm shadow-pop-hover"
                  style={{
                    width: "100%",
                    padding: "10px",
                    backgroundColor: isSelected ? "#00d4ff" : "#0a0a0a",
                    color: isSelected ? "#0a0a0a" : "#ffffff",
                    border: "3px solid #0a0a0a",
                    fontSize: "12px",
                    fontWeight: 900,
                    textTransform: "uppercase",
                    letterSpacing: "0.08em",
                    cursor: isSelected ? "default" : "pointer",
                  }}
                >
                  {isSelected ? "Currently Active" : "Select Companion ->"}
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
