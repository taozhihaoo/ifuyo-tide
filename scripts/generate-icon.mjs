// Generates src-tauri/icons/icon.ico — a 64x64 "Tide" mark.
//
// The ICO embeds a raw 32-bit BMP (not PNG-compressed) so the Windows
// resource compiler (rc.exe via tauri-build / winres) accepts it.
// Run: node scripts/generate-icon.mjs

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 64;
const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "src-tauri", "icons", "icon.ico");

// ---- draw: dark rounded square + white "T" with a wave cut ----
const px = new Uint8Array(SIZE * SIZE * 4); // RGBA

function set(x, y, r, g, b, a = 255) {
  const i = (y * SIZE + x) * 4;
  px[i] = r;
  px[i + 1] = g;
  px[i + 2] = b;
  px[i + 3] = a;
}

function rect(x0, y0, x1, y1, r, g, b, a = 255) {
  for (let y = Math.max(0, y0); y < Math.min(SIZE, y1); y++)
    for (let x = Math.max(0, x0); x < Math.min(SIZE, x1); x++) set(x, y, r, g, b, a);
}

// background: #1c1f25 with 10px corner radius, 1px transparent margin
const BG = [0x1c, 0x1f, 0x25];
rect(2, 2, 62, 62, ...BG);
for (const [cx, cy] of [
  [11, 11],
  [52, 11],
  [11, 52],
  [52, 52],
]) {
  for (let y = 2; y < 62; y++)
    for (let x = 2; x < 62; x++) {
      const dx = x - cx;
      const dy = y - cy;
      const outsideCorner =
        (cx < 32 ? x < cx : x > cx) && (cy < 32 ? y < cy : y > cy);
      if (outsideCorner && dx * dx + dy * dy > 81) set(x, y, 0, 0, 0, 0);
    }
}

// white "T"
const W = [0xf2, 0xf4, 0xf7];
rect(14, 18, 50, 24, ...W); // bar
rect(29, 24, 35, 48, ...W); // stem

// wave through the stem (background color)
rect(26, 38, 38, 41, ...BG);
rect(29, 35, 35, 38, ...W);

// ---- encode ICO with raw BITMAPINFOHEADER payload ----
const xor = Buffer.alloc(SIZE * SIZE * 4); // BGRA, bottom-up
const andMaskStride = Math.ceil(SIZE / 32) * 4; // bytes per AND row
const and = Buffer.alloc(andMaskStride * SIZE); // bottom-up

for (let y = 0; y < SIZE; y++) {
  const srcRow = y;
  const dstRow = SIZE - 1 - y;
  for (let x = 0; x < SIZE; x++) {
    const s = (srcRow * SIZE + x) * 4;
    const d = (dstRow * SIZE + x) * 4;
    xor[d] = px[s + 2];
    xor[d + 1] = px[s + 1];
    xor[d + 2] = px[s];
    xor[d + 3] = px[s + 3];
    if (px[s + 3] === 0) {
      and[dstRow * andMaskStride + (x >> 3)] |= 0x80 >> (x & 7);
    }
  }
}

const bmp = Buffer.alloc(40);
bmp.writeUInt32LE(40, 0); // biSize
bmp.writeInt32LE(SIZE, 4); // biWidth
bmp.writeInt32LE(SIZE * 2, 8); // biHeight (XOR + AND)
bmp.writeUInt16LE(1, 12); // biPlanes
bmp.writeUInt16LE(32, 14); // biBitCount
// compression BI_RGB (0), rest zero

const image = Buffer.concat([bmp, xor, and]);

const icondir = Buffer.alloc(6);
icondir.writeUInt16LE(0, 0); // reserved
icondir.writeUInt16LE(1, 2); // type: icon
icondir.writeUInt16LE(1, 4); // count

const entry = Buffer.alloc(16);
entry[0] = SIZE; // width
entry[1] = SIZE; // height
entry[2] = 0; // palette
entry[3] = 0; // reserved
entry.writeUInt16LE(1, 4); // planes
entry.writeUInt16LE(32, 6); // bpp
entry.writeUInt32LE(image.length, 8);
entry.writeUInt32LE(6 + 16, 12); // data offset

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, Buffer.concat([icondir, entry, image]));
console.log(`wrote ${OUT} (${6 + 16 + image.length} bytes)`);
