import { useState } from "react";
import { ErrorCallout } from "./ErrorCallout";
import { formatElapsed, isActive, taskStatusLabel } from "./format";
import type { TaskView } from "./types";

const OPERATION_LABELS: Record<string, string> = {
  text_to_3d: "Text → 3D",
  image_to_3d: "Image → 3D",
  retopology: "Retopology",
};

interface TaskQueueProps {
  tasks: TaskView[];
  nowMs: number;
  onOpen: (task: TaskView) => void;
  onResume: (task: TaskView) => void;
  onRegenerate: (task: TaskView) => void;
}

export function TaskQueue({ tasks, nowMs, onOpen, onResume, onRegenerate }: TaskQueueProps) {
  const [expanded, setExpanded] = useState<string | null>(null);
  if (tasks.length === 0) {
    return <p className="studio-muted studio-queue-empty">No tasks yet. Generated models and their history appear here.</p>;
  }
  return (
    <table className="studio-queue">
      <thead>
        <tr>
          <th>Operation</th>
          <th>Asset</th>
          <th>Provider</th>
          <th>Status</th>
          <th>Elapsed</th>
          <th aria-label="Actions" />
        </tr>
      </thead>
      <tbody>
        {tasks.map((task) => {
          const active = isActive(task);
          return (
            <tr key={task.id} className={`state-${task.state.toLowerCase()}`}>
              <td>{OPERATION_LABELS[task.operation] ?? task.operation}</td>
              <td title={task.assetName}>{task.assetName}</td>
              <td>{task.provider}</td>
              <td>
                <span className="studio-status">{taskStatusLabel(task)}</span>
                {active && task.progress !== null && (
                  <progress max={100} value={task.progress} aria-label={`${task.progress}% reported by provider`} />
                )}
                {task.attempts > 1 && <small className="studio-muted"> attempt {task.attempts}</small>}
                {task.error && task.state === "FAILED" && (
                  <button className="studio-link" onClick={() => setExpanded(expanded === task.id ? null : task.id)}>
                    {expanded === task.id ? "Hide details" : "Details"}
                  </button>
                )}
                {task.error && active && (
                  <small className="studio-muted" title={task.error.message}>
                    {" "}
                    {task.error.message}
                  </small>
                )}
                {expanded === task.id && task.error && <ErrorCallout error={task.error} title="Task failed" />}
              </td>
              <td>{formatElapsed(task.createdAt, task.completedAt, nowMs)}</td>
              <td className="studio-queue-actions">
                {task.versionId && <button onClick={() => onOpen(task)}>Open</button>}
                {task.canResume && (
                  <button onClick={() => onResume(task)} title="Check the same provider task again (no extra credits)">
                    Resume
                  </button>
                )}
                {!active && !task.canResume && task.state !== "SUCCEEDED" && (
                  <button onClick={() => onRegenerate(task)} title="Submit a new task with the same settings (uses credits)">
                    Retry
                  </button>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
