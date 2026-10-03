import { useEffect, useRef, useState } from "react";
import { EmptyState } from "../../components/EmptyState";
import { TaskCard } from "./TaskCard";
import { useTasks } from "./useTasks";

interface TasksViewProps {
  /** Bumped by the header "+" button to open the create form. */
  createRequest: number;
}

export function TasksView({ createRequest }: TasksViewProps) {
  const { tasks, error, busy, create, remove, changeState } = useTasks();
  const [formOpen, setFormOpen] = useState(false);
  const [name, setName] = useState("");
  const nameInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (createRequest > 0) {
      setFormOpen(true);
      nameInput.current?.focus();
    }
  }, [createRequest]);

  async function submit() {
    const created = await create(name.trim() || undefined);
    if (created) {
      setName("");
      setFormOpen(false);
    }
  }

  return (
    <section className="view" aria-label="Downloads">
      <div className="view-header">
        <h2>Downloads</h2>
        <button
          type="button"
          className="btn"
          disabled={busy}
          onClick={() => {
            setFormOpen(true);
            nameInput.current?.focus();
          }}
        >
          New test task
        </button>
      </div>

      {error && (
        <div className="banner banner-error" role="alert">
          {error}
        </div>
      )}

      {formOpen && (
        <form
          className="create-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <input
            ref={nameInput}
            value={name}
            placeholder="Task name (optional, defaults to “Test Task”)"
            onChange={(e) => setName(e.target.value)}
            maxLength={200}
          />
          <button type="submit" className="btn btn-primary" disabled={busy}>
            Create
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy}
            onClick={() => setFormOpen(false)}
          >
            Cancel
          </button>
        </form>
      )}

      {tasks === null ? (
        <p className="dim">Loading…</p>
      ) : tasks.length === 0 ? (
        <EmptyState
          title="No downloads yet"
          hint="Create a test task to verify the M0 pipeline. Real downloads arrive in M1."
        />
      ) : (
        <ul className="task-list">
          {tasks.map((task) => (
            <TaskCard
              key={task.taskId}
              task={task}
              busy={busy}
              onDelete={(id) => void remove(id)}
              onTransition={(id, next) => void changeState(id, next)}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
