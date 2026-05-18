// Pixel Tower Defense - pixel art rendering

// Shared enemy body matrix (chars map to a per-type palette).
// o=outline b=body h=highlight s=shadow e=eye-white p=pupil m=mouth
const BLOB = [
  '....oooooo....',
  '..oobbbbbboo..',
  '.obbbbbbbbbbo.',
  'obbhhhhhhhhbbo',
  'obbhbbbbbbhbbo',
  'obbbbbbbbbbbbo',
  'obeeobbbboeebo',
  'obeppobboeppbo',
  'obeppobboeppbo',
  'obbbbbbbbbbbbo',
  'obbbbmmmmbbbbo',
  'obbbbbbbbbbbbo',
  '.obbssssssbbo.',
  '..oobsssssoo..',
  '...oosssoo....',
];

const ENEMY_PALETTE = {
  grunt:   { o: '#241a33', b: '#6fae3e', h: '#a6d76a', s: '#4c7d28', e: '#f4f9ff', p: '#1c1426', m: '#241a33' },
  runner:  { o: '#33210f', b: '#e8893a', h: '#ffc77a', s: '#a85f24', e: '#fff4e6', p: '#2a1606', m: '#33210f' },
  swarm:   { o: '#2b1340', b: '#b77cff', h: '#dcbcff', s: '#7d4fbe', e: '#fff', p: '#23103a', m: '#2b1340' },
  armored: { o: '#1c2230', b: '#8a93a8', h: '#cbd2e0', s: '#5a6276', e: '#dfe8ff', p: '#11151f', m: '#1c2230' },
  flyer:   { o: '#0e2e36', b: '#5fc7d8', h: '#a6ecf3', s: '#357f8c', e: '#eafcff', p: '#08222a', m: '#0e2e36' },
  healer:  { o: '#3a1226', b: '#e85d8a', h: '#ffaecb', s: '#a83a60', e: '#fff', p: '#2a0a18', m: '#3a1226' },
  boss:    { o: '#2a0a0a', b: '#c0392b', h: '#ff7a5f', s: '#7d1f16', e: '#ffe7c2', p: '#1c0606', m: '#2a0a0a' },
};

function drawMatrix(ctx, matrix, palette, ox, oy, unit) {
  for (let r = 0; r < matrix.length; r++) {
    const row = matrix[r];
    for (let c = 0; c < row.length; c++) {
      const ch = row[c];
      if (ch === '.' || ch === ' ') continue;
      const col = palette[ch];
      if (!col) continue;
      ctx.fillStyle = col;
      ctx.fillRect(ox + c * unit, oy + r * unit, unit, unit);
    }
  }
}

// Draw an enemy centered at (x,y) sized to `size` px, with a walk wobble.
function drawEnemy(ctx, type, x, y, size, t, hpRatio) {
  const pal = ENEMY_PALETTE[type] || ENEMY_PALETTE.grunt;
  const cols = BLOB[0].length;
  const rows = BLOB.length;
  const wob = Math.sin(t * 9) * 0.06;
  const w = size * (1 + wob);
  const h = size * (1 - wob);
  const unit = w / cols;
  const ox = x - w / 2;
  const oy = y - h / 2 - size * 0.1;

  // soft shadow
  ctx.fillStyle = 'rgba(0,0,0,0.28)';
  ctx.beginPath();
  ctx.ellipse(x, y + size * 0.42, size * 0.42, size * 0.16, 0, 0, Math.PI * 2);
  ctx.fill();

  // type overlays drawn behind body
  if (type === 'flyer') drawWings(ctx, x, y, size, t);

  drawMatrix(ctx, BLOB, pal, ox, oy, unit);

  // type overlays drawn on body
  if (type === 'armored') {
    ctx.fillStyle = pal.h;
    ctx.fillRect(ox + unit * 2, oy, unit * 10, unit * 2.4);
    ctx.fillStyle = pal.o;
    ctx.fillRect(ox + unit * 6, oy - unit, unit * 2, unit * 2);
  }
  if (type === 'healer') {
    const cx = x, cy = oy + unit * 1.5;
    ctx.fillStyle = '#fff';
    ctx.fillRect(cx - unit * 0.7, cy - unit * 2, unit * 1.4, unit * 4);
    ctx.fillRect(cx - unit * 2, cy - unit * 0.7, unit * 4, unit * 1.4);
  }
  if (type === 'boss') {
    ctx.fillStyle = '#ffd34d';
    for (let i = 0; i < 4; i++) {
      ctx.fillRect(ox + unit * (2 + i * 3), oy - unit * 2.4, unit * 1.6, unit * 2.6);
    }
    ctx.fillRect(ox + unit * 1.5, oy - unit * 0.6, unit * 11, unit * 1.4);
  }

  // hp bar
  if (hpRatio < 1) {
    const bw = size * 0.92;
    const by = y - size * 0.62;
    ctx.fillStyle = 'rgba(0,0,0,0.6)';
    ctx.fillRect(x - bw / 2 - 1, by - 1, bw + 2, 6);
    ctx.fillStyle = '#3a1320';
    ctx.fillRect(x - bw / 2, by, bw, 4);
    ctx.fillStyle = hpRatio > 0.5 ? '#5fe07a' : hpRatio > 0.25 ? '#f4c85a' : '#ff5a5a';
    ctx.fillRect(x - bw / 2, by, bw * hpRatio, 4);
  }
}

