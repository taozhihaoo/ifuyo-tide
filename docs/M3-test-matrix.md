# M3 — Test Matrix and Results

All from real runs (2026-10-04, Windows 11 x64, librqbit 9.0.1,
loopback swarm — no public DHT/trackers in CI, M3 §44):

```text
cargo test --locked     # 207 passed, 0 failed
pnpm test / pnpm build  # PASS
pnpm tauri build        # PASS
```

## Integration tests (`tests/m3_torrent.rs`)

| # | Scenario (M3 §46) | Result |
|---|---|---|
| T01 | `.torrent` → metadata → file list | PASS |
| T02 | `.torrent` → full download → byte-exact verify (1 MiB) | PASS |
| T03 | multi-file + partial selection → selected only completes (M3 §82) | PASS |
| T04 | pause → bytes stop → resume → bytes continue (M3 §19) | PASS |
| T05/T06 | partial → crash → corrupt a byte → restart → piece re-verified + re-downloaded → final correct (M3 §22) | PASS |
| T07 | duplicate add → AlreadyManaged, no second download (M3 §17) | PASS |
| T08 | paths | covered by validation tests (unicode accepted, see source unit tests) |
| T09 | malicious paths rejected (traversal/absolute/reserved) | PASS |
| T10 | completed torrent → Tide task Completed | PASS |
| — | zero-byte file in multi-file torrent listed + completes (M2 §61 carry-over) | PASS |
| — | pause stops bytes within 500 ms (asserted, not just observed) | PASS |

## Security validation (M3 §12)

Traversal (`../`, `..\`), absolute (`C:\`, `C:/`), UNC (`\\server`),
device namespace (`\\?\`), reserved stems (CON/NUL/COM1…/LPT1… with or
without extension), trailing dot/space, illegal characters, empty →
all rejected at the path-validation layer, in unit tests
(`rejects_traversal_and_absolute`, `rejects_reserved_and_abusive_components`,
`validate_all_paths_rejects_whole_torrent`) and integration
(`malicious_torrent_paths_are_rejected`).

## Magnet tests (M3 §47)

Unit: malformed, missing xt, no v1 hash, truncated, over-long rejected;
valid btih parsed with trackers/name. Integration: metadata resolution
locally via initial_peers is supported by the engine (magnet add path
is exercised in recovery) but the full local-swarm magnet resolution is
marked MANUAL — public magnets are banned from CI (M3 §44/§88).

## Performance (M3 §57/§93)

1 torrent completes in ~2 s (1 MiB, 128 KiB pieces, loopback);
pause takes effect within one chunk; 10 concurrent torrents not
exercised in CI (M3 §67 marks it) — the 3-slot download gate bounds it.
