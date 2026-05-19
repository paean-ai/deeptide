// Pixel Plumber - pipe placement, water flow simulation, progress save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-plumber-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, stars: d.stars || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, stars: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

const QUEUE_LEN = 6;
let state = null;

// ---- level setup ---------------------------------------------------------
function buildLevel(index) {
  const lv = LEVELS[index];
  const rng = seededRandom(lv.seed);
  const grid = Array.from({ length: lv.rows }, () => new Array(lv.cols).fill(null));
  grid[lv.source.r][lv.source.c] = 'source';
  const queue = [];
  for (let i = 0; i < QUEUE_LEN; i++) queue.push(PIECE_BAG[(rng() * PIECE_BAG.length) | 0]);

  const cell = Math.floor(Math.min(344 / lv.cols, 300 / lv.rows));
  const gw = cell * lv.cols, gh = cell * lv.rows;
  const st = STEP[lv.source.dir];
  state = {
    index, lv, rng, grid, queue, cell,
    cols: lv.cols, rows: lv.rows,
    gx: Math.round((VW - gw) / 2),
    gy: Math.round(100 + (304 - gh) / 2),
    flooded: Array.from({ length: lv.rows }, () => new Array(lv.cols).fill(false)),
    front: { r: lv.source.r + st[0], c: lv.source.c + st[1], from: OPP[lv.source.dir] },
    t: 0, waterStarted: false, flowTimer: 0, status: 'build',
    overwrites: 0, pulse: null,
  };
  updateHud();
}

function inBounds(r, c) { return r >= 0 && c >= 0 && r < state.rows && c < state.cols; }

// ---- placement -----------------------------------------------------------
function placePiece(r, c) {
  if (!state || (state.status !== 'build' && state.status !== 'flow')) return;
  if (!inBounds(r, c)) return;
  if (state.grid[r][c] === 'source' || state.flooded[r][c]) return;
  if (state.grid[r][c] !== null) state.overwrites++;
  state.grid[r][c] = state.queue.shift();
  state.queue.push(PIECE_BAG[(state.rng() * PIECE_BAG.length) | 0]);
}

// ---- water flow ----------------------------------------------------------
function advanceWater() {
  const f = state.front;
  if (!inBounds(f.r, f.c)) { loseGame(); return; }
  const piece = state.grid[f.r][f.c];
  if (!piece || piece === 'source') { loseGame(); return; }
  const op = PIECES[piece];
  if (!op.includes(f.from)) { loseGame(); return; }
  state.flooded[f.r][f.c] = true;
  state.pulse = { r: f.r, c: f.c, t: 0.32 };
  if (f.r === state.lv.goal.r && f.c === state.lv.goal.c) { winGame(); return; }
  const exit = piece === 'x' ? OPP[f.from] : op.find(d => d !== f.from);
  const st = STEP[exit];
  state.front = { r: f.r + st[0], c: f.c + st[1], from: OPP[exit] };
}

