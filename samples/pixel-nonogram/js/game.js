// Pixel Nonogram - clue logic, grid input, rendering, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-nonogram-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') {
      return { unlocked: d.unlocked, stars: d.stars || {} };
    }
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

const LW = 96, TW = 96;
let state = null;
let mode = 'fill';
const stroke = { active: false, blocked: false, last: -1 };

// ---- puzzle setup --------------------------------------------------------
function loadPuzzle(index) {
  const pz = PUZZLES[index];
  const H = pz.grid.length, W = pz.grid[0].length;
  const sol = [];
  for (let r = 0; r < H; r++) {
    for (let c = 0; c < W; c++) sol.push(pz.grid[r][c] === '1' ? 1 : 0);
  }
  const rowClues = [], colClues = [];
  for (let r = 0; r < H; r++) {
    rowClues.push(lineClue(sol.slice(r * W, r * W + W)));
  }
  for (let c = 0; c < W; c++) {
    const col = [];
    for (let r = 0; r < H; r++) col.push(sol[r * W + c]);
    colClues.push(lineClue(col));
  }
  const cs = Math.min(46, Math.floor(Math.min((354 - LW) / W, (384 - TW) / H)));
  const boardW = LW + W * cs, boardH = TW + H * cs;
  state = {
    index, pz, W, H, sol, rowClues, colClues, cs,
    cells: new Array(W * H).fill(0),
    pop: new Array(W * H).fill(0),
    flash: new Array(W * H).fill(0),
    total: sol.reduce((a, b) => a + b, 0),
    filled: 0, mistakes: 0, time: 0, running: false, won: false, winT: 0,
    gridX: Math.round((VW - boardW) / 2) + LW,
    gridY: Math.round(46 + (384 - boardH) / 2) + TW,
    sparkle: [],
  };
  updateHud();
  updateModeBtn();
}

// ---- clue helpers --------------------------------------------------------
function arrEq(a, b) {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}
// Has the player's row/column been filled to exactly match its clue?
function lineSolved(isRow, idx) {
  const { W, H, cells } = state;
  const line = [];
  if (isRow) for (let c = 0; c < W; c++) line.push(cells[idx * W + c] === 1 ? 1 : 0);
  else for (let r = 0; r < H; r++) line.push(cells[r * W + idx] === 1 ? 1 : 0);
  return arrEq(lineClue(line), isRow ? state.rowClues[idx] : state.colClues[idx]);
}

// ---- play actions --------------------------------------------------------
function fillCell(idx) {
  if (state.won || stroke.blocked) return;
  if (state.cells[idx] !== 0) return;
  if (!state.running) state.running = true;
  if (state.sol[idx] === 1) {
    state.cells[idx] = 1;
    state.pop[idx] = 1;
    state.filled++;
    if (state.filled >= state.total) winPuzzle();
  } else {
    state.cells[idx] = 2;
    state.flash[idx] = 1;
    state.mistakes++;
    stroke.blocked = true;
    updateHud();
  }
}
function toggleMark(idx) {
  if (state.won) return;
  if (state.cells[idx] === 1) return;
  if (!state.running) state.running = true;
  state.cells[idx] = state.cells[idx] === 2 ? 0 : 2;
}
function setMark(idx) {
  if (state.won) return;
  if (state.cells[idx] === 0) state.cells[idx] = 2;
}

function winPuzzle() {
  state.won = true;
  state.running = false;
  state.winT = 0;
  for (let i = 0; i < state.W * state.H; i++) {
    if (state.cells[i] === 1) {
      state.sparkle.push({
        x: state.gridX + (i % state.W) * state.cs + state.cs / 2,
        y: state.gridY + ((i / state.W) | 0) * state.cs + state.cs / 2,
        vx: (Math.random() - 0.5) * 90, vy: -40 - Math.random() * 110,
        life: 0.6 + Math.random() * 0.5,
      });
    }
  }
  const stars = state.mistakes === 0 ? 3 : state.mistakes <= 2 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < PUZZLE_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  state.winStars = stars;
  setTimeout(() => showWin(stars), 900);
}

function showWin(stars) {
  document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
  document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
  document.getElementById('win-name').textContent = t('nameLine', state.pz.name[langIdx()]);
  document.getElementById('win-line').textContent = t('winLine', fmtTime(state.time), state.mistakes);
  document.getElementById('btn-next').style.display =
    state.index + 1 < PUZZLE_COUNT ? '' : 'none';
  showOverlay('overlay-win');
}
function langIdx() { return currentLang === 'zh' ? 1 : 0; }

// ---- HUD -----------------------------------------------------------------
function fmtTime(s) {
  const m = Math.floor(s / 60), sec = Math.floor(s % 60);
  return m + ':' + String(sec).padStart(2, '0');
}
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  document.getElementById('hud-miss').textContent = '✕ ' + state.mistakes;
  document.getElementById('hud-time').textContent = fmtTime(state.time);
}
function updateModeBtn() {
  const btn = document.getElementById('btn-mode');
  btn.textContent = mode === 'fill' ? t('modeFill') : t('modeMark');
  btn.classList.toggle('mark', mode === 'mark');
}

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.running && !state.won) { state.time += dt; updateHud(); }
  for (let i = 0; i < state.pop.length; i++) {
    if (state.pop[i] > 0) state.pop[i] = Math.max(0, state.pop[i] - dt * 6);
    if (state.flash[i] > 0) state.flash[i] = Math.max(0, state.flash[i] - dt * 2);
  }
  if (state.won) state.winT += dt;
  for (const s of state.sparkle) {
    s.life -= dt; s.x += s.vx * dt; s.y += s.vy * dt; s.vy += 240 * dt;
  }
  state.sparkle = state.sparkle.filter(s => s.life > 0);
}

