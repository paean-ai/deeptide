// Starforge Idle - pixel art: the animated star forge and per-item icons.
// The forge is authored on an 80x80 grid (4px pixels) so it stays crisp.

const FU = 4; // forge pixel unit (80 * 4 = 320)

function fShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function fp(ctx, gx, gy, gw, gh, c) {
  ctx.fillStyle = c;
  ctx.fillRect(Math.round(gx * FU), Math.round(gy * FU), Math.ceil(gw * FU), Math.ceil(gh * FU));
}

// Deterministic background star positions.
const FORGE_STARS = [];
for (let i = 0; i < 26; i++) {
  FORGE_STARS.push({ x: (i * 47 + 11) % 78 + 1, y: (i * 29 + 7) % 44 + 1, ph: i * 0.7 });
}

// Draws the full forge scene. strike 0..1 (hammer slam), heat 0..1 (auto rate),
// sparks = array of {x,y,vx,vy,life,color} in canvas pixels.
function renderForge(ctx, time, strike, heat, sparks, mining) {
  ctx.clearRect(0, 0, 320, 320);
  // backdrop
  fp(ctx, 0, 0, 80, 80, '#070a12');
  for (let y = 0; y < 80; y += 4) fp(ctx, 0, y, 80, 2, y % 8 === 0 ? '#0c1320' : '#0a0f1a');
  // twinkling stars
  for (const s of FORGE_STARS) {
    const tw = 0.4 + 0.6 * Math.abs(Math.sin(time * 1.6 + s.ph));
    ctx.globalAlpha = tw;
    fp(ctx, s.x, s.y, 1, 1, s.ph % 2 ? '#9fd8ff' : '#fff2c0');
  }
  ctx.globalAlpha = 1;

  // ground glow from the forge heat
  const glow = 0.12 + heat * 0.5 + (mining ? 0.25 : 0);
  ctx.globalAlpha = Math.min(0.8, glow);
  fp(ctx, 14, 40, 52, 30, '#3a2a18');
  ctx.globalAlpha = 1;

  // --- anvil ---
  fp(ctx, 26, 61, 28, 9, '#2a3242');     // base block
  fp(ctx, 26, 61, 28, 2, '#3c4658');
  fp(ctx, 28, 70, 24, 2, '#11151d');     // base shadow
  fp(ctx, 33, 55, 14, 6, '#39414f');     // waist
  fp(ctx, 21, 47, 38, 8, '#4a5567');     // top body
  fp(ctx, 21, 47, 38, 2, '#5d6a80');     // top highlight
  fp(ctx, 12, 48, 10, 5, '#4a5567');     // horn
  fp(ctx, 12, 48, 10, 2, '#5d6a80');
  fp(ctx, 21, 53, 38, 2, '#222a36');     // top underside shadow

  // --- star core on the anvil ---
  const pulse = Math.sin(time * 4) * 1.2 + (mining ? 1.4 : 0);
  const cx = 40, cy = 38;
  const coreHot = strike > 0.15;
  const haloC = coreHot ? '#ffe6a0' : '#67e7ff';
  ctx.globalAlpha = 0.22 + heat * 0.18;
  fp(ctx, cx - 16, cy - 16, 32, 32, haloC);
  ctx.globalAlpha = 1;
  // diamond core
  const cr = 7 + pulse;
  for (let i = 0; i < cr; i++) {
    const w = (cr - i) * 2;
    const col = coreHot ? '#ffd24d' : '#aa7dff';
    fp(ctx, cx - (cr - i), cy - i - 1, w, 1, col);
    fp(ctx, cx - (cr - i), cy + i, w, 1, fShade(col, -40));
  }
  fp(ctx, cx - 3, cy - 3, 6, 5, coreHot ? '#fff6d0' : '#cde6ff'); // hot centre
  fp(ctx, cx - 2, cy - 6, 4, 3, '#ffffff');

  // --- hammer ---
  // rests raised, swings down on strike. strike 1 = fully down.
  const swing = strike > 0 ? Math.pow(strike, 0.6) : 0;
  const bob = Math.sin(time * 2.5) * 0.04;
  const angle = -1.05 + bob + swing * 1.35;
  ctx.save();
  ctx.translate((cx + 4) * FU, (cy - 4) * FU);
  ctx.rotate(angle);
  // handle
  ctx.fillStyle = '#7a5230';
  ctx.fillRect(0, -2 * FU, 26 * FU, 4 * FU);
  ctx.fillStyle = '#9a6a40';
  ctx.fillRect(0, -2 * FU, 26 * FU, 1.4 * FU);
  // head
  ctx.fillStyle = '#3a4150';
  ctx.fillRect(22 * FU, -8 * FU, 14 * FU, 16 * FU);
  ctx.fillStyle = '#565f72';
  ctx.fillRect(22 * FU, -8 * FU, 14 * FU, 4 * FU);
  ctx.fillStyle = '#20262f';
  ctx.fillRect(22 * FU, 4 * FU, 14 * FU, 4 * FU);
  ctx.fillStyle = '#6f7a8e';
  ctx.fillRect(24 * FU, -6 * FU, 3 * FU, 3 * FU);
  ctx.restore();

  // --- embers + sparks ---
  const emberN = Math.round(heat * 7);
  for (let i = 0; i < emberN; i++) {
    const ey = (cy + 4) - ((time * 14 + i * 13) % 30);
    const ex = cx - 6 + ((i * 17) % 13) + Math.sin(time * 3 + i) * 2;
    ctx.globalAlpha = Math.max(0, (ey - (cy - 26)) / 30);
    fp(ctx, ex, ey, 1, 1, i % 2 ? '#ffd24d' : '#ff8a3c');
  }
  ctx.globalAlpha = 1;
  for (const s of sparks) {
    ctx.globalAlpha = Math.max(0, s.life);
    ctx.fillStyle = s.color;
    ctx.fillRect(s.x | 0, s.y | 0, 4, 4);
  }
  ctx.globalAlpha = 1;
}