function winGame() {
  state.status = 'win';
  const stars = state.overwrites === 0 ? 3 : state.overwrites <= 4 ? 2 : 1;
  const i = state.index;
  if ((progress.stars[i] || 0) < stars) progress.stars[i] = stars;
  if (i + 1 < LEVEL_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = stars === 3 ? t('perfect') : t('win');
    document.getElementById('win-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('win-line').textContent = t('winLine', state.overwrites);
    document.getElementById('btn-next').style.display = i + 1 < LEVEL_COUNT ? '' : 'none';
    showOverlay('overlay-win');
  }, 650);
}
function loseGame() {
  state.status = 'lose';
  setTimeout(() => showOverlay('overlay-lose'), 650);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.pulse) { state.pulse.t -= dt; if (state.pulse.t <= 0) state.pulse = null; }
  if (state.status !== 'build' && state.status !== 'flow') return;
  state.t += dt;
  if (!state.waterStarted && state.t >= state.lv.delay) {
    state.waterStarted = true;
    state.status = 'flow';
    state.flooded[state.lv.source.r][state.lv.source.c] = true;
    state.flowTimer = 0;
  }
  if (state.waterStarted && state.status === 'flow') {
    state.flowTimer += dt;
    let guard = 0;
    while (state.flowTimer >= state.lv.interval && state.status === 'flow' && guard < 50) {
      state.flowTimer -= state.lv.interval;
      advanceWater();
      guard++;
    }
  }
  updateHud();
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  const { cell, gx, gy } = state;

  // upcoming-piece queue
  const qn = 6, qsize = 34, qx = Math.round((VW - qn * qsize) / 2), qy = 54;
  for (let i = 0; i < qn; i++) {
    const x = qx + i * qsize;
    ctx.fillStyle = i === 0 ? '#243f4e' : '#16262f';
    ctx.fillRect(x + 1, qy + 1, qsize - 2, qsize - 2);
    if (state.queue[i]) drawPipe(ctx, x, qy, qsize, PIECES[state.queue[i]], false);
  }

  // grid
  ctx.fillStyle = '#05151c';
  ctx.fillRect(gx - 2, gy - 2, state.cols * cell + 4, state.rows * cell + 4);
  for (let r = 0; r < state.rows; r++) {
    for (let c = 0; c < state.cols; c++) {
      const x = gx + c * cell, y = gy + r * cell;
      ctx.fillStyle = '#0e1f28';
      ctx.fillRect(x, y, cell, cell);
      ctx.strokeStyle = '#1d3340';
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, cell - 1, cell - 1);
      const piece = state.grid[r][c];
      if (piece === 'source') drawSource(ctx, x, y, cell, state.lv.source.dir);
      else if (piece) drawPipe(ctx, x, y, cell, PIECES[piece], state.flooded[r][c]);
      if (r === state.lv.goal.r && c === state.lv.goal.c) {
        drawGoal(ctx, x, y, cell, state.flooded[r][c]);
      }
    }
  }
  // highlight where the water will enter, before it starts
  if (state.status === 'build' && inBounds(state.front.r, state.front.c)) {
    const x = gx + state.front.c * cell, y = gy + state.front.r * cell;
    ctx.strokeStyle = 'rgba(79,184,232,0.7)';
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 1.5, y + 1.5, cell - 3, cell - 3);
  }
  // flood pulse
  if (state.pulse) {
    const x = gx + state.pulse.c * cell, y = gy + state.pulse.r * cell;
    ctx.fillStyle = 'rgba(159,224,244,' + (state.pulse.t / 0.32 * 0.5) + ')';
    ctx.fillRect(x, y, cell, cell);
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  const flow = document.getElementById('hud-flow');
  if (!state.waterStarted) flow.textContent = t('flowIn', Math.ceil(state.lv.delay - state.t));
  else flow.textContent = t('flowing');
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || (state.status !== 'build' && state.status !== 'flow')) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const c = Math.floor((px - state.gx) / state.cell);
  const r = Math.floor((py - state.gy) / state.cell);
  placePiece(r, c);
});

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
  LEVELS.forEach((lv, i) => {
    const btn = document.createElement('button');
    btn.className = 'level-cell';
    const locked = i + 1 > progress.unlocked;
    const st = progress.stars[i] || 0;
    if (locked) btn.classList.add('locked');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : '★'.repeat(st) + '☆'.repeat(3 - st)) + '</span>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildLevel(index);
  showScreen('screen-game');
}

document.getElementById('btn-play').onclick = () => {
  let next = 0;
  for (let i = 0; i < LEVEL_COUNT; i++) if (i + 1 <= progress.unlocked) next = i;
  startLevel(next);
};
document.getElementById('btn-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => showScreen('screen-title');
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, LEVEL_COUNT - 1));
document.getElementById('btn-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-win-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-lose-retry').onclick = () => startLevel(state.index);
document.getElementById('btn-lose-levels').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  updateHud();
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
