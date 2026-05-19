// Pixel Block Drop - piece logic, gravity, line clears, scoring, input, save.

const BEST_KEY = 'pixel-block-drop-best';
const PANEL_X = 166;
const SOFT_INTERVAL = 0.028;

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

// on-canvas control buttons
const BUTTONS = [
  { id: 'left',   glyph: '◀', x: 10,  y: 318, w: 88, h: 50 },
  { id: 'rotate', glyph: '↻', x: 106, y: 318, w: 88, h: 50 },
  { id: 'right',  glyph: '▶', x: 202, y: 318, w: 88, h: 50 },
  { id: 'soft',   glyph: '▼', x: 10,  y: 374, w: 88, h: 50 },
  { id: 'hard',   glyph: '⤓', x: 106, y: 374, w: 88, h: 50 },
  { id: 'hold',   glyph: '⇄', x: 202, y: 374, w: 88, h: 50 },
];

let g = null;
let best = +(localStorage.getItem(BEST_KEY) || 0);
let lastT = performance.now();
const pressed = new Set();
const pointerRole = new Map();

// ---- setup -------------------------------------------------------------
function newGame() {
  const board = [];
  for (let y = 0; y < ROWS; y++) board.push(new Array(COLS).fill(0));
  g = {
    board, bag: [], nextQ: [],
    cur: null, holdId: null, holdUsed: false,
    score: 0, lines: 0, level: 0,
    dropTimer: 0, lockTimer: 0, lockResets: 0,
    softDrop: false, heldX: 0, dasTimer: 0, dasPhase: 'delay',
    clearAnim: null, over: false,
  };
  spawnPiece(takeNext());
}

function refillBag() {
  const b = PIECE_IDS.slice();
  for (let i = b.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [b[i], b[j]] = [b[j], b[i]];
  }
  g.bag = b;
}
function takeNext() {
  while (g.nextQ.length < 5) {
    if (g.bag.length === 0) refillBag();
    g.nextQ.push(g.bag.pop());
  }
  return g.nextQ.shift();
}
function spawnPiece(id) {
  g.cur = { id, rot: 0, x: SPAWN_X, y: SPAWN_Y };
  g.dropTimer = 0; g.lockTimer = 0; g.lockResets = 0;
  if (collides(id, 0, SPAWN_X, SPAWN_Y)) gameOver();
}
function spawnNext() {
  g.holdUsed = false;
  spawnPiece(takeNext());
}

// ---- collision / moves -------------------------------------------------
function collides(id, rot, x, y) {
  for (const [ox, oy] of PIECES[id].states[rot]) {
    const cx = x + ox, cy = y + oy;
    if (cx < 0 || cx >= COLS || cy >= ROWS) return true;
    if (cy >= 0 && g.board[cy][cx]) return true;
  }
  return false;
}
function resetLock() {
  if (g.lockResets < MAX_LOCK_RESETS) { g.lockTimer = 0; g.lockResets++; }
}
function tryMove(dx, dy) {
  if (g.over || g.clearAnim || !g.cur) return false;
  const c = g.cur;
  if (collides(c.id, c.rot, c.x + dx, c.y + dy)) return false;
  c.x += dx; c.y += dy;
  if (dy > 0) { g.lockTimer = 0; g.lockResets = 0; }
  else resetLock();
  return true;
}
function tryRotate(dir) {
  if (g.over || g.clearAnim || !g.cur) return false;
  const c = g.cur;
  const nr = (c.rot + dir + 4) % 4;
  for (const k of KICKS) {
    if (!collides(c.id, nr, c.x + k, c.y)) {
      c.rot = nr; c.x += k; resetLock(); return true;
    }
  }
  return false;
}
function ghostY() {
  let gy = g.cur.y;
  while (!collides(g.cur.id, g.cur.rot, g.cur.x, gy + 1)) gy++;
  return gy;
}
function hardDrop() {
  if (g.over || g.clearAnim || !g.cur) return;
  let d = 0;
  while (!collides(g.cur.id, g.cur.rot, g.cur.x, g.cur.y + 1)) { g.cur.y++; d++; }
  g.score += d * 2;
  lockPiece();
}
function holdPiece() {
  if (g.over || g.clearAnim || g.holdUsed || !g.cur) return;
  const curId = g.cur.id;
  if (g.holdId == null) { g.holdId = curId; spawnPiece(takeNext()); }
  else { const h = g.holdId; g.holdId = curId; spawnPiece(h); }
  g.holdUsed = true;
}

