// Neon Rift Runner - pixel art: animated hero, distinct enemies, skill icons.

function nrShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}

// Animated hero. `player` carries vx / vy / grounded / attackPulse for pose;
// the sprite keeps the same bounding box the old static art used.
function drawPixelHero(ctx, x, y, flip, player) {
  const t = performance.now() / 1000;
  const p = player || {};
  const attacking = (p.attackPulse || 0) > 0;
  const airborne = p.grounded === false;
  const running = !airborne && Math.abs(p.vx || 0) > 0.5;
  ctx.save();
  ctx.translate(Math.round(x), Math.round(y));
  ctx.scale(flip ? -1 : 1, 1);
  const R = (gx, gy, gw, gh, c) => { ctx.fillStyle = c; ctx.fillRect(gx, gy, gw, gh); };

  const ink = '#101722', suit = '#34506b', suitL = '#4d6e8f', suitD = '#22384c';
  const glow = '#61e5ff', trim = '#52dc88', skin = '#f0c79a';

  // --- legs --------------------------------------------------------------
  let lA = 0, lB = 0;
  if (running) {
    const cyc = [[-4, 4], [-1, 1], [4, -4], [-1, 1]][Math.floor(t * 13) % 4];
    lA = cyc[0]; lB = cyc[1];
  } else if (airborne) {
    lA = (p.vy || 0) < 0 ? -3 : 3; lB = (p.vy || 0) < 0 ? 3 : -3;
  } else {
    lB = Math.sin(t * 3) > 0 ? 0 : 1;
  }
  R(-8 + lA, 6, 6, 11 - Math.abs(lA) * 0.4, ink);
  R(2 + lB, 6, 6, 11 - Math.abs(lB) * 0.4, ink);
  R(-8 + lA, 14, 6, 3, '#0a0f17');
  R(2 + lB, 14, 6, 3, '#0a0f17');

  // --- torso -------------------------------------------------------------
  const lean = running ? 2 : 0;
  R(-9 + lean, -16, 18, 14, suit);
  R(-9 + lean, -16, 18, 4, suitL);
  R(-9 + lean, -4, 18, 2, suitD);
  R(-6 + lean, -12, 12, 3, trim);
  R(-9 + lean, -16, 3, 14, suitL);

  // --- head --------------------------------------------------------------
  R(-7 + lean, -28, 14, 12, ink);
  R(-5 + lean, -27, 10, 7, glow);
  R(-5 + lean, -27, 10, 2, '#bff4ff');
  R(-4 + lean, -25, 3, 3, '#0a0f17');
  R(-7 + lean, -16, 14, 2, suitD);
  R(-1 + lean, -31, 2, 3, trim);

  // --- arm + plasma blade ------------------------------------------------
  if (attacking) {
    const swing = Math.min(1, p.attackPulse / 10);
    const reach = 8 + swing * 16;
    R(8 + lean, -14, 8, 5, skin);
    ctx.globalAlpha = 0.4;
    R(14 + lean, -22, reach + 6, 18, '#ff8a6a');
    ctx.globalAlpha = 1;
    R(14 + lean, -16, reach, 6, '#ffd36a');
    R(14 + lean + reach, -18, 5, 10, '#fff2c0');
  } else {
    R(7 + lean, -13, 6, 8, skin);
    R(10 + lean, -13, 4, 12, '#61e5ff');
  }
  ctx.restore();
}

