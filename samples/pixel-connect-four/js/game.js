// Pixel Connect Four - turn flow, drop animation, AI hand-off, stats save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-connect-four-stats';
function loadStats() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && d['0']) return d;
  } catch (e) { /* ignore */ }
  return { 0: { w: 0, l: 0, d: 0 }, 1: { w: 0, l: 0, d: 0 }, 2: { w: 0, l: 0, d: 0 } };
}
function saveStats() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(stats)); } catch (e) { /* ignore */ }
}
let stats = loadStats();

const CELL = 46, GX = Math.round((VW - CELL * COLS) / 2), GY = 128;
let state = null;

// ---- game setup ----------------------------------------------------------
function newGame(diff) {
  state = {
    board: emptyBoard(), diff,
    turn: PLAYER, status: 'play',
    result: null, winLine: null,
    falling: null, aiTimer: 0,
  };
  updateHud();
}

function startFall(col, row, player) {
  state.falling = {
    col, row, player,
    y: GY - CELL, vy: 0,
    targetY: GY + row * CELL + CELL / 2,
  };
}

function playerDrop(col) {
  if (!state || state.status !== 'play' || state.turn !== PLAYER || state.falling) return;
  const r = dropRow(state.board, col);
  if (r < 0) return;
  startFall(col, r, PLAYER);
}

function resolveMove(fc) {
  if (winsAt(state.board, fc.player, fc.row, fc.col)) {
    gameOver(fc.player === PLAYER ? 'win' : 'lose', findWin(state.board, fc.player));
  } else if (boardFull(state.board)) {
    gameOver('draw', null);
  } else {
    state.turn = fc.player === PLAYER ? AI : PLAYER;
    if (state.turn === AI) state.aiTimer = 0.5;
    updateHud();
  }
}

function gameOver(result, winLine) {
  state.status = 'over';
  state.result = result;
  state.winLine = winLine;
  const rec = stats[state.diff];
  if (result === 'win') rec.w++;
  else if (result === 'lose') rec.l++;
  else rec.d++;
  saveStats();
  setTimeout(() => {
    document.getElementById('over-title').textContent = t(result);
    document.getElementById('over-line').textContent =
      t(result === 'win' ? 'winMsg' : result === 'lose' ? 'loseMsg' : 'drawMsg');
    showScreen('overlay-over', true);
  }, 700);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  if (!state || state.status !== 'play') return;
  if (state.falling) {
    const f = state.falling;
    f.vy += 1900 * dt;
    f.y += f.vy * dt;
    if (f.y >= f.targetY) {
      state.board[f.row][f.col] = f.player;
      state.falling = null;
      resolveMove(f);
    }
    return;
  }
  if (state.turn === AI) {
    state.aiTimer -= dt;
    if (state.aiTimer <= 0) {
      state.aiTimer = 999;
      const col = aiMove(state.board, DIFFICULTIES[state.diff].depth);
      const r = dropRow(state.board, col);
      if (r >= 0) startFall(col, r, AI);
    }
  }
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  drawBoard(ctx, GX, GY, CELL, state.board, state.falling ? state.falling.col : -1);
  if (state.falling) {
    const f = state.falling;
    drawDisc(ctx, GX + f.col * CELL + CELL / 2, f.y, CELL * 0.4, f.player, false);
  }
  if (state.winLine) {
    for (const [r, c] of state.winLine) {
      drawDisc(ctx, GX + c * CELL + CELL / 2, GY + r * CELL + CELL / 2,
        CELL * 0.4, state.board[r][c], true);
    }
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-diff').textContent =
    DIFFICULTIES[state.diff].name[currentLang === 'zh' ? 1 : 0].toUpperCase();
  const turn = document.getElementById('hud-turn');
  turn.textContent = state.turn === PLAYER ? t('yourTurn') : t('aiTurn');
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  if (py < GY - CELL || py > GY + ROWS * CELL) return;
  const col = Math.floor((px - GX) / CELL);
  if (col >= 0 && col < COLS) playerDrop(col);
});

// ---- screens -------------------------------------------------------------
function showScreen(id, isOverlay) {
  if (!isOverlay) {
    document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
    document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  }
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  let w = 0, l = 0, d = 0;
  for (const k of ['0', '1', '2']) { w += stats[k].w; l += stats[k].l; d += stats[k].d; }
  document.getElementById('title-stats').textContent =
    (w + l + d) > 0 ? t('statsLine', w, l, d) : t('noStats');
}
function buildDiffRow() {
  const row = document.getElementById('diff-row');
  row.innerHTML = '';
  DIFFICULTIES.forEach((dd, i) => {
    const btn = document.createElement('button');
    btn.textContent = dd.name[currentLang === 'zh' ? 1 : 0];
    if (i !== 2) btn.className = 'ghost';
    btn.onclick = () => { newGame(i); showScreen('screen-game'); };
    row.appendChild(btn);
  });
}

document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };
document.getElementById('btn-over-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };
document.getElementById('btn-again').onclick = () => { newGame(state.diff); showScreen('screen-game'); };

setupLanguageToggle(() => { refreshTitle(); buildDiffRow(); updateHud(); });

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
refreshTitle();
buildDiffRow();
showScreen('screen-title');
requestAnimationFrame(loop);
