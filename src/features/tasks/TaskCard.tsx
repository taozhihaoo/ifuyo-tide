import type { TaskDto, TaskState } from "../../lib/types";

interface TaskCardProps {
  task: TaskDto;
  busy: boolean;
  onDelete: (taskId: string) => void;
  onTransition: (taskId: string, next: TaskState) => void;
}

/** The actions the Rust state machine allows from a given state. */
function allowedTransitions(state: TaskState): Array<[TaskState, string]> {
  switch (state) {
    case "created":
      return [["ready", "Prepare"]];
    case "ready":
      return [["running", "Start"]];
    case "running":
      return [
        ["paused", "Pause"],
        ["completed", "Complete"],
      ];
    case "paused":
      return [["running", "Resume"]];
    default:
      return [];
  }
}

export function TaskCard({ task, busy, onDelete, onTransition }: TaskCardProps) {
  return (
    <li className="task-card" data-testid="task-card">
      <div className="task-main">
        <span className="task-name" title={task.name}>
          {task.name}
        </span>
        <span className={`task-state state-${task.state}`}>{task.state}</span>
      </div>
      <div className="task-meta">
        <span>created {formatTime(task.createdAt)}</span>
        <span>updated {formatTime(task.updatedAt)}</span>
      </div>
      <div className="task-actions">
        {allowedTransitions(task.state).map(([next, label]) => (
          <button
            key={next}
            type="button"
            className="btn btn-small"
            disabled={busy}
            onClick={() => onTransition(task.taskId, next)}
          >
            {label}
          </button>
        ))}
        <span className="task-spacer" />
        <button
          type="button"
          className="btn btn-small btn-danger"
          disabled={busy}
          onClick={() => onDelete(task.taskId)}
        >
          Delete
        </button>
      </div>
    </li>
  );
}

function formatTime(unixMs: number): string {
  // UTC internally; the UI renders local time.
  return new Date(unixMs).toLocaleString();
}
