// Pixel Bubble Pop - an aim-and-shoot bubble shooter on an offset grid.

const BEST_KEY = 'pixel-bubble-pop-best';
const GRID_ROWS = 16;
const LX = VW / 2, LY = VH - 46;       // launcher position

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();

const inGrid = (r, c) => r >= 0 && r < GRID_ROWS && c >= 0 && c < rowLen(r);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  const grid = [];
  for (let r = 0; r < GRID_ROWS; r++) grid.push(new Array(rowLen(r)).fill(null));
  game = { grid, shot: null, angle: 0, score: 0, over: false, t: 0,
    pops: [], falling: [], banner: null, nextColor: 0 };
  fillTop(5);
  game.nextColor = pickColor();
}

function fillTop(rows) {
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < rowLen(r); c++) {
      game.grid[r][c] = (Math.random() * BUB_COLORS.length) | 0;
    }
  }
}

// next colour is drawn from colours still on the board (so it is always usable)
function pickColor() {
  const present = new Set();
  for (let r = 0; r < GRID_ROWS; r++)
    for (let c = 0; c < rowLen(r); c++)
      if (game.grid[r][c] != null) present.add(game.grid[r][c]);
  const pool = present.size ? [...present] : BUB_COLORS.map((_, i) => i);
  return pool[(Math.random() * pool.length) | 0];
}

function setBanner(text) { game.banner = { text, life: 1.2 }; }

// ---- shooting ----------------------------------------------------------
function shoot() {
  if (!game || game.over || game.shot || !overlaysClosed()) return;
  game.shot = {
    x: LX, y: LY, color: game.nextColor,
    vx: Math.sin(game.angle) * SHOT_SPEED,
    vy: -Math.cos(game.angle) * SHOT_SPEED,
  };
  game.nextColor = pickColor();
}

function bubbleAt(x, y) {
  for (let r = 0; r < GRID_ROWS; r++) {
    for (let c = 0; c < rowLen(r); c++) {
      if (game.grid[r][c] == null) continue;
      const dx = x - cellX(r, c), dy = y - cellY(r);
      if (dx * dx + dy * dy < (BUB * 0.92) ** 2) return { r, c };
    }
  }
  return null;
}

function snapShot() {
  const s = game.shot;
  let target = null;
  if (s.y <= BUB / 2 + 1) {
    // hit the ceiling - snap to the nearest empty cell in row 0
    let bestD = Infinity, bc = 0;
    for (let c = 0; c < rowLen(0); c++) {
      if (game.grid[0][c] != null) continue;
      const d = Math.abs(s.x - cellX(0, c));
      if (d < bestD) { bestD = d; bc = c; }
    }
    target = { r: 0, c: bc };
  } else {
    const hit = bubbleAt(s.x, s.y);
    if (!hit) return false;
    let bestD = Infinity;
    for (const [nr, nc] of neighbors(hit.r, hit.c)) {
      if (!inGrid(nr, nc) || game.grid[nr][nc] != null) continue;
      const dx = s.x - cellX(nr, nc), dy = s.y - cellY(nr, nc);
      const d = dx * dx + dy * dy;
      if (d < bestD) { bestD = d; target = { r: nr, c: nc }; }
    }
    if (!target) return false;
  }
  game.grid[target.r][target.c] = s.color;
  game.shot = null;
  resolve(target.r, target.c);
  if (cellY(target.r) + BUB / 2 >= DANGER_Y) gameOver();
  return true;
}

// flood-fill the same-colour cluster, then drop anything detached
function resolve(r, c) {
  const color = game.grid[r][c];
  const cluster = floodSame(r, c, color);
  if (cluster.length >= 3) {
    for (const [cr, cc] of cluster) {
      game.pops.push({ x: cellX(cr, cc), y: cellY(cr, cc), color, life: 1 });
      game.grid[cr][cc] = null;
    }
    game.score += cluster.length * 10 + (cluster.length - 3) * 6;
    dropDetached();
  }
  // board cleared - refill the top
  let any = false;
  for (let rr = 0; rr < GRID_ROWS && !any; rr++)
    for (let cc = 0; cc < rowLen(rr); cc++) if (game.grid[rr][cc] != null) any = true;
  if (!any) { game.score += 500; setBanner(t('cleared')); fillTop(5); }
}

function floodSame(r, c, color) {
  const seen = new Set([r + ',' + c]), out = [[r, c]], stack = [[r, c]];
  while (stack.length) {
    const [cr, cc] = stack.pop();
    for (const [nr, nc] of neighbors(cr, cc)) {
      if (!inGrid(nr, nc) || game.grid[nr][nc] !== color) continue;
      const k = nr + ',' + nc;
      if (seen.has(k)) continue;
      seen.add(k); out.push([nr, nc]); stack.push([nr, nc]);
    }
  }
  return out;
}

