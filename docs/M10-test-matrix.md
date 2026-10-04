# M10 Test Matrix (§31/§32/§33)

## Security test suites (automated, this milestone)

| Suite | Location | Cases |
|---|---|---|
| URL/SSRF matrix | `tests/security_corpus.rs` | scheme allowlist (7 rejects), private/loopback/link-local IPv4+IPv6 literals (7 hosts × allow/deny), numeric-IP encodings, redirect-target re-validation |
| Redirect security | `redirect_to_private_is_blocked` | loopback + link-local metadata blocked mid-chain; public hop allowed |
| Hostile string fuzz | `hostile_strings_never_panic` | null bytes, RTLO, emoji×1000, 100 KB strings, device paths, UNC, traversal — through URL/magnet validators ×2 policy settings, no panic |
| Gateway range abuse | `hostile_ranges_fail_closed` | overflow, inverted, multi, duplicate, malformed, empty — parser fails closed |
| Prior M4 gateway security matrix | `m4_streaming::gateway_security_matrix` + spike gateway suite | token/host/range/method matrices |
| Prior M6 filesystem matrix | `m6_library` + `m6_library_lower` | traversal, junction, containment, reserved names |

## Prior-suite regression (§91–§97)

All M0–M8 suites re-run after every M10 commit — see M9-regression-
matrix.md for the suite inventory; final counts in M10-report §19.

## Manual items (recorded, not silently assumed)

- §35 runtime packet-capture no-cloud verification (network surface
  documented from code inventory; a packet capture on the installed
  build is the TOOL VERIFIED step).
- §83 release smoke test on a clean machine (install → launch → task →
  restart → tray → deep link → association → autostart → uninstall).
- §77 DPI / multi-monitor.
