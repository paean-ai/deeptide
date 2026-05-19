// Pixel Road Hop - rows, traffic, river logs, hopping, scoring, save.

const BEST_KEY = 'pixel-road-hop-best';
const START_ROW = 2;

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let g = null;
let best = +(localStorage.getItem(BEST_KEY) || 0);
let lastT = performance.now();

const lerp = (a, b, f) => a + (b - a) * f;

// ---- row generation ----------------------------------------------------
function genRow(idx) {
  if (idx < SAFE_ROWS) return { type: 'grass', trees: [], shade: idx % 2 === 0 };
  const d = Math.min(1, idx / 140);
  let roll = Math.random();
  let type = roll < 0.30 ? 'grass' : (roll < 0.66 + d * 0.06 ? 'road' : 'river');
  const prev = g.rows[idx - 1];
  if (type === 'river' && prev && prev.type === 'river' && Math.random() < 0.55) type = 'road';

  if (type === 'grass') {
    const trees = [];
    for (let c = 0; c < COLS; c++) if (Math.random() < 0.20) trees.push(c);
    while (trees.length > COLS - 3) trees.pop();
    return { type: 'grass', trees, shade: idx % 2 === 0 };
  }
  if (type === 'road') {
    const dir = Math.random() < 0.5 ? 1 : -1;
    const speed = 1.6 + Math.random() * 2.0 + d * 1.8;
    const count = 2 + Math.floor(Math.random() * 2);
    const color = CAR_COLORS[Math.floor(Math.random() * CAR_COLORS.length)];
    const gap = (COLS + 4) / count;
    const off = Math.random() * gap;
    const mobs = [];
    for (let i = 0; i < count; i++) mobs.push({ x: -2 + off + i * gap, w: 1, speed, dir });
    return { type: 'road', dir, color, mobs };
  }
  const dir = Math.random() < 0.5 ? 1 : -1;
  const speed = 0.8 + Math.random() * 1.2 + d * 0.8;
  const gap = (COLS + 6) / 3;
  const off = Math.random() * gap;
  const mobs = [];
  for (let i = 0; i < 3; i++) {
    mobs.push({ x: -3 + off + i * gap, w: 2 + Math.floor(Math.random() * 3), speed, dir });
  }
  return { type: 'river', dir, mobs };
}

function ensureRows() {
  const lo = Math.floor(g.camRow) - 3, hi = Math.floor(g.camRow) + 16;
  for (let r = lo; r <= hi; r++) if (!g.rows[r]) g.rows[r] = genRow(r);
}

// ---- lifecycle ---------------------------------------------------------
function newGame() {
  g = {
    rows: {}, camRow: START_ROW - 3, maxRow: START_ROW,
    player: { row: START_ROW, pcol: (COLS - 1) / 2, hop: null, alive: true },
    over: false, deathCause: 'gameOver', deathT: 0,
  };
  ensureRows();
  updateHud();
}

function die(cause) {
  if (!g.player.alive) return;
  g.player.alive = false;
  g.deathCause = cause;
  g.deathT = 0;
}
function finishGame() {
  g.over = true;
  const score = g.maxRow - START_ROW;
  const isBest = score > best;
  if (isBest) { best = score; try { localStorage.setItem(BEST_KEY, best); } catch (e) {} }
  document.getElementById('over-title').textContent = t(g.deathCause);
  document.getElementById('final-score').textContent = t('finalScore', score);
  document.getElementById('final-best').textContent = isBest ? t('newBest') : `${t('best')}: ${best}`;
  showScreen('screen-over');
}

// ---- hopping -----------------------------------------------------------
function hop(dir) {
  const p = g.player;
  if (g.over || !p.alive || p.hop) return;
  const base = Math.round(p.pcol);
  let toRow = p.row, toPcol = base;
  if (dir === 'up') toRow = p.row + 1;
  else if (dir === 'down') toRow = p.row - 1;
  else if (dir === 'left') toPcol = base - 1;
  else if (dir === 'right') toPcol = base + 1;
  if (toPcol < 0 || toPcol >= COLS || toRow < 0) return;
  const tr = g.rows[toRow];
  if (tr && tr.type === 'grass' && tr.trees.includes(toPcol)) return;
  p.hop = { fromRow: p.row, fromPcol: p.pcol, toRow, toPcol, t: 0 };
  if (toRow > g.maxRow) g.maxRow = toRow;
}