function dropDetached() {
  const anchored = new Set();
  const stack = [];
  for (let c = 0; c < rowLen(0); c++) {
    if (game.grid[0][c] != null) { anchored.add('0,' + c); stack.push([0, c]); }
  }
  while (stack.length) {
    const [cr, cc] = stack.pop();
    for (const [nr, nc] of neighbors(cr, cc)) {
      if (!inGrid(nr, nc) || game.grid[nr][nc] == null) continue;
      const k = nr + ',' + nc;
      if (anchored.has(k)) continue;
      anchored.add(k); stack.push([nr, nc]);
    }
  }
  for (let r = 0; r < GRID_ROWS; r++) {
    for (let c = 0; c < rowLen(r); c++) {
      if (game.grid[r][c] != null && !anchored.has(r + ',' + c)) {
        game.falling.push({ x: cellX(r, c), y: cellY(r, c), color: game.grid[r][c], vy: 30 });
        game.grid[r][c] = null;
        game.score += 20;
      }
    }
  }
}

function gameOver() {
  if (game.over) return;
  game.over = true;
  if (game.score > bestScore()) localStorage.setItem(BEST_KEY, game.score);
  document.getElementById('over-score').textContent = t('finalScore', game.score);
  document.getElementById('over-best').textContent = t('bestScore', bestScore());
  showOverlay('overlay-over');
}

// ---- update / render ---------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }
  if (g.shot) {
    const s = g.shot;
    s.x += s.vx * dt; s.y += s.vy * dt;
    if (s.x < BUB / 2) { s.x = BUB / 2; s.vx = -s.vx; }
    if (s.x > VW - BUB / 2) { s.x = VW - BUB / 2; s.vx = -s.vx; }
    if (s.y <= BUB / 2 + 1 || bubbleAt(s.x, s.y)) snapShot();
  }
  for (const p of g.pops) p.life -= dt * 2.6;
  g.pops = g.pops.filter(p => p.life > 0);
  for (const f of g.falling) { f.y += f.vy * dt; f.vy += 700 * dt; }
  g.falling = g.falling.filter(f => f.y < VH + 40);
}

function render() {
  const g = game;
  ctx.fillStyle = '#141022';
  ctx.fillRect(0, 0, VW, VH);
  // danger line
  ctx.fillStyle = 'rgba(255,90,90,0.4)';
  for (let x = 0; x < VW; x += 16) ctx.fillRect(x, DANGER_Y, 9, 2);
  // settled bubbles
  for (let r = 0; r < GRID_ROWS; r++) {
    for (let c = 0; c < rowLen(r); c++) {
      if (g.grid[r][c] != null) drawBubble(ctx, cellX(r, c), cellY(r), BUB_COLORS[g.grid[r][c]], 1);
    }
  }
  for (const f of g.falling) drawBubble(ctx, f.x, f.y, BUB_COLORS[f.color], 1);
  for (const p of g.pops) drawBubble(ctx, p.x, p.y, BUB_COLORS[p.color], p.life);
  if (g.shot) drawBubble(ctx, g.shot.x, g.shot.y, BUB_COLORS[g.shot.color], 1);
  if (!g.over && !g.shot) drawAim(ctx, LX, LY, g.angle);
  drawLauncher(ctx, LX, LY, g.angle, BUB_COLORS[g.nextColor]);
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life);
    ctx.fillStyle = '#ffe9a0';
    ctx.font = '900 24px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, VH / 2);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
  document.getElementById('hud-score').textContent = g.score;
}

function bestScore() { return +(localStorage.getItem(BEST_KEY) || 0); }

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function startGame() { newGame(); hideAllOverlays(); showScreen('screen-game'); }
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
  showScreen('screen-title');
}
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideAllOverlays();
}

// ---- input -------------------------------------------------------------
function aimTo(clientX, clientY) {
  const r = canvas.getBoundingClientRect();
  const x = (clientX - r.left) / r.width * VW;
  const y = (clientY - r.top) / r.height * VH;
  let a = Math.atan2(x - LX, LY - y);
  game.angle = Math.max(-1.45, Math.min(1.45, a));
}
canvas.addEventListener('pointerdown', e => { e.preventDefault(); aimTo(e.clientX, e.clientY); });
canvas.addEventListener('pointermove', e => { if (game && !game.over) aimTo(e.clientX, e.clientY); });
canvas.addEventListener('pointerup', e => { aimTo(e.clientX, e.clientY); shoot(); });
addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (e.key === 'ArrowLeft') game.angle = Math.max(-1.45, game.angle - 0.07);
  if (e.key === 'ArrowRight') game.angle = Math.min(1.45, game.angle + 0.07);
  if (e.key === ' ') { e.preventDefault(); shoot(); }
  if (e.key === 'Escape') togglePause();
});

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestScore', bestScore());
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.04, (now - lastT) / 1000);
  lastT = now;
  if (game && !game.over && !document.getElementById('screen-game').classList.contains('hidden') && overlaysClosed()) {
    update(dt);
  }
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) render();
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
