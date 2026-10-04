# M5 Media Compatibility

Extends docs/M0.5-media-compatibility.md — the M0.5 matrix REMAINS the
authority (M5 §7: capability comes from measured facts, never
claims).

## Measured facts carried into M5 (M0.5 matrix, Windows 11 x64,
WebView2/Chromium 154)

| Container | Video | Audio | Verdict |
|---|---|---|---|
| MP4 | H.264 | AAC | SUPPORTED (TTFF 67–81 ms) |
| WebM | VP9 | Opus | SUPPORTED (TTFF 20–33 ms) |
| WebM | AV1 | Opus | SUPPORTED (TTFF 29–38 ms) |
| MKV | H.264 | AAC | SUPPORTED (no native player needed) |
| MP4 | HEVC | AAC | CONDITIONALLY_SUPPORTED (machine-dependent) |

## M5 runtime behavior

- **No format promises.** The player never claims codec support; a
  decode/src failure surfaces as the typed errors
  `UnsupportedMedia` / `DecodeError` with an honest message
  ("此媒体格式当前环境无法播放"), never a generic "播放失败" (§7/§32).
- **Capability probing** (§79): the element's `canPlayType` is
  available for UI hints but is explicitly NOT treated as a playback
  guarantee — the real decoder decides; M5 renders the outcome of
  real playback.
- **HEVC**: plays on machines with the extension/hardware decoder,
  fails cleanly as UnsupportedMedia elsewhere. Recorded, not worked
  around (no FFmpeg per §93/§94).
- **Autoplay**: `--autoplay-policy=no-user-gesture-required`
  (tauri.conf.json) is retained from M0.5; additionally every
  `play()` rejection is caught and mapped to a paused state — no
  unhandled rejections (§28).

## Test media (§62)

`scripts/generate-test-media.sh` regenerates the license-clean set
into `tests/fixtures/media/` (gitignored): MP4/H.264/AAC,
WebM/VP9/Opus, WebM/AV1/Opus, MKV/H.264/AAC, MP4/HEVC — 640×360, 90 s,
testsrc2+sine. The same set backs the M4 streaming suites; unsupported-
format behavior is covered by the error-model unit tests (media error
code 4 → UnsupportedMedia UI).
