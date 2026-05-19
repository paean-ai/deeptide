// Pixel Reversi - turn flow, passes, AI hand-off, stats save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-reversi-stats';
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

const CELL = 38, GX = Math.round((VW - CELL * N) / 2), GY = 116;
let state = null;

// ---- game setup ----------------------------------------------------------
function newGame(diff) {
  state = {
    board: initialBoard(), diff,
    turn: DARK, status: 'play',
    aiTimer: 0, lastMove: null, pass: null,
  };
  updateHud();
}

function doMove(player, r, c) {
  state.board = applyMove(state.board, player, r, c);
  state.lastMove = { r, c };
  advanceTurn(player);
}

function advanceTurn(mover) {
  const next = opponent(mover);
  if (legalMoves(state.board, next).length) {
    state.turn = next;
    if (next === LIGHT) state.aiTimer = 0.55;
  } else if (legalMoves(state.board, mover).length) {
    state.turn = mover;                          // opponent has no move -> pass
    state.pass = { who: next === DARK ? 'you' : 'ai', t: 1.4 };
    if (mover === LIGHT) state.aiTimer = 0.55;
  } else {
    gameOver();
    return;
  }
  updateHud();
}

function gameOver() {
  state.status = 'over';
  const { dark, light } = countDiscs(state.board);
  const result = dark > light ? 'win' : light > dark ? 'lose' : 'draw';
  const rec = stats[state.diff];
  if (result === 'win') rec.w++;
  else if (result === 'lose') rec.l++;
  else rec.d++;
  saveStats();
  setTimeout(() => {
    document.getElementById('over-title').textContent = t(result);
    document.getElementById('over-line').textContent =
      result === 'win' ? t('winMsg', dark, light)
        : result === 'lose' ? t('loseMsg', light, dark) : t('drawMsg');
    document.getElementById('overlay-over').classList.remove('hidden');
  }, 700);
}

// ---- update --------------------------------------------------------------
function update(dt) {
  if (!state) return;
  if (state.pass) { state.pass.t -= dt; if (state.pass.t <= 0) state.pass = null; }
  if (state.status !== 'play' || state.turn !== LIGHT) return;
  state.aiTimer -= dt;
  if (state.aiTimer <= 0) {
    state.aiTimer = 999;
    const m = aiMove(state.board, DIFFICULTIES[state.diff].depth);
    if (m) doMove(LIGHT, m.r, m.c);
  }
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  drawBoard(ctx, GX, GY, CELL);
  const hints = (state.status === 'play' && state.turn === DARK)
    ? legalMoves(state.board, DARK) : [];
  for (const m of hints) {
    drawHint(ctx, GX + m.c * CELL + CELL / 2, GY + m.r * CELL + CELL / 2);
  }
  for (let r = 0; r < N; r++) {
    for (let c = 0; c < N; c++) {
      if (state.board[r][c] === EMPTY) continue;
      drawDisc(ctx, GX + c * CELL + CELL / 2, GY + r * CELL + CELL / 2,
        CELL * 0.4, state.board[r][c]);
    }
  }
  if (state.lastMove) {
    ctx.strokeStyle = '#f2cf3f';
    ctx.lineWidth = 2;
    ctx.strokeRect(GX + state.lastMove.c * CELL + 2, GY + state.lastMove.r * CELL + 2,
      CELL - 4, CELL - 4);
  }
  if (state.pass) {
    ctx.globalAlpha = Math.min(1, state.pass.t * 2);
    ctx.fillStyle = '#f2cf3f';
    ctx.font = '900 18px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('passNote', state.pass.who), VW / 2, GY - 16);
    ctx.globalAlpha = 1;
    ctx.textAlign = 'left';
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!state) return;
  document.getElementById('hud-diff').textContent =
    DIFFICULTIES[state.diff].name[currentLang === 'zh' ? 1 : 0].toUpperCase();
  const { dark, light } = countDiscs(state.board);
  document.getElementById('hud-score').textContent = '● ' + dark + '  ○ ' + light;
  document.getElementById('hud-turn').textContent =
    state.turn === DARK ? t('yourTurn') : t('aiTurn');
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!state || state.status !== 'play' || state.turn !== DARK) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const c = Math.floor((px - GX) / CELL), r = Math.floor((py - GY) / CELL);
  if (r < 0 || c < 0 || r >= N || c >= N) return;
  if (flipsFor(state.board, DARK, r, c).length) doMove(DARK, r, c);
});

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
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
