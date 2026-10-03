# M3 Completion Report

## 1. Executive Summary

M3 adds a real BitTorrent Download Core on the validated M0.5
foundation: `Magnet/.torrent → validation → metadata → file selection →
librqbit (single session, loopback-seeded swarm in tests) → piece
download → verification → Completed task`. Seven integration tests run
a REAL loopback swarm (seeder session + production TorrentEngine) and
pass; the download-path security checklist is enforced at the
validation layer and proven by unit + integration tests.

## 2. FACTS

- librqbit 9.0.1 locked; APIs verified against vendored source.
- All M0/M1/M2 suites still pass; 207 Rust tests / 3 frontend tests, 0
  failures.
- `disable-upload` cargo feature + session flag verified to exist and
  to compile; swarm tests run with DownloadOnly active.

## 3. CURRENT M0/M1/M2 STATE

Unchanged behavior: HTTP downloads (M1), persistence/recovery (M2) —
full regression 0 failures. Torrent code lives in `core/torrent` +
`commands/torrent.rs` (production) and `spike/` remains quarantined.

## 4. librqbit Version / Dependency Audit

9.0.1 crates.io, Apache-2.0, locked. Features added: `disable-upload`,
`stream`. No version bump (M0.5-validated). Details:
M3-librqbit-integration.md.

## 5. Architecture Changes

`core/torrent/` (engine/source) + TorrentRepository + torrent IPC; no
librqbit types outside `core/torrent`; app wiring creates ONE session
at startup and shuts it down with the app.

## 6. TorrentEngine Design

M3-torrent-architecture.md. Methods: inspect / add (bytes+magnet) /
wait_metadata / files / pause / resume / set_only_files / remove /
stats / shutdown.

## 7. Source Model

`TorrentSource::Magnet{uri}` / `TorrentFile{bytes}`. Magnet parsing
delegated to librqbit (`Magnet::parse`), re-validated (v1 hash
required, length cap). `.torrent` imported to a stable app-data copy.

## 8. Metadata Model

info_hash (hex), name, total_size, piece_length (via
`ValidatedTorrentMetaV1Info::info()`), files[index, relative_path,
size, offset] — offsets preserved for M4's logical-byte-space model
(piece boundaries may cross files).

## 9. File Selection

only_files → librqbit; persisted in `torrent_files.selected`;
partial selection completes the task (tested).

## 10. Storage Architecture

Single-file → destination dir directly; multi-file → librqbit's root
folder layout; session state under app-data. No mixing with Tide's
database/logs (M3 §36).

## 11. Persistence Architecture

`torrent_tasks` + `torrent_files` tables (migration 0003); selection
identity = task_id+info_hash+file_index; metadata copies on disk with
SHA-256 recorded (M3 §85).

## 12. Recovery Algorithm

Startup: re-validate stored metadata → re-add PAUSED to session
(idempotent: AlreadyManaged reuses) → terminal states restored without
re-adding → paused/failed/cancelled keep user intent; missing metadata
copy or hostile paths surface as structured failures, never panics.

## 13. Tide ↔ librqbit State Mapping

Downloading↔Live, Paused↔Paused, Completed↔finished, Failed↔error;
librqbit-internal phases (Initializing/Checking/MetadataFetching) map
to TorrentPhase, not TaskState (M3 §18/§7).

## 14. Upload / Sharing Policy

DownloadOnly technically guaranteed (feature + session flag);
uploaded_bytes asserted 0 during swarm downloads; no anonymity claims.

## 15. Network Configuration

UPnP off, no HTTP API/prometheus/WebUI, TCP listener on OS-assigned
port, DHT on in production.

## 16. Security Audit

Path validation battery (traversal/absolute/reserved/trailing/
illegal/length) — unit + integration; metadata re-validated at
recovery; hostile torrent → structured rejection; no peer IPs or
magnets in logs.

## 17. Test Matrix

M3-test-matrix.md: 7 swarm integration tests + validation unit tests +
full M0/M1/M2 regression (207/0).

## 18. Crash / Failure Injection

pause-stop assertion, drop-engine-mid-download + corrupted-byte restart
(piece re-verification proven byte-exact), duplicate add, metadata
missing at recovery (structured failure).

## 19. Performance Results

1 MiB loopback swarm: complete ≈ 2 s; pause takes effect < 500 ms
(asserted); 2 MiB corrupted-piece recovery included in suite runtime
(~2.3 s total for 7 tests).

## 20. Manual Validation

GUI cases (§87 A–F) remain for the user: `pnpm tauri dev` → Torrents
view provides the same IPC flows the tests drive.

## 21. Files Changed

See git range `55e0178..HEAD` (M3): core/torrent/* (engine/source/
recovery/mod), commands/torrent.rs, app wiring, migration 0003,
torrent_repository, frontend torrents view + api, tests/m3_torrent.rs,
docs/M3-*.md ×7.

## 22. FACT / HYPOTHESIS / INFERENCE

- FACT: local-swarm downloads, selection, pause/resume, duplicate
  reuse, corruption recovery all pass deterministically.
- FACT: public-swarm behavior is NOT tested in CI (no public torrents
  permitted) — DHT/trackers enabled but unverified against real swarms.
- INFERENCE: M4 availability snapshots can read `stats()`/file_progress
  directly.

## 23. RESOLVED / CONFIRMED / OPEN / WORSENED

- [RESOLVED] Torrent task ↔ engine mapping without duplicating librqbit
- [RESOLVED] DownloadOnly with technical guarantee
- [CONFIRMED] Corruption recovery via librqbit re-verification
- [OPEN] Real public-swarm soak test (needs network-enabled manual run)
- [OPEN] Uploaded-bytes-remain-zero long-run proof (short tests show 0)
- [WORSENED] none

## 24. P0 / P1 Risks

P0: none known. P1: public-swarm soak untested; system-proxy still
disabled; magnet local resolution is implemented but not CI-tested.

## 25. Known Limitations

Magnet metadata persistence across restart re-resolves (no fast-resume
file for magnets); no torrent URL source (P1, M3 §24); file-selection
UI is index-based (list, not tree).

## 26. M4 Readiness

`TorrentMetadata.files` carry offsets in the torrent's logical byte
space; `stats().file_progress` is per file; the spike already proved
`ManagedTorrent::stream(file_id)` (tokio AsyncRead+AsyncSeek) — the
data basis for `Resource.read(range)`/Availability is in place.

## 27. Definition-of-Done Checklist

All §94 items verified: Source ✓ Metadata ✓ Download ✓ Selection ✓
State ✓ Persistence ✓ Verification ✓ Security ✓ Architecture ✓
Compatibility (M0/M1/M2 suites pass) ✓ Tests ✓ Build ✓.
