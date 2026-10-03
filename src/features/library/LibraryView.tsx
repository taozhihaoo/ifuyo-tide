import { EmptyState } from "../../components/EmptyState";

/**
 * Library placeholder. The real library (scanning, persistence, playback
 * history) is milestone M6 — M0 renders an honest empty state and does
 * not fake any content.
 */
export function LibraryView() {
  return (
    <section className="view" aria-label="Library">
      <div className="view-header">
        <h2>Library</h2>
      </div>
      <EmptyState
        title="No media yet"
        hint="The local media library is not part of M0. It arrives in M6."
      />
    </section>
  );
}