function render() {
  drawBackground(ctx);
  if (!state) return;
  const { W, H, cs, gridX, gridY, sol, cells } = state;
  const pcl = Math.max(2, Math.round(cs * 0.16));
  const color = state.pz.color;

  // clue strips
  for (let r = 0; r < H; r++) {
    const clue = state.rowClues[r];
    const solved = lineSolved(true, r);
    let rx = gridX - 4;
    const ty = gridY + r * cs + Math.round((cs - 4 * pcl) / 2);
    for (let k = clue.length - 1; k >= 0; k--) {
      const w = String(clue[k]).length * 4 * pcl - pcl;
      drawClueNumber(ctx, clue[k], rx, ty, pcl, solved ? '#4a536b' : '#cfd6e6');
      rx -= w + pcl * 2;
    }
  }
  for (let c = 0; c < W; c++) {
    const clue = state.colClues[c];
    const solved = lineSolved(false, c);
    const cx = gridX + c * cs + cs / 2;
    let by = gridY - 4 - 4 * pcl;
    for (let k = clue.length - 1; k >= 0; k--) {
      const w = String(clue[k]).length * 4 * pcl - pcl;
      drawClueNumber(ctx, clue[k], cx + w / 2, by, pcl, solved ? '#4a536b' : '#cfd6e6');
      by -= 4 * pcl + pcl;
    }
  }

  // grid backing
  ctx.fillStyle = '#05060c';
  ctx.fillRect(gridX - 1, gridY - 1, W * cs + 2, H * cs + 2);

  for (let r = 0; r < H; r++) {
    for (let c = 0; c < W; c++) {
      const i = r * W + c;
      const x = gridX + c * cs, y = gridY + r * cs;
      let st = cells[i];
      if (state.won && st === 2) st = 0;
      const popScale = st === 1 ? 1 - state.pop[i] * 0.55 : 1;
      drawCell(ctx, x, y, cs - 1, st, color, popScale);
      if (state.flash[i] > 0) {
        ctx.fillStyle = 'rgba(255, 80, 70, ' + state.flash[i] * 0.6 + ')';
        ctx.fillRect(x, y, cs - 1, cs - 1);
      }
    }
  }
  // every-5 group separators
  ctx.fillStyle = 'rgba(150, 165, 200, 0.4)';
  for (let c = 5; c < W; c += 5) ctx.fillRect(gridX + c * cs - 1, gridY, 2, H * cs);
  for (let r = 5; r < H; r += 5) ctx.fillRect(gridX, gridY + r * cs - 1, W * cs, 2);

  for (const s of state.sparkle) {
    ctx.globalAlpha = Math.min(1, s.life * 1.8);
    ctx.fillStyle = '#f2cf3f';
    ctx.fillRect(s.x | 0, s.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;
}

// ---- input ---------------------------------------------------------------
function cellAt(clientX, clientY) {
  const rect = canvas.getBoundingClientRect();
  const x = (clientX - rect.left) * VW / rect.width;
  const y = (clientY - rect.top) * VH / rect.height;
  const c = Math.floor((x - state.gridX) / state.cs);
  const r = Math.floor((y - state.gridY) / state.cs);
  if (c < 0 || r < 0 || c >= state.W || r >= state.H) return -1;
  return r * state.W + c;
}
function gameActive() {
  return state && !state.won &&
    !document.getElementById('screen-game').classList.contains('hidden');
}

canvas.addEventListener('pointerdown', e => {
  if (!gameActive()) return;
  const idx = cellAt(e.clientX, e.clientY);
  if (idx < 0) return;
  stroke.active = true;
  stroke.blocked = false;
  stroke.last = idx;
  if (mode === 'fill') fillCell(idx);
  else toggleMark(idx);
});
canvas.addEventListener('pointermove', e => {
  if (!stroke.active || !gameActive()) return;
  const idx = cellAt(e.clientX, e.clientY);
  if (idx < 0 || idx === stroke.last) return;
  stroke.last = idx;
  if (mode === 'fill') fillCell(idx);
  else setMark(idx);
});
function endStroke() { stroke.active = false; }
canvas.addEventListener('pointerup', endStroke);
canvas.addEventListener('pointercancel', endStroke);

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  hideAllOverlays();
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
}

function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  PUZZLES.forEach((pz, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) +
      '</span><em>' + pz.grid[0].length + '×' + pz.grid.length + '</em>';
    if (!locked) btn.onclick = () => startPuzzle(i);
    grid.appendChild(btn);
  });
}
function startPuzzle(index) {
  loadPuzzle(index);
  showScreen('screen-game');
}

// ---- buttons -------------------------------------------------------------
document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < PUZZLE_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startPuzzle(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-mode').onclick = () => {
  mode = mode === 'fill' ? 'mark' : 'fill';
  updateModeBtn();
};
document.getElementById('btn-next').onclick = () => startPuzzle(Math.min(state.index + 1, PUZZLE_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startPuzzle(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
  updateModeBtn();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (!document.getElementById('screen-game').classList.contains('hidden')) {
    update(dt);
    render();
  } else {
    drawBackground(ctx);
  }
  requestAnimationFrame(loop);
}
showScreen('screen-title');
requestAnimationFrame(loop);
