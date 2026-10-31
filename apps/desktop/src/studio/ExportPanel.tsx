import { useState } from "react";
import { studioApi } from "./api";
import { ErrorCallout } from "./ErrorCallout";
import { formatBytes } from "./format";
import type { ErrorReport, ExportView, GenerationView, LocalExportFormat } from "./types";

const FORMATS: { id: LocalExportFormat; label: string; hint: string }[] = [
  { id: "original", label: "Original", hint: "Exact copy of the generated file" },
  { id: "obj", label: "OBJ", hint: "Mesh + UVs; texture and .mtl saved alongside" },
  { id: "stl", label: "STL", hint: "Binary, geometry only (3D printing)" },
  { id: "ply", label: "PLY", hint: "Binary, keeps vertex colours" },
];
const MILLIMETRES_PER_METRE = 1000;

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function ExportPanel({ generation, onExported }: { generation: GenerationView; onExported: () => void }) {
  const [format, setFormat] = useState<LocalExportFormat>("original");
  const [scale, setScale] = useState(1);
  const [zUp, setZUp] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [saved, setSaved] = useState<ExportView | null>(null);

  const model = generation.outputs.find((output) => output.kind === "model");
  if (!model) return null;
  const compressed = (model.report?.extensionsRequired.length ?? 0) > 0;
  const convertible = model.format === "glb" && !compressed;
  const converting = format !== "original";

  const choose = (next: LocalExportFormat) => {
    setFormat(next);
    // Slicers expect z-up; keep the user's choice otherwise.
    if (next === "stl") setZUp(true);
  };

  const run = async () => {
    setBusy(true);
    setError(null);
    setSaved(null);
    const result = await studioApi.exportOutput(model.id, format, { scale, zUp });
    setBusy(false);
    if (!result.ok) {
      setError(result.error);
    } else if (result.data) {
      setSaved(result.data);
      onExported();
    }
  };

  return (
    <>
      <h4>Export</h4>
      <div className="studio-segmented" role="radiogroup" aria-label="Export format">
        {FORMATS.map((entry) => (
          <button
            key={entry.id}
            role="radio"
            aria-checked={format === entry.id}
            className={format === entry.id ? "active" : ""}
            title={entry.hint}
            disabled={entry.id !== "original" && !convertible}
            onClick={() => choose(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      {!convertible && (
        <p className="studio-muted">
          {compressed
            ? "This model uses compressed geometry; only the original can be exported."
            : `Conversion needs a GLB; this model is ${model.format.toUpperCase()}.`}
        </p>
      )}
      {converting && (
        <div className="studio-grid-2">
          <label className="studio-field">
            <span>Scale</span>
            <input type="number" min={0.001} max={1000} step="any" value={scale} onChange={(event) => setScale(Number(event.target.value))} />
          </label>
          <div className="studio-checks">
            <label className="studio-check">
              <input type="checkbox" checked={zUp} onChange={(event) => setZUp(event.target.checked)} />
              Z-up
            </label>
            <button className="studio-link" onClick={() => setScale(MILLIMETRES_PER_METRE)} title="Metres to millimetres">
              ×1000 (m→mm)
            </button>
          </div>
        </div>
      )}
      <button className="studio-primary" disabled={busy} onClick={() => void run()}>
        {busy ? "Exporting…" : "Export…"}
      </button>
      {saved && (
        <p className="studio-notice">
          Saved and verified: {saved.files.map(fileName).join(", ")} ({formatBytes(saved.byteSize)})
        </p>
      )}
      {error && <ErrorCallout error={error} title="Export failed" />}
      {generation.exports.length > 0 && (
        <details className="studio-details">
          <summary>Export history ({generation.exports.length})</summary>
          <ul className="studio-tree">
            {generation.exports.map((entry) => (
              <li key={entry.id} title={entry.destination}>
                {entry.format.toUpperCase()} · {fileName(entry.destination)} · {formatBytes(entry.byteSize)}
                <small className="studio-muted"> {new Date(entry.createdAt).toLocaleString()}</small>
              </li>
            ))}
          </ul>
        </details>
      )}
    </>
  );
}
