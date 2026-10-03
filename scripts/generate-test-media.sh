#!/usr/bin/env bash
# Generates self-made test media for the M0.5 technical spike.
#
# All output is generated locally with ffmpeg (testsrc2 + synthetic audio)
# and is therefore license-clean. Output goes to tests/fixtures/media/
# which is gitignored — regenerate with this script.
#
# Usage: scripts/generate-test-media.sh [path-to-ffmpeg]
set -euo pipefail

FF="${1:-ffmpeg}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/tests/fixtures/media"
DUR=90 # seconds of playable content for the playback matrix

mkdir -p "$OUT"

encode() {
  echo "[media] $1"
  "$FF" -y -hide_banner -loglevel error \
    -f lavfi -i "testsrc2=size=640x360:rate=30:duration=$DUR" \
    -f lavfi -i "sine=frequency=440:duration=$DUR" \
    "${@:2}" "$OUT/$1"
}

# --- playback matrix files ---
encode mp4_h264_aac.mp4 -c:v libx264 -preset ultrafast -pix_fmt yuv420p -c:a aac -b:a 96k -shortest
encode webm_vp9_opus.webm -c:v libvpx-vp9 -deadline realtime -cpu-used 8 -b:v 0 -crf 34 -c:a libopus -b:a 96k -shortest
encode webm_av1_opus.webm -c:v libsvtav1 -preset 10 -crf 38 -pix_fmt yuv420p -c:a libopus -b:a 96k -shortest
encode mkv_h264_aac.mkv -c:v libx264 -preset ultrafast -pix_fmt yuv420p -c:a aac -b:a 96k -shortest -f matroska
encode mp4_hevc_aac.mp4 -c:v libx265 -preset ultrafast -pix_fmt yuv420p -tag:v hvc1 -c:a aac -b:a 96k -shortest

# --- torrent fixture set (single-file seed + multi-file seed) ---
echo "[media] torrent_video_720p.mp4 (larger, for progressive streaming)"
"$FF" -y -hide_banner -loglevel error \
  -f lavfi -i "testsrc2=size=1280x720:rate=30:duration=120" \
  -f lavfi -i "sine=frequency=330:duration=120" \
  -c:v libx264 -preset ultrafast -pix_fmt yuv420p -c:a aac -b:a 96k \
  -shortest "$OUT/torrent_video_720p.mp4"

echo "[media] subtitle.srt / cover.jpg / other.bin"
{
  echo "1"
  echo "00:00:01,000 --> 00:00:04,000"
  echo "Tide M0.5 spike subtitle line one."
  echo ""
  echo "2"
  echo "00:00:05,000 --> 00:00:08,000"
  echo "Second line for the multi-file torrent fixture."
} > "$OUT/subtitle.srt"
"$FF" -y -hide_banner -loglevel error \
  -f lavfi -i "testsrc2=size=320x180:rate=1:duration=1" \
  -frames:v 1 "$OUT/cover.jpg"
head -c $((4 * 1024 * 1024)) /dev/urandom > "$OUT/other.bin"

echo "[media] done:"
ls -la "$OUT"
