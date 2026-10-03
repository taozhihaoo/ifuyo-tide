import { EmptyState } from "../../components/EmptyState";
import { VideoPlayer } from "./VideoPlayer";
import type { PlaybackSessionInfo } from "./playbackApi";

/**
 * M4(上): real progressive playback through the ONE gateway/ONE
 * player. The session object is owned by App (so Downloads/Torrents
 * views can open one); without a session this view stays an honest
 * empty state.
 */
export function PlayerView({
  session,
  onClose,
}: {
  session: PlaybackSessionInfo | null;
  onClose: () => void;
}) {
  return session ? (
    <VideoPlayer session={session} onClose={onClose} />
  ) : (
    <section className="view" aria-label="Player">
      <div className="view-header">
        <h2>Player</h2>
      </div>
      <EmptyState
        title="Nothing playing"
        hint="Press Play on a download or a torrent file — playback streams through the Tide gateway."
      />
    </section>
  );
}
