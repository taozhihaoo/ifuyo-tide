# M8 Security Review (§87–§89/§142)

## Threat surfaces → mitigations → tests

| Threat (§142) | Mitigation | Test |
|---|---|---|
| S1 deep link arbitrary protocol | scheme fixed `tide`, command fixed `add`, param fixed magnet/url, strict percent-decode, 64 KiB cap | `tide_deeplink_parsed_and_validated`, oversized/malformed cases |
| S2 CLI arbitrary command execution | argv is CLASSIFIED, never executed; unknown shapes → structured rejection | `unsupported_scheme_rejected`, `invalid_arg_fails_batch` |
| S3 association arbitrary file access | canonicalize + regular-file + extension allowlist + 16 MiB torrent size cap; reserved device names rejected | `torrent_path_classification`, `media_path_classification` |
| S4 opener shell injection | official opener plugin only (§18/§41); the raw explorer spawn was REMOVED; path resolved backend-side from item ids | m6 library suites (item-id resolution) |
| S5 second-instance duplicate runtime | official plugin: second process exits inside the callback; no engines/SQLite created there | plugin contract + manual T1–T4 |
| S6 notification failure | frontend-only, fully swallowed (§39/§72) | notify.ts try/catch |
| S7 autostart failure | mapped to structured Resource error; UI shows it; core unaffected | degraded-mode design §72 |
| S8 corrupt window state | window-state plugin resets; min window size constraint | manual matrix (§36) |
| S9 uninstall user data | NSIS default uninstall does not delete user-chosen download roots; app data policy documented | installer smoke (manual §84) |
| S10 log leakage | launch logs carry type/code only — never magnet/URL/path/token (§30) | code audit of platform logs |

## Ownership / authorization (§88)

`scheduler_set_task_priority` validates the task exists in the live
snapshot; `library_reveal` resolves the path backend-side from the
item id; launch requests never bypass the confirm flow (§46).

## TOCTOU (§143)

Launch-time path validation narrows the window (canonicalize →
metadata → use); the gateway/library layers re-validate at use (M4/M6
invariants unchanged). No decision trusts a path string across phases.

## Logging (§30/§140)

`launch_request.rejected` carries only the error code. Tray/deep-link/
notification logs are event-name + code only. NO full magnets, URLs,
paths or tokens — audited against §30's list.

## Telemetry (§31)

None added. Notification/tray usage is not measured anywhere.
