// Pixel Merge Garden - crop pixel art. Each crop is drawn on a 24x24 logical
// grid and scaled up crisply, so every tier has a readable silhouette.

const PX = 24;

function shade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function p(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x, y, w, h); }

// Terracotta pot - identical across every crop for visual consistency.
function drawPot(ctx) {
  p(ctx, 5, 14, 14, 2, '#d98a52');           // rim
  p(ctx, 5, 14, 14, 1, '#f0a972');           // rim highlight
  p(ctx, 6, 16, 12, 7, '#c8703f');           // body
  p(ctx, 6, 16, 2, 7, '#e0935c');            // body light edge
  p(ctx, 16, 16, 2, 7, '#9f5530');           // body shadow edge
  p(ctx, 7, 12, 10, 2, '#3a2b1c');           // soil
  p(ctx, 8, 12, 8, 1, '#4d3a25');            // soil highlight
}

function drawStem(ctx, c, top) {
  for (let y = top; y < 13; y++) p(ctx, 11, y, 2, 1, c.stem);
  p(ctx, 11, top, 1, 13 - top, shade(c.stem, 28)); // lit edge
}

function leafPair(ctx, c, y) {
  p(ctx, 7, y, 4, 3, c.leaf);
  p(ctx, 13, y, 4, 3, c.leaf);
  p(ctx, 7, y, 4, 1, c.glow);
  p(ctx, 13, y, 4, 1, c.glow);
}

function drawTopper(ctx, c, kind) {
  const L = c.leaf, D = c.stem, H = c.glow;
  if (kind === 'sprout') {
    drawStem(ctx, c, 7);
    p(ctx, 6, 5, 5, 3, L); p(ctx, 13, 5, 5, 3, L);
    p(ctx, 6, 5, 5, 1, H); p(ctx, 13, 5, 5, 1, H);
  } else if (kind === 'leaf') {
    drawStem(ctx, c, 5);
    p(ctx, 9, 2, 6, 5, L); p(ctx, 5, 6, 5, 4, L); p(ctx, 14, 6, 5, 4, L);
    p(ctx, 10, 3, 4, 2, H);
  } else if (kind === 'bud') {
    drawStem(ctx, c, 6); leafPair(ctx, c, 9);
    p(ctx, 9, 2, 6, 6, L); p(ctx, 10, 1, 4, 1, L);
    p(ctx, 9, 2, 2, 6, H); p(ctx, 11, 3, 2, 2, shade(L, 40));
  } else if (kind === 'bloom') {
    drawStem(ctx, c, 7); leafPair(ctx, c, 10);
    p(ctx, 9, 1, 6, 3, L); p(ctx, 9, 7, 6, 3, L);   // top/bottom petals
    p(ctx, 5, 4, 4, 4, L); p(ctx, 15, 4, 4, 4, L);  // side petals
    p(ctx, 9, 1, 6, 1, H); p(ctx, 5, 4, 1, 4, H);
    p(ctx, 9, 4, 6, 4, '#fff2c0'); p(ctx, 10, 5, 4, 2, '#f6c64a'); // core
  } else if (kind === 'fruit') {
    drawStem(ctx, c, 7); leafPair(ctx, c, 9);
    p(ctx, 7, 2, 10, 7, L); p(ctx, 8, 1, 8, 1, L); p(ctx, 8, 9, 8, 1, L);
    p(ctx, 8, 2, 3, 3, H); p(ctx, 14, 6, 2, 2, shade(L, -40));
  } else if (kind === 'berry') {
    drawStem(ctx, c, 8); leafPair(ctx, c, 8);
    const dot = (x, y) => { p(ctx, x, y, 4, 4, L); p(ctx, x, y, 2, 2, H); p(ctx, x + 2, y + 2, 2, 2, shade(L, -40)); };
    dot(6, 3); dot(14, 3); dot(10, 6);
  } else if (kind === 'star') {
    drawStem(ctx, c, 8); leafPair(ctx, c, 9);
    p(ctx, 10, 0, 4, 11, L); p(ctx, 5, 5, 14, 4, L);   // cross
    p(ctx, 8, 3, 8, 8, L);                              // body
    p(ctx, 10, 1, 2, 4, H); p(ctx, 9, 5, 4, 2, '#ffffff');
  }
}

// Paints a crop onto its cell canvas. `crop` is { level, mutation, wild }.
function paintCrop(canvas, crop) {
  canvas.width = PX; canvas.height = PX;
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, PX, PX);
  const art = cropArt(crop.level);
  drawPot(ctx);
  if (crop.wild) {
    // Wild crop: rainbow shimmer body, merges with anything.
    const bands = ['#ff6b8b', '#ffb454', '#ffe14d', '#62d879', '#64c7ff', '#aa7dff'];
    bands.forEach((b, i) => p(ctx, 6, 1 + i * 1.6, 12, 2, b));
    p(ctx, 8, 0, 8, 1, '#ffffff');
    drawStem(ctx, { stem: '#5b8f5b' }, 11);
  } else {
    drawTopper(ctx, art, art.topper);
  }
  // Prestige rings - one corner gem per palette loop completed.
  const prestige = cropPrestige(crop.level);
  for (let i = 0; i < Math.min(prestige, 4); i++) {
    const cx = i % 2 ? PX - 4 : 1, cy = i < 2 ? 1 : PX - 4;
    p(ctx, cx, cy, 3, 3, '#ffe14d');
    p(ctx, cx, cy, 1, 1, '#ffffff');
  }
  // Mutation frame.
  if (crop.mutation && crop.mutation !== 'plain') {
    const mc = MUTATIONS[crop.mutation].color;
    ctx.strokeStyle = mc;
    ctx.lineWidth = 1;
    ctx.strokeRect(1.5, 1.5, PX - 3, PX - 3);
    p(ctx, PX - 5, 2, 3, 3, mc);
    p(ctx, PX - 5, 2, 1, 1, '#ffffff');
  }
}
