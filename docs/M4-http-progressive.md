# M4 HTTP Progressive Streaming

How an HTTP download becomes watchable while it downloads — the
Critical Acceptance #1/#2 chain (M4(下) §126/§127).

## The lifecycle

```
create task → .part grows (flush per chunk)
→ create_playback_session(task_id)
   ├─ completed?  → File resource (final path)
   └─ in progress → Part resource:
       path          = .part
       total_len     = declared remote size
       len_rx        = live durable-len watch (if the task is restored)
       origin        = the task's source URL (read-through source)
→ gateway URL → <video> plays the prefix
→ player reaches the frontier → gateway read WAITS
→ next chunk flushes → watch fires → read completes → playback continues
→ user seeks past the frontier → origin read-through serves the range
```

## The two beyond-frontier paths

1. **Frontier wait** (no origin, or origin-less resources): the read
   waits on the durable-len watch until the download reaches the
   requested end or the 120 s deadline — a structured `WAIT_TIMEOUT`,
   never a hang, never a short read. This is the "watch while it
   downloads at pace" path.
2. **Origin read-through**: beyond-frontier ranges first grace-wait
   2 s for local growth (sequential viewing should not re-fetch from
   the origin what the download is about to store anyway), then the
   missing tail — starting at the first missing byte, not at the
   frontier — is fetched via ranged GET and served. This is the
   "seek into the un-downloaded region" path; measured ~3 ms per
   origin round trip.

## Origin read rules (§145)

- A ranged request answered `200` means the origin IGNORED Range.
  Serving that body would be WRONG BYTES (it starts at offset 0) —
  classified `RESOURCE_RANGE_UNSUPPORTED` and never served. The
  capability layer then reports `supports_random_access: false` and
  the UI says so honestly.
- Responses are read stream-limited to exactly the requested length;
  short bodies are structured errors.
- Network failures are classified (timeout / connect / HTTP status —
  §143), the session is NOT permanently failed (§142): the next
  request retries naturally.

## Byte-correctness guarantees (proven by tests)

- frontier wait + continue: byte-exact (Critical #1)
- read-through seek: byte-exact, and the origin log shows the real
  Range starting at the first missing byte (Critical #2)
- read-while-writing: repeated reads of the growing region during an
  active download never tear (§118 test)
- restart: availability = physical size, recovered bytes byte-exact
  (Critical #7)

## Known limitations (recorded, not hidden)

- Read-through ranges are not persisted — a restart re-fetches them
  if the frontier has not reached them yet (ADR-3).
- `unknown-size` downloads (no total) serve their prefix with
  snapshot lengths; no fabricated Content-Length (§108).
- Origin ETag changes mid-download are M1's domain; read-through does
  not add a second consistency mechanism (If-Range) — recorded as a
  P1 refinement.
