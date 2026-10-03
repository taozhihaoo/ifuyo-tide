import { EmptyState } from "../../components/EmptyState";

/**
 * Player placeholder. Progressive playback lands in M4/M5; M0 renders
 * an honest placeholder — no <video>, no fake playback.
 */
export function PlayerView() {
  return (
    <section className="view" aria-label="Player">
      <div className="view-header">
        <h2>Player</h2>
      </div>
      <EmptyState
        title="Not implemented in M0"
        hint="Progressive streaming and playback arrive in M4/M5."
      />
    </section>
  );
}
