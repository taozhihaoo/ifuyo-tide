# tests/fixtures

Test media and payloads for Tide.

The `media/` subdirectory is **generated, not committed**: run
`scripts/generate-test-media.sh <path-to-ffmpeg>` to (re)create it. All
files there are synthesized locally (ffmpeg `testsrc2` + sine audio) and
therefore license-clean; see `scripts/generate-test-media.sh` for the
exact recipes and `docs/M0.5-*.md` for what each file is used for.

**Allowed in this directory:**

- self-made test media (generated tones, test patterns, synthetic videos)
- public-domain material
- material under an explicit open license that permits redistribution

**Never place here:**

- commercial movies or TV shows
- copyrighted music
- pirated games or software
- any content whose license does not clearly allow redistribution in a
  public repository

The same rules apply to files generated at test runtime (see the overall
design document, "Deterministic Test Swarm" / "合规测试数据原则").