function drawWings(ctx, x, y, size, t) {
  const flap = Math.sin(t * 16) * size * 0.22;
  ctx.fillStyle = 'rgba(166,236,243,0.85)';
  ctx.beginPath();
  ctx.moveTo(x - size * 0.2, y);
  ctx.lineTo(x - size * 0.7, y - size * 0.3 - flap);
  ctx.lineTo(x - size * 0.55, y + size * 0.25);
  ctx.closePath();
  ctx.fill();
  ctx.beginPath();
  ctx.moveTo(x + size * 0.2, y);
  ctx.lineTo(x + size * 0.7, y - size * 0.3 - flap);
  ctx.lineTo(x + size * 0.55, y + size * 0.25);
  ctx.closePath();
  ctx.fill();
}

// --- Tile / terrain -----------------------------------------------------
const THEME = {
  grass: { g1: '#3f7d3a', g2: '#467f3f', g3: '#356b32', road: '#b89058', roadEdge: '#8a6a3c', deco: '#2c5a2c', decoHi: '#5a9a4a' },
  snow:  { g1: '#cfe0ea', g2: '#c2d6e2', g3: '#b4cad8', road: '#9aa8bc', roadEdge: '#76849a', deco: '#7d92a8', decoHi: '#e8f2fa' },
  lava:  { g1: '#4a3330', g2: '#523833', g3: '#3f2a28', road: '#d97a3a', roadEdge: '#9a4a1f', deco: '#2a1a18', decoHi: '#ff8a3a' },
};

function drawTileBg(ctx, theme, gx, gy) {
  const th = THEME[theme] || THEME.grass;
  const x = gx * TILE, y = gy * TILE;
  const checker = (gx + gy) % 2;
  ctx.fillStyle = checker ? th.g1 : th.g2;
  ctx.fillRect(x, y, TILE, TILE);
  // subtle pixel texture
  ctx.fillStyle = th.g3;
  const seed = (gx * 31 + gy * 17) % 7;
  if (seed < 3) ctx.fillRect(x + 6 + seed * 8, y + 10 + seed * 6, 5, 5);
  if (seed === 4) ctx.fillRect(x + 24, y + 8, 4, 4);
}