// Distinct enemy sprites - hovering drone vs legged crawler.
function drawEnemy(ctx, enemy, camX) {
  const t = performance.now() / 1000;
  const x = Math.round(enemy.x - camX);
  const y = Math.round(enemy.y);
  const R = (gx, gy, gw, gh, c) => { ctx.fillStyle = c; ctx.fillRect(gx, gy, gw, gh); };

  if (enemy.kind === 'drone') {
    const bob = Math.round(Math.sin(t * 4 + enemy.x * 0.05) * 3);
    const cy = y - enemy.h + bob;
    ctx.globalAlpha = 0.5;
    R(x - 14, cy - 4, 28, 3, '#8fa7c4');
    ctx.globalAlpha = 1;
    R(x - 2, cy - 6, 4, 4, '#3a4658');
    R(x - 11, cy, 22, 14, nrShade('#ad7dff', -10));
    R(x - 11, cy, 22, 3, '#c9a6ff');
    R(x - 11, cy + 11, 22, 3, '#5b3f8c');
    const blink = Math.sin(t * 3) > -0.7;
    R(x - 5, cy + 4, 10, 5, '#10131c');
    if (blink) { R(x - 3, cy + 5, 6, 3, '#ff6b6b'); R(x - 3, cy + 5, 2, 2, '#ffd0d0'); }
    R(x - 14, cy + 4, 3, 5, '#6f7a8e');
    R(x + 11, cy + 4, 3, 5, '#6f7a8e');
    ctx.globalAlpha = 0.6;
    R(x - 14, cy + 9, 3, 6 + bob, '#7ad0ff');
    R(x + 11, cy + 9, 3, 6 + bob, '#7ad0ff');
    ctx.globalAlpha = 1;
  } else {
    const step = Math.floor(t * 11 + enemy.x * 0.1) % 2;
    const top = y - enemy.h;
    const legY = y - 4;
    for (let i = 0; i < 4; i++) {
      const lx = x - 12 + i * 8;
      const up = (i % 2 === step) ? 2 : 0;
      R(lx, legY - up, 3, 6 - up, '#1c1014');
    }
    R(x - 13, top + 4, 26, enemy.h - 8, nrShade('#ec5b56', -8));
    R(x - 13, top + 4, 26, 3, '#ff8f78');
    R(x - 11, top + 8, 22, 4, '#7a2b2b');
    R(x + 9, top + 6, 7, enemy.h - 12, '#2a1518');
    R(x + 15, top + 7, 4, 3, '#ffd36a');
    R(x + 15, top + enemy.h - 13, 4, 3, '#ffd36a');
    R(x - 7, top + 7, 4, 4, '#ffd36a');
    R(x + 1, top + 7, 4, 4, '#ffd36a');
    R(x - 6, top + 8, 2, 2, '#1c1014');
    R(x + 2, top + 8, 2, 2, '#1c1014');
  }
  if (enemy.hp < enemy.maxHp) {
    const f = Math.max(0, enemy.hp / enemy.maxHp);
    R(x - 12, y - enemy.h - 8, 24, 4, '#10131c');
    ctx.fillStyle = f > 0.4 ? '#52dc88' : '#ec5b56';
    ctx.fillRect(x - 11, y - enemy.h - 7, 22 * f, 2);
  }
}

// Per-skill pixel glyphs (8x8), looked up by skill id.
const SKILL_GLYPHS = {
  blade: ['......11', '.....111', '....111.', '...111..', '..111...', '.111....', '111.....', '11......'],
  heart: ['.11..11.', '11111111', '11111111', '11111111', '.111111.', '..1111..', '...11...', '........'],
  jump: ['...11...', '..1111..', '.11..11.', '11....11', '...11...', '..1111..', '.11..11.', '11....11'],
  dash: ['....111.', '...111..', '..111...', '.1111111', '...111..', '..111...', '.111....', '.11.....'],
  crit: ['...11...', '..1..1..', '.1....1.', '11.11.11', '11.11.11', '.1....1.', '..1..1..', '...11...'],
  guard: ['11111111', '11111111', '.111111.', '.111111.', '.111111.', '..1111..', '...11...', '........'],
  magnet: ['.11..11.', '.11..11.', '.11..11.', '.11..11.', '.111111.', '..1111..', '11....11', '11....11'],
  tempo: ['.1....1.', '.11...11', '.111..11', '.1111111', '.1111111', '.111..11', '.11...11', '.1....1.'],
  coin: ['..1111..', '.11..11.', '11.11.11', '11.111.1', '1.111.11', '11.11.11', '.11..11.', '..1111..'],
};

function drawSkillIcon(ctx, color, id) {
  ctx.clearRect(0, 0, 54, 54);
  ctx.fillStyle = '#080b11';
  ctx.fillRect(0, 0, 54, 54);
  ctx.fillStyle = '#11161f';
  ctx.fillRect(3, 3, 48, 48);
  const data = SKILL_GLYPHS[id] || SKILL_GLYPHS.crit;
  const s = 5, ox = 7, oy = 7;
  ctx.fillStyle = nrShade(color, -70);
  for (let y = 0; y < data.length; y++)
    for (let x = 0; x < data[y].length; x++)
      if (data[y][x] === '1') ctx.fillRect(ox + x * s + 1, oy + y * s + 1, s, s);
  ctx.fillStyle = color;
  for (let y = 0; y < data.length; y++)
    for (let x = 0; x < data[y].length; x++)
      if (data[y][x] === '1') ctx.fillRect(ox + x * s, oy + y * s, s, s);
  ctx.fillStyle = 'rgba(255,255,255,0.3)';
  ctx.fillRect(3, 3, 48, 2);
}