// Spawns a burst of sparks at the core (canvas pixel coords ~160,152).
function forgeSparks() {
  const out = [];
  for (let i = 0; i < 16; i++) {
    const a = -Math.PI / 2 + (Math.random() - 0.5) * 2.4;
    const sp = 60 + Math.random() * 200;
    out.push({ x: 160, y: 150, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      life: 1, color: i % 3 ? '#ffd24d' : '#fff2c0' });
  }
  return out;
}

// --- per-item pixel icons (54x54, authored on an 18x18 grid) -------------
function drawItemIcon(ctx, item, lv) {
  const U = 3;
  const p = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(x * U, y * U, w * U, h * U); };
  const c = item.color, dk = fShade(c, -56), lt = fShade(c, 50);
  // tile background
  p(0, 0, 18, 18, '#0c1320');
  p(0, 0, 18, 2, '#141d2e');
  p(0, 16, 18, 2, '#070a12');
  const metal = '#9aa6bc', metalD = '#5c6577', wood = '#7a5230';

  if (item.id === 'pick') {
    p(8, 3, 2, 12, wood);
    p(3, 4, 12, 3, metal); p(3, 4, 12, 1, lt);
    p(3, 4, 3, 5, metal); p(12, 4, 3, 5, metal);
  } else if (item.id === 'glove') {
    p(5, 5, 8, 9, c); p(5, 5, 8, 2, lt);
    p(4, 6, 2, 5, c); p(5, 2, 2, 4, c); p(7, 2, 2, 4, c); p(9, 2, 2, 4, c); p(11, 2, 2, 4, c);
    p(5, 12, 8, 2, dk);
  } else if (item.id === 'smelter' || item.id === 'foundry') {
    p(3, 4, 12, 11, metalD); p(3, 4, 12, 2, metal);
    p(6, 7, 6, 6, '#1a0f08'); p(7, 8, 4, 4, c); p(8, 9, 2, 2, '#fff2d0');
    p(6, 2, 3, 3, metalD); p(10, 1, 3, 4, metalD);
  } else if (item.id === 'lens') {
    p(4, 4, 10, 10, dk); p(5, 5, 8, 8, c); p(6, 6, 4, 4, lt); p(7, 7, 2, 2, '#ffffff');
  } else if (item.id === 'drone') {
    p(6, 7, 6, 5, metalD); p(6, 7, 6, 2, metal);
    p(2, 6, 4, 2, c); p(12, 6, 4, 2, c);
    p(7, 8, 4, 2, c); p(8, 12, 2, 3, metalD);
  } else if (item.id === 'rig') {
    p(6, 2, 6, 4, metalD); p(8, 6, 2, 7, metal); p(8, 6, 2, 7, metal);
    p(7, 13, 4, 2, c); p(8, 13, 2, 3, dk); p(5, 3, 8, 1, lt);
  } else if (item.id === 'reactor') {
    p(4, 4, 10, 10, metalD); p(4, 4, 10, 2, metal);
    p(7, 7, 4, 4, c); p(8, 8, 2, 2, '#ffffff');
    p(2, 8, 2, 2, c); p(14, 8, 2, 2, c); p(8, 2, 2, 2, c); p(8, 14, 2, 2, c);
  } else if (item.id === 'memory') {
    p(4, 5, 10, 8, '#10202e'); p(5, 6, 8, 6, c);
    for (let i = 0; i < 4; i++) { p(2, 6 + i, 2, 1, metal); p(14, 6 + i, 2, 1, metal); }
    p(7, 8, 4, 2, '#0c1320');
  } else if (item.id === 'crown') {
    p(4, 9, 10, 4, c); p(4, 9, 10, 1, lt);
    p(4, 4, 2, 5, c); p(8, 3, 2, 6, c); p(12, 4, 2, 5, c);
    p(4, 4, 2, 2, '#fff2c0'); p(8, 3, 2, 2, '#fff2c0'); p(12, 4, 2, 2, '#fff2c0');
  } else if (item.id === 'engine') {
    p(6, 6, 6, 6, c); p(7, 7, 4, 4, '#0c1320'); p(8, 8, 2, 2, lt);
    p(8, 2, 2, 4, dk); p(8, 12, 2, 4, dk); p(2, 8, 4, 2, dk); p(12, 8, 4, 2, dk);
    p(4, 4, 3, 3, dk); p(11, 11, 3, 3, dk);
  } else {
    p(5, 5, 8, 8, c); p(5, 5, 8, 2, lt);
  }
  // level pips along the bottom
  const pips = Math.min(6, lv);
  for (let i = 0; i < pips; i++) p(2 + i * 2.5, 15, 2, 1, '#f4c656');
}
