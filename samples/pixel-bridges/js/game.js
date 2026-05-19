// Pixel Bridges - puzzle play, bridge cycling, win detection, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-bridges-save';
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, done: d.done || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, done: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}
let progress = loadProgress();

let state = null;   // { index, pz, geom, bridges[], counts[], inc[], cross[], won, hover }

function buildLevel(index) {
  const pz = buildPuzzle(PUZZLES[index]);
  const inc = pz.islands.map(() => []);
  pz.edges.forEach((e, i) => { inc[e.a].push(i); inc[e.b].push(i); });
  const cross = pz.edges.map(() => []);
  for (let i = 0; i < pz.edges.length; i++) {
    for (let j = i + 1; j < pz.edges.length; j++) {
      if (edgesCross(pz.edges[i], pz.edges[j])) { cross[i].push(j); cross[j].push(i); }
    }
  }
  state = {
    index, pz, geom: boardGeom(pz),
    bridges: pz.edges.map(() => 0),
    counts: pz.islands.map(() => 0),
    inc, cross, won: false, hover: -1,
  };
  recount();
  updateHud();
}

function recount() {
  state.counts = state.pz.islands.map(() => 0);
  state.pz.edges.forEach((e, i) => {
    state.counts[e.a] += state.bridges[i];
    state.counts[e.b] += state.bridges[i];
  });
}

function allConnected() {
  const n = state.pz.islands.length;
  const par = []; for (let i = 0; i < n; i++) par.push(i);
  const find = x => { while (par[x] !== x) x = par[x] = par[par[x]]; return x; };
  state.pz.edges.forEach((e, i) => { if (state.bridges[i] > 0) par[find(e.a)] = find(e.b); });
  const root = find(0);
  for (let i = 0; i < n; i++) if (find(i) !== root) return false;
  return true;
}

function cycleEdge(i) {
  if (state.won) return;
  let next = (state.bridges[i] + 1) % 3;
  if (next > 0 && state.cross[i].some(j => state.bridges[j] > 0)) next = 0;
  state.bridges[i] = next;
  recount();
  checkWin();
}

function checkWin() {
  const ok = state.pz.islands.every((is, k) => state.counts[k] === is.n);
  if (!ok || !allConnected()) return;
  state.won = true;
  const i = state.index;
  progress.done[i] = true;
  if (i + 1 < PUZZLE_COUNT && progress.unlocked < i + 2) progress.unlocked = i + 2;
  saveProgress();
  setTimeout(() => {
    document.getElementById('win-title').textContent = t('win');
    document.getElementById('win-line').textContent = t('winLine');
    document.getElementById('btn-next').style.display = i + 1 < PUZZLE_COUNT ? '' : 'none';
    document.getElementById('overlay-win').classList.remove('hidden');
  }, 400);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (state) drawBoard(ctx, state.pz, state.geom, state);
}
function loop() {
  render();
  requestAnimationFrame(loop);
}

function updateHud() {
  if (!state) return;
  document.getElementById('hud-level').textContent = t('level') + ' ' + (state.index + 1);
  const done = state.pz.islands.filter((is, k) => state.counts[k] === is.n).length;
  document.getElementById('hud-prog').textContent = done + '/' + state.pz.islands.length;
}

// ---- input ---------------------------------------------------------------
function distToSeg(px, py, ax, ay, bx, by) {
  const dx = bx - ax, dy = by - ay;
  const len2 = dx * dx + dy * dy || 1;
  let t = ((px - ax) * dx + (py - ay) * dy) / len2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (ax + dx * t), py - (ay + dy * t));
}

canvas.addEventListener('pointerdown', e => {
  if (!state || state.won) return;
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  const px = (e.clientX - rect.left) * VW / rect.width;
  const py = (e.clientY - rect.top) * VH / rect.height;
  let best = -1, bestD = state.geom.cell * 0.42;
  state.pz.edges.forEach((ed, i) => {
    const pa = islandXY(state.geom, state.pz.islands[ed.a]);
    const pb = islandXY(state.geom, state.pz.islands[ed.b]);
    const d = distToSeg(px, py, pa.x, pa.y, pb.x, pb.y);
    if (d < bestD) { bestD = d; best = i; }
  });
  if (best >= 0) { cycleEdge(best); updateHud(); }
});

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById('overlay-win').classList.add('hidden');
  document.getElementById(id).classList.remove('hidden');
}
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  PUZZLES.forEach((p, i) => {
    const locked = i + 1 > progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (locked ? ' locked' : '');
    btn.innerHTML = '<b>' + (i + 1) + '</b><span>' +
      (locked ? '🔒' : progress.done[i] ? '✓' : '·') + '</span><em>' +
      p.name[lang === 'en' ? 0 : 1] + '</em>';
    if (!locked) btn.onclick = () => startLevel(i);
    grid.appendChild(btn);
  });
}
function startLevel(index) {
  buildLevel(index);
  showScreen('screen-game');
}
function refreshText() {
  document.getElementById('title-h').textContent = t('title');
  document.getElementById('title-tag').textContent = t('tagline');
  document.getElementById('title-howto').textContent = t('howto');
  document.getElementById('btn-play').textContent = t('play');
  document.getElementById('levels-h').textContent = t('pick');
  document.getElementById('btn-levels-back').textContent = t('menu');
  document.getElementById('btn-restart').textContent = t('restart');
  document.getElementById('btn-game-menu').textContent = t('menu');
  document.getElementById('btn-next').textContent = t('next');
  document.getElementById('btn-win-menu').textContent = t('menu');
}

document.getElementById('btn-play').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-levels-back').onclick = () => showScreen('screen-title');
document.getElementById('btn-game-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };
document.getElementById('btn-restart').onclick = () => startLevel(state.index);
document.getElementById('btn-next').onclick = () => startLevel(Math.min(state.index + 1, PUZZLE_COUNT - 1));
document.getElementById('btn-win-menu').onclick = () => { buildLevelGrid(); showScreen('screen-levels'); };

setupLanguageToggle(() => {
  refreshText();
  updateHud();
  if (!document.getElementById('screen-levels').classList.contains('hidden')) buildLevelGrid();
});

refreshText();
showScreen('screen-title');
requestAnimationFrame(loop);
