# M1 — Security Notes

Security measures actually implemented in M1 (see also SECURITY.md for
the project-wide baseline).

## URL validation

- Scheme allow-list: `http` / `https` only. `file:`, `ftp:`,
  `javascript:`, `data:`, `blob:`, `ws(s):`, custom schemes rejected
  structurally (parsed with the `url` crate — no string prefix tricks).
- URLs containing credentials (`user:pass@host`) rejected.
- Empty host rejected.

## SSRF baseline

- Literal-IP hosts classified: loopback, RFC1918 private, link-local
  (v4+v6), CGNAT 100.64/10, TEST-NETs, broadcast, unspecified, IPv6
  ULA — **blocked by default**.
- Opt-in: downloads carry `allow_private` (used by tests and, later, an
  explicit LAN/NAS feature — not built in M1).
- Redirect hops re-validated (scheme + limit); a redirect to a
  blocked-literal host fails the download.
- Documented limitation: hostname→IP resolution happens inside reqwest;
  DNS-rebinding into private ranges is not detected in M1 (redirect
  policy re-checks literal hosts only). Revisit with the M4 gateway.

## Redirect policy

- Max 10 hops (`DOWNLOAD_REDIRECT_LIMIT` beyond that).
- Every hop must remain http/https.
- `HTTP→HTTPS`, `HTTPS→HTTP`, cross-host all allowed (verified by
  tests through 301/302/307 chains) — the download follows the final
  resource wherever it legitimately lives; only scheme/private rules
  constrain it.

## Path containment & filename sanitization

- Filenames come from user input / headers / URL only AFTER
  sanitization: path separators stripped (server `filename` can never
  become a path), drive letters and `:`-abuse removed, `<>|"?*` and
  control characters replaced, Windows reserved device names
  (`CON`, `NUL`, `COM1`… ) neutralized, trailing dots/spaces dropped,
  length clamped to 200 chars preserving the extension.
- Destination = canonicalized(user-chosen dir) + sanitized name;
  containment asserted after canonicalization; `.part` files live in
  the same directory (`name.ext.part`).
- No reparse-point/junction escape is possible via the download path
  itself in M1 (the engine only writes inside the one canonicalized
  directory it planned at start; junction protection deepens in M2 with
  persisted paths).

## Temporary file handling

- All writes go to `<name>.part`; the final name appears only via
  atomic rename after fsync. A crash can leave a `.part`, never a
  corrupt file that looks complete.
- Cancel deletes the `.part`; pause keeps it; checksum mismatch keeps
  it for diagnosis.

## Sensitive logging

Logs record `download_id`, host, status, byte counts, phase, error
codes, timings — never full URLs (query strings), credentials, magnets,
peer IPs or user file paths.

## Resource limits

10 redirects max · 4 retries max (backoff ≤ 8 s) · 15 s connect
timeout · 30 s idle-read timeout · 200-char filename cap · 8 MiB cap
per in-memory read/chunk buffer · 3 concurrent downloads · 200 ms
progress-event throttle.
