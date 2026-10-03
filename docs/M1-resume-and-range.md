# M1 — Resume and Range Semantics

Exact rules implemented by the engine (no "implementation decides"
ambiguity, M1 §59). HTTP semantics follow RFC 9110 §13–§15.

## Resume decision table

| Local state | Validators known? | Request sent | Server replies | Engine action |
|---|---|---|---|---|
| no `.part` (0 bytes) | — | plain GET | 200 | full download |
| partial, size N | ETag or Last-Modified | `Range: bytes=N-` + `If-Range: <validator>` | **206** | verify `Content-Range.start == N` → append; total refreshed from Content-Range |
| partial, size N | ETag or Last-Modified | same | **200** (validator mismatched → representation changed) | **restart from zero** (truncate part; the full body is written over it — never appended) |
| partial, size N | no validators | `Range: bytes=N-` only | **206** | verify `Content-Range.start == N` → append (accepted risk: server gave no validator; Content-Range consistency still enforced) |
| partial, size N | any | `Range: bytes=N-` | **200** (server ignores Range) | **restart from zero** — a 200 body is a FULL representation; appending it to a partial is the forbidden corruption case (M1 §14/§79) |
| partial, size N | any | `Range: bytes=N-` | **416** + `Content-Range: bytes */T` | if `N == T`: the local part already holds the whole resource → **finalize** (no re-download) |
| partial, size N | any | same | **416** with `T ≠ N` | local size is wrong → **restart from zero** |
| partial > remote total | any | `Range: bytes=N-` | 416 (N > T) | restart from zero (covered by the T ≠ N rule) |

Additional invariants:

- A 206 whose `Content-Range.start ≠ N`, or whose header is missing /
  malformed, fails with `DOWNLOAD_INVALID_CONTENT_RANGE` (never a blind
  append).
- The part file is truncated to the verified byte count when the sink
  opens, so a crash mid-write can't leave bytes beyond the resume point.
- After a body EOF with known total: `downloaded == total` is enforced
  before finalize; a short body is treated as a retryable network
  failure (and resumes).
- Remote changed mid-download (no validator, server switched
  representation between chunks): NOT detectable without validators in
  M1 — mitigated by Content-Range start checks on every resume and by
  optional user-supplied SHA-256. Documented limitation; M2 records
  validators at first byte.

## When resume is ALLOWED

`.part` exists with size N > 0 **in the same app session** (M1 sessions
are in-memory; M2 adds cross-restart resume). The engine then always
attempts Range+If-Range and trusts only verified responses.

## When resume MUST restart

- Server returns 200 for a ranged request (Range ignored or validator
  mismatch).
- Server returns 416 with a total that doesn't match the local size.
- Content-Range start disagrees with the local size.
- The user chose Cancel (part discarded) — a later start is a fresh
  download by definition.

## ETag vs Last-Modified

`If-Range` prefers the ETag; Last-Modified is used only when no ETag
was seen (HTTP-date comparators are weaker, RFC 9110 §13.1.5). Both are
stored when present; comparison is exact-string.
