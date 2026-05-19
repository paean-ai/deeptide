// Pixel Dungeon Delve - pixel art. Sprites are authored on a 16x16 grid and
// scaled to TILE so the dungeon stays crisp at any canvas size.

const U = TILE / 16; // logical pixels per art-pixel

// px(): draw an art-pixel rect within the tile whose top-left is (ox, oy).
function px(ctx, ox, oy, x, y, w, h, c) {
  ctx.fillStyle = c;
  ctx.fillRect(ox + x * U, oy + y * U, w * U, h * U);
}

function shadeHex(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// --- terrain -----------------------------------------------------------
function drawFloor(ctx, ox, oy, variant) {
  px(ctx, ox, oy, 0, 0, 16, 16, '#262533');
  px(ctx, ox, oy, 0, 0, 16, 1, '#312f42');
  px(ctx, ox, oy, 0, 15, 16, 1, '#1c1b27');
  // deterministic speckle so floors don't shimmer between renders
  const spots = [[3, 4], [11, 2], [6, 11], [13, 9], [9, 6]];
  const s = spots[variant % spots.length];
  px(ctx, ox, oy, s[0], s[1], 2, 2, '#2f2e3e');
  px(ctx, ox, oy, (s[0] + 7) % 14, (s[1] + 6) % 14, 1, 1, '#34324a');
}

function drawWall(ctx, ox, oy) {
  px(ctx, ox, oy, 0, 0, 16, 16, '#403a52');
  px(ctx, ox, oy, 0, 0, 16, 2, '#544d6c');     // top light
  px(ctx, ox, oy, 0, 14, 16, 2, '#2c2740');    // base shadow
  px(ctx, ox, oy, 7, 0, 1, 16, '#322d44');     // mortar vertical
  px(ctx, ox, oy, 0, 7, 16, 1, '#322d44');     // mortar horizontal
  px(ctx, ox, oy, 1, 1, 5, 5, '#4a4360');
  px(ctx, ox, oy, 9, 9, 5, 5, '#4a4360');
}

function drawStairs(ctx, ox, oy) {
  px(ctx, ox, oy, 0, 0, 16, 16, '#1a1925');
  for (let i = 0; i < 5; i++) {
    px(ctx, ox, oy, 2 + i, 3 + i * 2, 12 - i, 2, '#5b5470');
    px(ctx, ox, oy, 2 + i, 3 + i * 2, 12 - i, 1, '#76708e');
  }
  px(ctx, ox, oy, 6, 1, 4, 2, '#9c84ff'); // glow lip
}

// --- entities ----------------------------------------------------------
function withFlash(ctx, flash, fn) {
  fn();
  if (flash) {
    ctx.globalCompositeOperation = 'lighter';
    ctx.fillStyle = 'rgba(255,120,120,0.5)';
    ctx.fillRect(ctx._fx, ctx._fy, TILE, TILE);
    ctx.globalCompositeOperation = 'source-over';
  }
}

function drawHero(ctx, ox, oy, facing, flash) {
  ctx._fx = ox; ctx._fy = oy;
  withFlash(ctx, flash, () => {
    const f = facing >= 0 ? 1 : -1;
    const mir = (x, w) => f > 0 ? x : 16 - x - w;
    // shadow
    ctx.fillStyle = 'rgba(0,0,0,0.35)';
    ctx.fillRect(ox + 4 * U, oy + 14 * U, 8 * U, 2 * U);
    px(ctx, ox, oy, mir(5, 6), 9, 6, 5, '#3b5fb0');     // legs/tunic
    px(ctx, ox, oy, mir(4, 8), 4, 8, 6, '#5479d8');     // body
    px(ctx, ox, oy, mir(4, 8), 4, 8, 2, '#7e9cea');     // body light
    px(ctx, ox, oy, mir(5, 6), 0, 6, 5, '#e7b88a');     // head
    px(ctx, ox, oy, mir(5, 0), 0, 6, 2, '#caa06f');     // hair
    px(ctx, ox, oy, mir(8, 2), 2, 2, 2, '#1c1726');     // eye
    // sword in lead hand
    px(ctx, ox, oy, mir(12, 2), 1, 2, 9, '#d9def0');
    px(ctx, ox, oy, mir(11, 4), 9, 4, 2, '#8a6b3a');
  });
}

function drawEnemy(ctx, ox, oy, glyph, flash, t) {
  ctx._fx = ox; ctx._fy = oy;
  withFlash(ctx, flash, () => {
    ctx.fillStyle = 'rgba(0,0,0,0.32)';
    ctx.fillRect(ox + 4 * U, oy + 14 * U, 8 * U, 2 * U);
    if (glyph === 'rat') {
      px(ctx, ox, oy, 3, 8, 9, 5, '#7d6a55'); px(ctx, ox, oy, 11, 6, 4, 4, '#8d7a64');
      px(ctx, ox, oy, 13, 7, 1, 1, '#ff5d5d'); px(ctx, ox, oy, 0, 9, 4, 1, '#5c4f40');
    } else if (glyph === 'bat') {
      const flap = Math.floor(t * 6) % 2 ? 2 : 5;
      px(ctx, ox, oy, 6, 6, 4, 5, '#5b4a7a');
      px(ctx, ox, oy, 1, flap, 5, 4, '#473a63'); px(ctx, ox, oy, 10, flap, 5, 4, '#473a63');
      px(ctx, ox, oy, 7, 7, 1, 1, '#ff5d5d'); px(ctx, ox, oy, 8, 7, 1, 1, '#ff5d5d');
    } else if (glyph === 'skel') {
      px(ctx, ox, oy, 5, 1, 6, 5, '#e6e3d4'); px(ctx, ox, oy, 6, 3, 1, 2, '#1c1726');
      px(ctx, ox, oy, 9, 3, 1, 2, '#1c1726'); px(ctx, ox, oy, 5, 7, 6, 6, '#cdc9b6');
      px(ctx, ox, oy, 5, 8, 6, 1, '#9a9684'); px(ctx, ox, oy, 12, 4, 2, 9, '#d9def0');
    } else if (glyph === 'orc') {
      px(ctx, ox, oy, 3, 2, 10, 5, '#5f9143'); px(ctx, ox, oy, 3, 2, 10, 2, '#79ad57');
      px(ctx, ox, oy, 5, 4, 2, 2, '#ffe14d'); px(ctx, ox, oy, 9, 4, 2, 2, '#ffe14d');
      px(ctx, ox, oy, 5, 6, 1, 1, '#fff'); px(ctx, ox, oy, 10, 6, 1, 1, '#fff');
      px(ctx, ox, oy, 2, 7, 12, 7, '#4d7637'); px(ctx, ox, oy, 13, 3, 3, 11, '#7d7d86');
    } else if (glyph === 'wraith') {
      ctx.globalAlpha = 0.85;
      const sw = Math.sin(t * 4) * 1;
      px(ctx, ox, oy, 4 + sw, 1, 8, 7, '#7d5fb8'); px(ctx, ox, oy, 3 + sw, 7, 10, 6, '#5c4490');
      px(ctx, ox, oy, 2 + sw, 12, 12, 3, '#3f2f68');
      px(ctx, ox, oy, 6 + sw, 3, 2, 2, '#b6ffe6'); px(ctx, ox, oy, 9 + sw, 3, 2, 2, '#b6ffe6');
      ctx.globalAlpha = 1;
    } else if (glyph === 'dragon') {
      px(ctx, ox, oy, 1, 5, 14, 9, '#a8332f'); px(ctx, ox, oy, 1, 5, 14, 3, '#cf4a44');
      px(ctx, ox, oy, 9, 0, 7, 7, '#c0403a'); px(ctx, ox, oy, 13, 2, 2, 2, '#ffe14d');
      px(ctx, ox, oy, 9, 2, 3, 2, '#5a1a17'); // mouth
      px(ctx, ox, oy, 0, 1, 5, 6, '#7c2723'); px(ctx, ox, oy, 4, 2, 4, 4, '#7c2723'); // wing
      px(ctx, ox, oy, 2, 13, 3, 3, '#ffe14d'); px(ctx, ox, oy, 11, 13, 3, 3, '#ffe14d'); // claws
    }
  });
}

function drawItem(ctx, ox, oy, kind, t) {
  const bob = Math.round(Math.sin(t * 3) * U);
  oy += bob;
  if (kind === 'potion') {
    px(ctx, ox, oy, 6, 3, 4, 2, '#cfd6e8');
    px(ctx, ox, oy, 5, 5, 6, 8, '#ff5d7a'); px(ctx, ox, oy, 6, 6, 2, 3, '#ffb0bf');
  } else if (kind === 'gold') {
    px(ctx, ox, oy, 5, 7, 7, 5, '#f4c85a'); px(ctx, ox, oy, 5, 7, 7, 2, '#ffe6a8');
    px(ctx, ox, oy, 6, 5, 4, 3, '#f4c85a'); px(ctx, ox, oy, 9, 9, 2, 2, '#b8902f');
  } else if (kind === 'weapon') {
    px(ctx, ox, oy, 7, 1, 2, 10, '#d9def0'); px(ctx, ox, oy, 7, 1, 1, 10, '#ffffff');
    px(ctx, ox, oy, 5, 11, 6, 2, '#8a6b3a'); px(ctx, ox, oy, 7, 13, 2, 2, '#8a6b3a');
  } else if (kind === 'armor') {
    px(ctx, ox, oy, 4, 3, 8, 9, '#7d8aa8'); px(ctx, ox, oy, 4, 3, 8, 2, '#9eabc8');
    px(ctx, ox, oy, 4, 3, 2, 3, '#5e6986'); px(ctx, ox, oy, 10, 3, 2, 3, '#5e6986');
    px(ctx, ox, oy, 7, 6, 2, 4, '#5e6986');
  }
}
