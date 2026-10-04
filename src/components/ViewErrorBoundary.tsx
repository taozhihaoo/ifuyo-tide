import { Component, type ErrorInfo, type ReactNode } from "react";

/**
 * M9 §89 — frontend error boundary: one broken view must NOT white-
 * screen the whole app. Renders a fallback with Reload; the rest of
 * the shell stays alive. Real error text is logged to the console for
 * diagnosis, not shown raw to the user (§15).
 */

interface Props {
  children: ReactNode;
  /** Defaults to "Downloads" — callers pass the view name. */
  viewName?: string;
}

interface State {
  error: Error | null;
}

export class ViewErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Console only — no telemetry (§31), no raw panic text in the UI.
    console.error(`[tide] view error in ${this.props.viewName ?? "view"}`, error, info.componentStack);
  }

  render(): ReactNode {
    if (this.state.error) {
      return (
        <section className="view" aria-label={this.props.viewName ?? "View"}>
          <div className="view-header">
            <h2>{this.props.viewName ?? "View"}</h2>
          </div>
          <div className="banner banner-error" role="alert">
            This view hit an unexpected error. The rest of Tide still works.
          </div>
          <div className="task-actions">
            <button
              type="button"
              className="btn btn-small"
              onClick={() => this.setState({ error: null })}
            >
              Try again
            </button>
          </div>
        </section>
      );
    }
    return this.props.children;
  }
}