function drawDeco(ctx, theme, gx, gy) {
  const th = THEME[theme] || THEME.grass;
  const x = gx * TILE + TILE / 2, y = gy * TILE + TILE / 2;
  ctx.fillStyle = 'rgba(0,0,0,0.25)';
  ctx.beginPath();
  ctx.ellipse(x, y + 12, 14, 5, 0, 0, Math.PI * 2);
  ctx.fill();
  if (theme === 'lava') {
    ctx.fillStyle = th.deco;
    ctx.fillRect(x - 12, y - 4, 24, 16);
    ctx.fillStyle = th.decoHi;
    ctx.fillRect(x - 8, y - 2, 6, 6);
    ctx.fillRect(x + 2, y + 4, 6, 4);
  } else {
    // rock / tree
    ctx.fillStyle = th.deco;
    ctx.fillRect(x - 4, y - 2, 8, 16);
    ctx.beginPath();
    ctx.arc(x, y - 8, 13, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = th.decoHi;
    ctx.beginPath();
    ctx.arc(x - 4, y - 11, 5, 0, Math.PI * 2);
    ctx.fill();
  }
}

// --- Towers -------------------------------------------------------------
// Draw a placed tower. `tower` has type, tier, branch, angle, stats.
function drawTower(ctx, tower) {
  const def = TOWERS[tower.type];
  const x = tower.x, y = tower.y;
  const tier = tower.tier;

  // base platform
  ctx.fillStyle = 'rgba(0,0,0,0.3)';
  ctx.beginPath();
  ctx.ellipse(x, y + 11, 18, 7, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#3a3550';
  ctx.fillRect(x - 15, y - 2, 30, 14);
  ctx.fillStyle = '#544f72';
  ctx.fillRect(x - 15, y - 2, 30, 4);
  // tier studs
  ctx.fillStyle = '#ffd34d';
  for (let i = 0; i < tier; i++) ctx.fillRect(x - 12 + i * 9, y + 6, 5, 4);

  // tower body
  ctx.save();
  ctx.translate(x, y - 4);
  ctx.fillStyle = def.accent;
  ctx.fillRect(-11, -16, 22, 20);
  ctx.fillStyle = def.color;
  ctx.fillRect(-11, -16, 22, 5);
  ctx.fillRect(-11, -16, 6, 20);
  // turret
  ctx.rotate(tower.angle || 0);
  if (tower.type === 'cannon') {
    ctx.fillStyle = '#2a2535';
    ctx.fillRect(0, -6, 22 + tier * 3, 12);
    ctx.fillStyle = def.color;
    ctx.fillRect(0, -6, 8, 12);
  } else if (tower.type === 'frost') {
    ctx.fillStyle = def.color;
    ctx.fillRect(2, -4, 16 + tier * 2, 8);
    ctx.fillStyle = '#eafcff';
    ctx.fillRect(14 + tier * 2, -6, 6, 12);
  } else if (tower.type === 'arcane') {
    ctx.fillStyle = '#2a2535';
    ctx.fillRect(0, -5, 14, 10);
    ctx.fillStyle = def.color;
    ctx.beginPath();
    ctx.arc(16 + tier, 0, 6 + tier, 0, Math.PI * 2);
    ctx.fill();
  } else { // arrow
    ctx.fillStyle = '#2a2535';
    ctx.fillRect(0, -4, 20 + tier * 3, 8);
    ctx.fillStyle = def.color;
    ctx.fillRect(16 + tier * 3, -5, 5, 10);
  }
  ctx.restore();

  // top gem
  ctx.fillStyle = def.color;
  ctx.fillRect(x - 4, y - 22, 8, 8);
  ctx.fillStyle = '#fff';
  ctx.fillRect(x - 3, y - 21, 3, 3);
}

// small tower icon for the build bar / panel
function drawTowerIcon(ctx, type, cx, cy, scale) {
  const def = TOWERS[type];
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scale, scale);
  ctx.fillStyle = '#3a3550';
  ctx.fillRect(-13, 2, 26, 10);
  ctx.fillStyle = def.accent;
  ctx.fillRect(-9, -12, 18, 16);
  ctx.fillStyle = def.color;
  ctx.fillRect(-9, -12, 18, 4);
  ctx.fillStyle = def.color;
  ctx.fillRect(-4, -20, 8, 8);
  ctx.fillStyle = '#2a2535';
  ctx.fillRect(0, -16, 16, 6);
  ctx.fillStyle = def.color;
  ctx.fillRect(12, -17, 5, 8);
  ctx.restore();
}

function drawStar(ctx, x, y, r, filled) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const ang = (Math.PI / 5) * i - Math.PI / 2;
    const rad = i % 2 === 0 ? r : r * 0.45;
    ctx[i === 0 ? 'moveTo' : 'lineTo'](x + Math.cos(ang) * rad, y + Math.sin(ang) * rad);
  }
  ctx.closePath();
  ctx.fillStyle = filled ? '#ffd34d' : '#2e2a40';
  ctx.fill();
  ctx.lineWidth = 2;
  ctx.strokeStyle = filled ? '#a8780f' : '#1a1726';
  ctx.stroke();
}
