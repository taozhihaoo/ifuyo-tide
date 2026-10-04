import { EmptyState } from "../../components/EmptyState";
import { strings } from "../../lib/strings";
import { PlayerPage } from "./PlayerPage";
import type { PlaybackSessionInfo } from "./playbackApi";

/**
 * M5 §42: the Player view hosts the PlayerPage for the app's single
 * playback session. The session object is owned by App (backend is
 * the source of truth — after a frontend reload App re-queries the
 * backend's active session, §37/§38); without one this view stays an
 * honest empty state.
 */
export function PlayerView({
  session,
  onClose,
}: {
  session: PlaybackSessionInfo | null;
  onClose: () => void;
}) {
  return session ? (
    <PlayerPage session={session} onClose={onClose} />
  ) : (
    <section className="view" aria-label="Player">
      <div className="view-header">
        <h2>Player</h2>
      </div>
      <EmptyState
        title={strings.library.nothingPlaying}
        hint={strings.library.nothingPlayingHint}
      />
    </section>
  );
}