// ---- update ------------------------------------------------------------
function update(dt) {
  if (!g || g.over) return;
  if (!g.player.alive) {
    g.deathT += dt;
    if (g.deathT > 0.8) finishGame();
    return;
  }
  // camera: follow forward fast, creep slowly when idle
  const desired = g.player.row - 3;
  if (desired > g.camRow) g.camRow += Math.min(desired - g.camRow, 14 * dt);
  g.camRow += CREEP * dt;
  ensureRows();

  // moving traffic / logs
  for (const k in g.rows) {
    const row = g.rows[k];
    if (!row.mobs) continue;
    for (const m of row.mobs) {
      m.x += m.speed * m.dir * dt;
      if (m.dir > 0 && m.x > COLS + 3) m.x = -m.w - 3;
      else if (m.dir < 0 && m.x < -m.w - 3) m.x = COLS + 3;
    }
  }

  // hop animation
  const p = g.player;
  if (p.hop) {
    p.hop.t += dt / HOP_DUR;
    if (p.hop.t >= 1) { p.row = p.hop.toRow; p.pcol = p.hop.toPcol; p.hop = null; }
  }

  // grounded interactions
  if (!p.hop && p.alive) {
    const row = g.rows[p.row];
    if (row && row.type === 'road') {
      for (const m of row.mobs) {
        if (p.pcol + 0.85 > m.x && p.pcol + 0.15 < m.x + m.w) { die('gameOver'); break; }
      }
    } else if (row && row.type === 'river') {
      const center = p.pcol + 0.5;
      let log = null;
      for (const m of row.mobs) if (center > m.x && center < m.x + m.w) { log = m; break; }
      if (log) {
        p.pcol += log.speed * log.dir * dt;
        if (p.pcol < -0.55 || p.pcol > COLS - 0.45) die('drowned');
      } else {
        die('drowned');
      }
    }
    if (p.alive && p.row <= g.camRow + 0.02) die('caught');
  }

  updateHud();
}

// ---- render ------------------------------------------------------------
function render() {
  ctx.fillStyle = '#56b855';
  ctx.fillRect(0, 0, VW, VH);
  const lo = Math.floor(g.camRow) - 3, hi = Math.floor(g.camRow) + 15;
  for (let r = hi; r >= lo; r--) {
    const row = g.rows[r];
    if (!row) continue;
    const sy = rowScreenY(r, g.camRow);
    drawRowBand(ctx, row, sy);
    if (row.type === 'grass') {
      for (const c of row.trees) drawTree(ctx, c * TILE + TILE / 2, sy);
    } else if (row.type === 'road') {
      for (const m of row.mobs) drawCar(ctx, m.x * TILE + 4, sy, m.dir, row.color);
    } else {
      for (const m of row.mobs) drawLog(ctx, m.x * TILE, sy, m.w * TILE);
    }
  }
  // player
  const p = g.player;
  let pr = p.row, pc = p.pcol, hopZ = 0;
  if (p.hop) {
    pr = lerp(p.hop.fromRow, p.hop.toRow, p.hop.t);
    pc = lerp(p.hop.fromPcol, p.hop.toPcol, p.hop.t);
    hopZ = Math.sin(p.hop.t * Math.PI) * 14;
  }
  const cx = pc * TILE + TILE / 2;
  const cy = rowScreenY(pr, g.camRow) + TILE / 2;
  drawPlayer(ctx, cx, cy, hopZ, !p.alive);
}

function updateHud() {
  if (!g) return;
  const score = g.maxRow - START_ROW;
  document.getElementById('hud-score').textContent = `${t('score')} ${score}`;
  document.getElementById('hud-best').textContent = `${t('best')} ${Math.max(best, score)}`;
}

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}

// ---- input -------------------------------------------------------------
let downX = 0, downY = 0, downId = null;
canvas.addEventListener('pointerdown', e => {
  if (!g || g.over || document.getElementById('screen-game').classList.contains('hidden')) return;
  e.preventDefault();
  downId = e.pointerId; downX = e.clientX; downY = e.clientY;
});
canvas.addEventListener('pointerup', e => {
  if (e.pointerId !== downId || !g || g.over) return;
  downId = null;
  const dx = e.clientX - downX, dy = e.clientY - downY;
  if (Math.abs(dx) < 22 && Math.abs(dy) < 22) { hop('up'); return; }
  if (Math.abs(dx) > Math.abs(dy)) hop(dx > 0 ? 'right' : 'left');
  else hop(dy > 0 ? 'down' : 'up');
});
window.addEventListener('keydown', e => {
  if (!g || g.over) return;
  const k = e.code;
  if (k === 'ArrowUp' || k === 'KeyW') { e.preventDefault(); hop('up'); }
  else if (k === 'ArrowDown' || k === 'KeyS') { e.preventDefault(); hop('down'); }
  else if (k === 'ArrowLeft' || k === 'KeyA') { e.preventDefault(); hop('left'); }
  else if (k === 'ArrowRight' || k === 'KeyD') { e.preventDefault(); hop('right'); }
});

document.getElementById('btn-play').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => {
  document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
  showScreen('screen-title');
};
setupLanguageToggle(() => { if (g) updateHud(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (g && !document.getElementById('screen-game').classList.contains('hidden')) {
    update(dt);
    render();
  }
  requestAnimationFrame(loop);
}

newGame();
document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
showScreen('screen-title');
requestAnimationFrame(loop);