// ---- lock / line clear -------------------------------------------------
function lockPiece() {
  const c = g.cur;
  let placed = 0;
  for (const [ox, oy] of PIECES[c.id].states[c.rot]) {
    const cx = c.x + ox, cy = c.y + oy;
    if (cy >= 0) { g.board[cy][cx] = PIECES[c.id].color; placed++; }
  }
  if (placed === 0) { gameOver(); return; }
  const full = [];
  for (let y = 0; y < ROWS; y++) if (g.board[y].every(v => v)) full.push(y);
  if (full.length) g.clearAnim = { rows: full, t: 0 };
  else spawnNext();
}
function applyClears(rows) {
  rows.sort((a, b) => b - a);
  for (const ry of rows) g.board.splice(ry, 1);
  for (let i = 0; i < rows.length; i++) g.board.unshift(new Array(COLS).fill(0));
  const n = rows.length;
  g.lines += n;
  g.level = Math.floor(g.lines / 10);
  g.score += LINE_SCORES[n] * (g.level + 1);
}

function gameOver() {
  g.over = true;
  const isBest = g.score > best;
  if (isBest) { best = g.score; try { localStorage.setItem(BEST_KEY, best); } catch (e) {} }
  document.getElementById('final-score').textContent = t('finalScore', g.score);
  document.getElementById('final-best').textContent = isBest ? t('newBest') : `${t('best')}: ${best}`;
  showScreen('screen-over');
}

// ---- update ------------------------------------------------------------
function update(dt) {
  if (!g || g.over) return;
  if (g.clearAnim) {
    g.clearAnim.t += dt;
    if (g.clearAnim.t >= 0.26) { applyClears(g.clearAnim.rows); g.clearAnim = null; spawnNext(); }
    return;
  }
  if (g.heldX !== 0) {
    g.dasTimer += dt;
    if (g.dasPhase === 'delay' && g.dasTimer >= 0.17) {
      tryMove(g.heldX, 0); g.dasTimer = 0; g.dasPhase = 'repeat';
    } else if (g.dasPhase === 'repeat' && g.dasTimer >= 0.045) {
      tryMove(g.heldX, 0); g.dasTimer = 0;
    }
  }
  const c = g.cur;
  const interval = g.softDrop
    ? Math.min(SOFT_INTERVAL, gravitySec(g.level))
    : gravitySec(g.level);
  g.dropTimer += dt;
  if (g.dropTimer >= interval) {
    g.dropTimer = 0;
    if (!collides(c.id, c.rot, c.x, c.y + 1)) {
      c.y++; g.lockResets = 0; g.lockTimer = 0;
      if (g.softDrop) g.score += 1;
    }
  }
  if (!collides(c.id, c.rot, c.x, c.y + 1)) {
    g.lockTimer = 0;
  } else {
    g.lockTimer += dt;
    if (g.lockTimer >= LOCK_DELAY) lockPiece();
  }
}

