// Pixel Dots & Boxes - turn flow, AI hand-off, screens, stats.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-dots-boxes-save';
function loadStats() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && d.rec) return d;
  } catch (e) { /* ignore */ }
  return { rec: [[0, 0, 0], [0, 0, 0], [0, 0, 0]] };
}
function saveStats() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(stats)); } catch (e) { /* ignore */ }
}
let stats = loadStats();

let game = null;   // { state, diff, turn, busy, lastEdge, over }

// ---- nearest undrawn edge to a tap ---------------------------------------
function edgeAt(px, py) {
  const g = boardGeom();
  let best = null, bestD = g.cell * 0.42;
  const consider = (e, mx, my) => {
    const d = Math.hypot(px - mx, py - my);
    if (d < bestD) { bestD = d; best = e; }
  };
  for (let r = 0; r <= B; r++) for (let c = 0; c < B; c++) {
    if (game.state.h[r][c]) continue;
    const p = dotXY(g, c, r);
    consider({ t: 0, r, c }, p.x + g.cell / 2, p.y);
  }
  for (let r = 0; r < B; r++) for (let c = 0; c <= B; c++) {
    if (game.state.v[r][c]) continue;
    const p = dotXY(g, c, r);
    consider({ t: 1, r, c }, p.x, p.y + g.cell / 2);
  }
  return best;
}

// ---- turn flow -----------------------------------------------------------
function applyMove(edge, who) {
  const done = drawEdge(game.state, edge, who);
  game.lastEdge = edge;
  if (isOver(game.state)) { finishGame(); return; }
  if (done === 0) game.turn = who === PLAYER ? AI : PLAYER;   // no box -> swap
  updateHud();
  if (game.turn === AI && !game.over) scheduleAI();
}

function scheduleAI() {
  game.busy = true;
  updateHud();
  setTimeout(() => {
    if (game.over) return;
    const edge = aiPickEdge(game.state, DIFFICULTIES[game.diff].level);
    game.busy = false;
    if (edge) applyMove(edge, AI);
  }, 420);
}

function finishGame() {
  game.over = true;
  const sc = scores(game.state);
  const r = stats.rec[game.diff];
  let title;
  if (sc.player > sc.ai) { r[0]++; title = t('win'); }
  else if (sc.player < sc.ai) { r[1]++; title = t('lose'); }
  else { r[2]++; title = t('draw'); }
  saveStats();
  updateHud();
  setTimeout(() => {
    document.getElementById('over-title').textContent = title;
    document.getElementById('over-line').textContent = t('result', sc.player, sc.ai);
    document.getElementById('overlay-over').classList.remove('hidden');
  }, 600);
}

// ---- input ---------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!game || game.over || game.busy || game.turn !== PLAYER) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  const edge = edgeAt(px, py);
  if (edge) applyMove(edge, PLAYER);
});

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (game) drawBoard(ctx, game.state, game.lastEdge);
}
function loop() {
  render();
  requestAnimationFrame(loop);
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!game) return;
  document.getElementById('hud-diff').textContent =
    DIFFICULTIES[game.diff].name[lang === 'en' ? 0 : 1].toUpperCase();
  const sc = scores(game.state);
  document.getElementById('hud-score').textContent = '● ' + sc.player + '  ◆ ' + sc.ai;
  document.getElementById('hud-turn').textContent =
    game.over ? '' : game.turn === PLAYER ? t('yourTurn') : t('cpuTurn');
}

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById('overlay-over').classList.add('hidden');
  document.getElementById(id).classList.remove('hidden');
}

let pickedDiff = 1;
function buildDiffRow() {
  const row = document.getElementById('diff-row');
  row.innerHTML = '';
  DIFFICULTIES.forEach((d, i) => {
    const b = document.createElement('button');
    b.className = 'diff' + (i === pickedDiff ? ' on' : '');
    b.textContent = d.name[lang === 'en' ? 0 : 1];
    b.onclick = () => { pickedDiff = i; startGame(); };
    row.appendChild(b);
  });
}

function startGame() {
  game = { state: newState(), diff: pickedDiff, turn: PLAYER, busy: false, lastEdge: null, over: false };
  updateHud();
  showScreen('screen-game');
}

function refreshTitle() {
  document.getElementById('title-h').textContent = t('title');
  document.getElementById('title-tag').textContent = t('tagline');
  document.getElementById('title-pick').textContent = t('difficulty');
  const r = stats.rec[pickedDiff];
  document.getElementById('title-record').textContent = t('record') + ': ' + t('rec', r[0], r[1], r[2]);
  document.getElementById('btn-menu').textContent = t('menu');
  document.getElementById('btn-again').textContent = t('again');
  document.getElementById('btn-over-menu').textContent = t('menu');
  buildDiffRow();
}

document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };
document.getElementById('btn-again').onclick = () => startGame();
document.getElementById('btn-over-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => {
  refreshTitle();
  updateHud();
});

refreshTitle();
showScreen('screen-title');
requestAnimationFrame(loop);
