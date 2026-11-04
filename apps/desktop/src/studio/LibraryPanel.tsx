import { useEffect, useState } from "react";
import { readOutput } from "./api";
import type { AssetSummary } from "./types";

function Thumbnail({ outputId, name }: { outputId: number | null; name: string }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (outputId === null) return;
    let objectUrl: string | null = null;
    let cancelled = false;
    readOutput(outputId)
      .then((bytes) => {
        if (cancelled) return;
        objectUrl = URL.createObjectURL(new Blob([bytes]));
        setUrl(objectUrl);
      })
      .catch((error: unknown) => console.warn("[studio] thumbnail unavailable", error));
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [outputId]);
  return url ? <img src={url} alt={`Preview of ${name}`} loading="lazy" /> : <div className="studio-thumb-empty" aria-hidden="true" />;
}

interface LibraryPanelProps {
  assets: AssetSummary[];
  selectedAssetId: string | null;
  onSelect: (asset: AssetSummary) => void;
}

export function LibraryPanel({ assets, selectedAssetId, onSelect }: LibraryPanelProps) {
  const [filter, setFilter] = useState("");
  const visible = assets.filter((asset) => asset.name.toLowerCase().includes(filter.trim().toLowerCase()));
  return (
    <section className="studio-panel studio-library" aria-label="Asset library">
      <input type="search" placeholder="Filter assets" value={filter} onChange={(event) => setFilter(event.target.value)} aria-label="Filter assets" />
      {visible.length === 0 ? (
        <p className="studio-muted">{assets.length === 0 ? "No assets in this project yet." : "No assets match."}</p>
      ) : (
        <ul className="studio-asset-grid">
          {visible.map((asset) => (
            <li key={asset.id}>
              <button className={asset.id === selectedAssetId ? "active" : ""} onClick={() => onSelect(asset)}>
                <Thumbnail outputId={asset.thumbnailOutputId} name={asset.name} />
                <span>{asset.name}</span>
                <small className="studio-muted">
                  {asset.versionCount} version{asset.versionCount === 1 ? "" : "s"}
                </small>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
