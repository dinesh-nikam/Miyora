import type { ErrorReport } from "./types";

/** What failed, why, and what the user can do. Never shows stack traces. */
export function ErrorCallout({ error, title }: { error: ErrorReport; title?: string }) {
  return (
    <div className="studio-error" role="alert">
      <strong>{title ?? "Something went wrong"}</strong>
      <p>{error.message}</p>
      <p className="studio-error-hint">{error.suggestion}</p>
      {(error.requestId || error.providerCode !== null) && (
        <p className="studio-muted">
          {error.providerCode !== null && <>Provider code {error.providerCode}</>}
          {error.providerCode !== null && error.requestId && " · "}
          {error.requestId && <>Request ID <code>{error.requestId}</code></>}
        </p>
      )}
    </div>
  );
}
