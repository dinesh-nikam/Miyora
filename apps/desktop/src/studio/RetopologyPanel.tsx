import { useEffect, useState } from "react";
import { studioApi } from "./api";
import { ErrorCallout } from "./ErrorCallout";
import type { ErrorReport, GenerationView, TaskView } from "./types";

const MIN_TARGET = 4;
const RATIOS = [0.5, 0.25, 0.1];
const BUDGETS = [20_000, 5_000, 1_000];

interface RetopologyPanelProps {
  generation: GenerationView;
  /** Triangles of the model as loaded in the viewport (falls back to the file report). */
  triangles: number | null;
  onCreated: (task: TaskView) => void;
}

export function RetopologyPanel({ generation, triangles, onCreated }: RetopologyPanelProps) {
  const [target, setTarget] = useState<number>(0);
  const [preserveBorders, setPreserveBorders] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ErrorReport | null>(null);

  useEffect(() => {
    setTarget(triangles ? Math.max(MIN_TARGET, Math.round(triangles / 4)) : 0);
    setError(null);
  }, [generation.id, triangles]);

  const model = generation.outputs.find((output) => output.kind === "model");
  if (!model || !triangles) return null;
  const compressed = (model.report?.extensionsRequired.length ?? 0) > 0;
  const usable = model.format === "glb" && !compressed && triangles > MIN_TARGET;
  const valid = Number.isInteger(target) && target >= MIN_TARGET && target < triangles;

  const reduce = async () => {
    setBusy(true);
    setError(null);
    const result = await studioApi.retopologize(generation.id, target, preserveBorders);
    setBusy(false);
    if (result.ok) onCreated(result.data);
    else setError(result.error);
  };

  return (
    <>
      <h4>Retopology</h4>
      {!usable ? (
        <p className="studio-muted">
          {compressed ? "Compressed models cannot be reduced locally." : "Polygon reduction needs a GLB model."}
        </p>
      ) : (
        <>
          <p className="studio-muted">
            Reduces triangles on this computer (quadric edge collapse) and saves a new version. Triangles only; quad
            remeshing is not available locally.
          </p>
          <label className="studio-field">
            <span>Target triangles (now {triangles.toLocaleString("en-US")})</span>
            <input
              type="number"
              min={MIN_TARGET}
              max={triangles - 1}
              value={target || ""}
              onChange={(event) => setTarget(Number.parseInt(event.target.value, 10) || 0)}
            />
          </label>
          <div className="studio-row-actions" style={{ justifyContent: "flex-start" }}>
            {RATIOS.map((ratio) => (
              <button key={ratio} onClick={() => setTarget(Math.max(MIN_TARGET, Math.round(triangles * ratio)))}>
                {ratio * 100}%
              </button>
            ))}
            {BUDGETS.filter((budget) => budget < triangles).map((budget) => (
              <button key={budget} onClick={() => setTarget(budget)}>
                {budget / 1000}k
              </button>
            ))}
          </div>
          <label className="studio-check" title="Keeps open edges and UV seams in place; turn off for stronger reduction">
            <input type="checkbox" checked={preserveBorders} onChange={(event) => setPreserveBorders(event.target.checked)} />
            Preserve borders and UV seams
          </label>
          <button className="studio-primary" disabled={busy || !valid} onClick={() => void reduce()}>
            {busy ? "Reducing…" : "Reduce polygons"}
          </button>
        </>
      )}
      {error && <ErrorCallout error={error} title="Polygon reduction failed" />}
    </>
  );
}
