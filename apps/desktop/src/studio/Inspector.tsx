import type { ReactNode } from "react";
import { ExportPanel } from "./ExportPanel";
import { RetopologyPanel } from "./RetopologyPanel";
import { formatBytes, formatCount, taskStatusLabel } from "./format";
import type { SceneStats } from "./sceneStats";
import type { AssetDetail, GenerationView, ModelReport, TaskView } from "./types";

interface InspectorProps {
  detail: AssetDetail | null;
  generation: GenerationView | null;
  stats: SceneStats | null;
  onSelectGeneration: (generation: GenerationView) => void;
  onRegenerate: (generation: GenerationView) => void;
  onExported: () => void;
  onVersionCreated: (task: TaskView) => void;
}

const OPERATION_LABELS: Record<string, string> = {
  text_to_3d: "Text → 3D",
  image_to_3d: "Image → 3D",
  retopology: "Retopology (local)",
};

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="studio-row">
      <dt>{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

const yesNo = (value: boolean | null | undefined) => (value === null || value === undefined ? "Unknown" : value ? "Yes" : "No");

function modelReport(generation: GenerationView): ModelReport | null {
  return generation.outputs.find((output) => output.kind === "model")?.report ?? null;
}

function pbrSummary(report: ModelReport | null): string {
  if (!report) return "Not inspected";
  const pbr = report.pbr;
  if (!pbr) return `Not readable from ${report.format.toUpperCase()}`;
  const present = [
    pbr.baseColor && "base color",
    pbr.metallicRoughness && "metallic/roughness",
    pbr.normal && "normal",
    pbr.occlusion && "AO",
    pbr.emissive && "emissive",
  ].filter((entry): entry is string => typeof entry === "string");
  return present.length > 0 ? present.join(", ") : "None in file";
}

export function Inspector({ detail, generation, stats, onSelectGeneration, onRegenerate, onExported, onVersionCreated }: InspectorProps) {
  if (!detail) {
    return (
      <section className="studio-panel studio-inspector" aria-label="Inspector">
        <p className="studio-muted">Select an asset in the library to inspect it.</p>
      </section>
    );
  }
  const report = generation ? modelReport(generation) : null;
  const settings = generation?.settings ?? {};
  const units = settings.autoSize ? "m" : "units";
  return (
    <section className="studio-panel studio-inspector" aria-label="Inspector">
      <h3>{detail.name}</h3>
      <h4>Versions</h4>
      <ol className="studio-versions">
        {detail.generations.map((entry) => (
          <li key={entry.id}>
            <button className={entry.id === generation?.id ? "active" : ""} onClick={() => onSelectGeneration(entry)}>
              {entry.versionNumber ? `v${entry.versionNumber}` : "—"} · {entry.task ? taskStatusLabel(entry.task) : "Unknown"}
              <small className="studio-muted"> {new Date(entry.createdAt).toLocaleString()}</small>
            </button>
          </li>
        ))}
      </ol>

      {generation && (
        <>
          <h4>Generation</h4>
          <dl>
            <Row label="Source">{OPERATION_LABELS[generation.operation] ?? generation.operation}</Row>
            {generation.operation === "retopology" && typeof settings.sourceTriangles === "number" && typeof settings.resultTriangles === "number" && (
              <Row label="Reduction">
                {formatCount(settings.sourceTriangles)} → {formatCount(settings.resultTriangles)} triangles (−
                {Math.round((1 - settings.resultTriangles / settings.sourceTriangles) * 100)}%)
              </Row>
            )}
            {generation.prompt && <Row label="Prompt">{generation.prompt}</Row>}
            {generation.negativePrompt && <Row label="Negative">{generation.negativePrompt}</Row>}
            {generation.inputs.map((input) => (
              <Row key={input.sha256} label="Input image">
                {input.originalName} · {input.width}×{input.height} · {formatBytes(input.byteSize)}
              </Row>
            ))}
            <Row label="Provider">{generation.provider}</Row>
            <Row label="Model">{generation.model}</Row>
            <Row label="Model seed">{settings.modelSeed ?? "—"}</Row>
            <Row label="Texture seed">{settings.textureSeed ?? "—"}</Row>
            <Row label="Settings">
              <code className="studio-code">{JSON.stringify(settings)}</code>
            </Row>
            <Row label="Created">{new Date(generation.createdAt).toLocaleString()}</Row>
            {generation.task && <Row label="Task ID">{generation.task.providerTaskId}</Row>}
            <Row label="Credits">
              {generation.usage
                ? `${generation.usage.credits.toFixed(2)} (${generation.usage.source === "provider" ? "reported by provider" : "ESTIMATED"})`
                : "Not reported"}
            </Row>
          </dl>
          {generation.operation !== "retopology" && (
            <button onClick={() => onRegenerate(generation)} title="Submit the same request with new seeds">
              Generate new version
            </button>
          )}

          <h4>Mesh</h4>
          <dl>
            <Row label="Format">{report ? `${report.format.toUpperCase()} · ${formatBytes(report.byteSize)}` : "—"}</Row>
            <Row label="Triangles">{formatCount(stats?.triangles ?? report?.triangles)}</Row>
            <Row label="Vertices">{formatCount(stats?.vertices ?? report?.vertices)}</Row>
            <Row label="Meshes">{formatCount(stats?.meshes ?? report?.meshes)}</Row>
            <Row label="Dimensions">
              {stats ? `${stats.size.x.toFixed(3)} × ${stats.size.y.toFixed(3)} × ${stats.size.z.toFixed(3)} ${units}` : "—"}
            </Row>
            <Row label="Topology">{settings.quad ? "Quad requested" : "Triangles"}</Row>
            <Row label="UVs">{yesNo(report?.hasUv)}</Row>
            {report && report.extensionsRequired.length > 0 && <Row label="Extensions">{report.extensionsRequired.join(", ")}</Row>}
          </dl>

          <h4>Materials &amp; textures</h4>
          <dl>
            <Row label="Materials">{formatCount(stats?.materials ?? report?.materials)}</Row>
            <Row label="PBR maps">{pbrSummary(report)}</Row>
            <Row label="Vertex colours">{yesNo(report?.hasVertexColors)}</Row>
            {stats?.textures.map((texture, index) => (
              <Row key={`${texture.slot}-${index}`} label={texture.slot}>
                {texture.width && texture.height ? `${texture.width}×${texture.height}` : "size unknown"}
              </Row>
            ))}
          </dl>

          <RetopologyPanel generation={generation} triangles={stats?.triangles ?? report?.triangles ?? null} onCreated={onVersionCreated} />

          <ExportPanel generation={generation} onExported={onExported} />

          <h4>Rig &amp; animation</h4>
          <dl>
            <Row label="Skinned">{report?.skins === null || report?.skins === undefined ? "Unknown" : report.skins > 0 ? "Yes" : "No"}</Row>
            <Row label="Animations">{formatCount(report?.animations)}</Row>
          </dl>

          {stats && stats.hierarchy.length > 0 && (
            <details className="studio-details">
              <summary>Hierarchy</summary>
              <ul className="studio-tree">
                {stats.hierarchy.map((node, index) => (
                  <li key={index} style={{ paddingLeft: node.depth * 12 }}>
                    {node.name} <small className="studio-muted">{node.type}</small>
                  </li>
                ))}
                {stats.hierarchyTruncated && <li className="studio-muted">…truncated</li>}
              </ul>
            </details>
          )}
        </>
      )}
    </section>
  );
}