// ---- render ------------------------------------------------------------
function render() {
  drawBackground(ctx);
  // score bar
  ctx.fillStyle = '#f2cf3f';
  ctx.font = '900 14px ui-monospace, Menlo, monospace';
  ctx.textAlign = 'left';
  ctx.fillText(`${t('score')} ${g.score}`, 12, 22);

  drawBoard(ctx, g.board, g.clearAnim && g.clearAnim.rows, g.clearAnim ? g.clearAnim.t : 0);
  if (g.cur && !g.over && !g.clearAnim) {
    drawPiece(ctx, g.cur.id, g.cur.rot, g.cur.x, ghostY(), true);
    drawPiece(ctx, g.cur.id, g.cur.rot, g.cur.x, g.cur.y, false);
  }

  // panel
  ctx.fillStyle = '#8fa2c8';
  ctx.font = '900 11px ui-monospace, Menlo, monospace';
  ctx.fillText(t('hold'), PANEL_X, 38);
  ctx.fillStyle = '#070a12';
  ctx.fillRect(PANEL_X, 44, 116, 56);
  if (g.holdId) drawMiniPiece(ctx, g.holdId, PANEL_X + 32, 44, 52, 11);
  ctx.fillStyle = '#8fa2c8';
  ctx.fillText(t('next'), PANEL_X, 118);
  for (let i = 0; i < 3 && i < g.nextQ.length; i++) {
    const by = 124 + i * 48;
    ctx.fillStyle = '#070a12';
    ctx.fillRect(PANEL_X, by, 116, 44);
    drawMiniPiece(ctx, g.nextQ[i], PANEL_X + 32, by - 4, 52, 10);
  }
  ctx.fillStyle = '#5fc06e';
  ctx.font = '900 12px ui-monospace, Menlo, monospace';
  ctx.fillText(`${t('level')} ${g.level}`, PANEL_X, 288);
  ctx.fillStyle = '#4ad6e0';
  ctx.fillText(`${t('lines')} ${g.lines}`, PANEL_X, 308);

  drawButtons(ctx, BUTTONS, pressed);
}

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}

// ---- input -------------------------------------------------------------
function setHeldX(d) {
  if (!g || g.over) return;
  g.heldX = d; g.dasTimer = 0; g.dasPhase = 'delay';
  if (d !== 0) tryMove(d, 0);
}
function doButton(id) {
  if (id === 'left') setHeldX(-1);
  else if (id === 'right') setHeldX(1);
  else if (id === 'rotate') tryRotate(1);
  else if (id === 'hard') hardDrop();
  else if (id === 'hold') holdPiece();
  else if (id === 'soft') g.softDrop = true;
}
function endButton(id) {
  if (id === 'left' && g.heldX === -1) g.heldX = 0;
  else if (id === 'right' && g.heldX === 1) g.heldX = 0;
  else if (id === 'soft') g.softDrop = false;
}
function hitButton(px, py) {
  for (const b of BUTTONS)
    if (px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h) return b.id;
  return null;
}
canvas.addEventListener('pointerdown', e => {
  if (!g || g.over || document.getElementById('screen-game').classList.contains('hidden')) return;
  e.preventDefault();
  const r = canvas.getBoundingClientRect();
  const px = (e.clientX - r.left) / r.width * VW;
  const py = (e.clientY - r.top) / r.height * VH;
  const id = hitButton(px, py);
  if (id) { pointerRole.set(e.pointerId, id); pressed.add(id); doButton(id); }
});
function releasePointer(e) {
  const id = pointerRole.get(e.pointerId);
  if (id == null) return;
  pointerRole.delete(e.pointerId);
  pressed.delete(id);
  if (g && !g.over) endButton(id);
}
canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);
canvas.addEventListener('contextmenu', e => e.preventDefault());

window.addEventListener('keydown', e => {
  if (!g || g.over) return;
  const k = e.code;
  if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Space'].includes(k)) e.preventDefault();
  if (k === 'ArrowLeft') setHeldX(-1);
  else if (k === 'ArrowRight') setHeldX(1);
  else if (k === 'ArrowUp' || k === 'KeyX') { if (!e.repeat) tryRotate(1); }
  else if (k === 'KeyZ') { if (!e.repeat) tryRotate(-1); }
  else if (k === 'ArrowDown') g.softDrop = true;
  else if (k === 'Space') { if (!e.repeat) hardDrop(); }
  else if (k === 'KeyC' || k === 'ShiftLeft' || k === 'ShiftRight') { if (!e.repeat) holdPiece(); }
});
window.addEventListener('keyup', e => {
  if (!g) return;
  const k = e.code;
  if (k === 'ArrowLeft' && g.heldX === -1) g.heldX = 0;
  else if (k === 'ArrowRight' && g.heldX === 1) g.heldX = 0;
  else if (k === 'ArrowDown') g.softDrop = false;
});

document.getElementById('btn-play').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => {
  document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
  showScreen('screen-title');
};
setupLanguageToggle();

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
