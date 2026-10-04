# M4(上) PlaybackSession & Player

## Lifecycle separation (§51–§53)

`PlaybackSession` is a distinct lifecycle from `DownloadTask`:

- Opening playback never changes task state; pausing/stopping playback
  never pauses/cancels the download.
- Playing to EOF ends the SESSION, not the task.
- Deleting a task while playing is the user's explicit destructive
  action (M3 command semantics unchanged).

Session state model: M4(上) keeps sessions implicit-but-tracked
(`created_at`, token, resource) rather than importing the full
Created/Preparing/Playing/… state machine — WebView2's media element
already owns playing/paused/seeking truth (§51: "复用而不是重新设计").

## Tokens are runtime-only (§69/§70)

Nothing about a session is written to SQLite. `create_playback_session`
returns `{ sessionId, url, size, contentType, sourceKind }`; the URL
dies with the app run. Restart regenerates everything.

## IPC surface (§68, minimal by design)

| Command | Purpose |
|---|---|
| `create_playback_session(source)` | resolve ID → resource → session URL |
| `stop_playback_session(id)` | revoke gateway access + cancel demands |
| `get_playback_info(id)` | session metadata (None if gone) |
| `notify_seek(id)` | drop obsolete demands after a seek (§80) |

Explicitly NO IPC for play/pause/timeupdate/buffered: those are native
`<video>` element operations (§68's principle: "能由 WebView2 原生事件
直接完成的，不要多造 IPC"). pause_playback_session/resume_playback_session
from §68 are intentionally absent — the element does it.

Sources accepted (trusted backend mapping only — the renderer never
sends paths, §59/§60):

- `{ kind: "download_task", task_id }` → final file when complete, else
  the `.part` at PHYSICAL size (nothing downloaded yet → structured
  error).
- `{ kind: "torrent_file", task_id, file_index }` → M3 repository
  lookup; a paused torrent is auto-resumed so playback is not starved
  (§52 allows Task=Paused + Playback=Playing).

## Frontend: ONE player (§64/§65/§66/§73)

- `VideoPlayer.tsx` is the only player component; every source kind
  feeds it a session URL (`video.src = url`). No
  HttpVideoPlayer/TorrentVideoPlayer split exists.
- Real state only: play/waiting/seeking/time/buffered values are read
  from element events (`timeupdate`, `progress`, `waiting`, `seeked`,
  …). Unknown values render "unavailable" — no fake numbers (§65).
- Controls: native `controls` attribute plus explicit Play / Pause /
  Seek-50% / Stop buttons driving the same element. `seeked` fires
  `notify_seek` (§80).
- App-level rule: ONE session at a time — opening a new one stops the
  previous (gateway revoked, demands cancelled). Navigating away from
  the Player view keeps the session alive; Stop or the 12 h TTL ends
  it.
- Autoplay relies on the WebView2 arg
  `--autoplay-policy=no-user-gesture-required` (wired in M0.5,
  `tauri.conf.json` additionalBrowserArgs).

## M4(下) additions

- Sessions carry an owner task id; task deletion invalidates its
  sessions (§102).
- Session DTO carries `capability` (§146/§147) and `availableBytes`
  refreshed on `get_playback_info` (§149).
- The player shows Browser-buffered and Backend-available as distinct
  metrics and disables seeking honestly when the capability says no.

## Known limits

- No `LastAccessAt`/`CurrentRange` tracking (reaping is
  creation-time-based, 12 h).
- Playback-resumed-after-navigation restarts at 0 (the element
  reloads the URL); acceptable for the minimal player, M5 owns the
  real player UX.
