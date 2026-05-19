// Pixel Mind Match - a memory pairs game with a level campaign.

const SAVE_KEY = 'pixel-mind-match-save';
const CW = 480, CH = 480;

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = CW;
canvas.height = CH;
ctx.imageSmoothingEnabled = false;

let game = null;
let progress = loadProgress();
let lastT = performance.now();

function shuffle(a) {
  for (let i = a.length - 1; i > 0; i--) {
    const j = (Math.random() * (i + 1)) | 0;
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

// ---- progress save -----------------------------------------------------
function loadProgress() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.unlocked === 'number') return { unlocked: d.unlocked, best: d.best || {} };
  } catch (e) { /* ignore */ }
  return { unlocked: 1, best: {} };
}
function saveProgress() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(progress)); } catch (e) { /* ignore */ }
}

// ---- level lifecycle ---------------------------------------------------
function loadLevel(index) {
  const lv = LEVELS[index];
  const pairs = pairsFor(lv);
  const picked = shuffle(CREATURES.slice()).slice(0, pairs);
  const deck = [];
  for (const cr of picked) { deck.push(cr); deck.push(cr); }
  shuffle(deck);
  game = {
    index, level: lv, cols: lv.c, rows: lv.r,
    tiles: deck.map(cr => ({ creature: cr, flipped: true, matched: false, flip: 1 })),
    faceUp: [], moves: 0, matched: 0, combo: 0,
    phase: 'preview', previewT: 1.5 + pairs * 0.06, resolveT: 0, matchFlag: false,
    t: 0, won: false,
  };
  updateHud();
}

function flipTile(i) {
  if (!game || game.phase !== 'play') return;
  const tile = game.tiles[i];
  if (!tile || tile.matched || tile.flipped) return;
  if (game.faceUp.length >= 2) return;
  tile.flipped = true;
  game.faceUp.push(i);
  if (game.faceUp.length === 2) {
    game.moves++;
    const [a, b] = game.faceUp;
    game.matchFlag = game.tiles[a].creature.id === game.tiles[b].creature.id;
    game.phase = 'resolving';
    game.resolveT = game.matchFlag ? 0.4 : 0.78;
    updateHud();
  }
}

function resolvePair() {
  const [a, b] = game.faceUp;
  if (game.matchFlag) {
    game.tiles[a].matched = game.tiles[b].matched = true;
    game.matched += 2;
    game.combo++;
  } else {
    game.tiles[a].flipped = game.tiles[b].flipped = false;
    game.combo = 0;
  }
  game.faceUp = [];
  game.phase = 'play';
  updateHud();
  if (game.matched >= game.tiles.length) winLevel();
}

function winLevel() {
  game.phase = 'won';
  game.won = true;
  const idx = game.index, m = game.moves;
  const prev = progress.best[idx];
  const isBest = prev == null || m < prev;
  if (isBest) progress.best[idx] = m;
  progress.unlocked = Math.max(progress.unlocked, Math.min(LEVEL_COUNT, idx + 2));
  saveProgress();
  const stars = m <= game.level.starMoves[0] ? 3 : m <= game.level.starMoves[1] ? 2 : 1;
  if (idx + 1 >= LEVEL_COUNT) {
    showOverlay('overlay-alldone');
  } else {
    document.getElementById('cleared-stars').textContent = '★'.repeat(stars) + '☆'.repeat(3 - stars);
    document.getElementById('cleared-msg').textContent = t('clearedMsg', m);
    document.getElementById('cleared-best').textContent = isBest ? t('newBest') : t('bestMoves', progress.best[idx]);
    showOverlay('overlay-cleared');
  }
}

// ---- update / render ---------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  if (g.phase === 'preview') {
    g.previewT -= dt;
    if (g.previewT <= 0) {
      for (const tile of g.tiles) tile.flipped = false;
      g.phase = 'play';
    }
  } else if (g.phase === 'resolving') {
    g.resolveT -= dt;
    if (g.resolveT <= 0) resolvePair();
  }
  for (const tile of g.tiles) {
    const target = (tile.flipped || tile.matched) ? 1 : 0;
    tile.flip += (target - tile.flip) * Math.min(1, dt * 14);
  }
}

function layout() {
  const g = game;
  const pad = 14, gap = 6;
  const s = Math.floor(Math.min(
    (CW - pad * 2 - gap * (g.cols - 1)) / g.cols,
    (CH - pad * 2 - gap * (g.rows - 1)) / g.rows));
  const gw = g.cols * s + (g.cols - 1) * gap;
  const gh = g.rows * s + (g.rows - 1) * gap;
  return { s, gap, ox: (CW - gw) / 2, oy: (CH - gh) / 2 };
}

function render() {
  ctx.fillStyle = '#14101f';
  ctx.fillRect(0, 0, CW, CH);
  if (!game) return;
  const { s, gap, ox, oy } = layout();
  for (let i = 0; i < game.tiles.length; i++) {
    const tile = game.tiles[i];
    const cx = i % game.cols, cy = (i / game.cols) | 0;
    const px = ox + cx * (s + gap), py = oy + cy * (s + gap);
    // horizontal flip: |flip*2-1| = 1 (full) at face/back, 0 at the edge
    const sc = Math.abs(tile.flip * 2 - 1);
    const drawW = Math.max(1, s * sc);
    ctx.save();
    ctx.translate(px + (s - drawW) / 2, py);
    ctx.scale(drawW / s, 1);
    if (tile.flip >= 0.5) drawCreature(ctx, 0, 0, s, tile.creature, tile.matched, game.t);
    else drawCardBack(ctx, 0, 0, s);
    ctx.restore();
  }
  updateHud();
}

function updateHud() {
  if (!game) return;
  document.getElementById('hud-level').textContent = `${t('level')} ${game.index + 1}`;
  document.getElementById('hud-moves').textContent = `${t('moves')} ${game.moves}`;
  document.getElementById('hud-combo').textContent = `${t('combo')} ${game.combo > 1 ? 'x' + game.combo : '—'}`;
  const banner = document.getElementById('preview-banner');
  banner.classList.toggle('hidden', game.phase !== 'preview');
  if (game.phase === 'preview') banner.textContent = t('memorise');
}

// ---- level select ------------------------------------------------------
function buildLevelGrid() {
  const grid = document.getElementById('level-grid');
  grid.innerHTML = '';
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const unlocked = i < progress.unlocked;
    const btn = document.createElement('button');
    btn.className = 'level-cell' + (unlocked ? '' : ' locked');
    const b = progress.best[i];
    btn.innerHTML = `<b>${i + 1}</b><span>${unlocked ? (b == null ? '—' : b + 'm') : '🔒'}</span>`;
    btn.disabled = !unlocked;
    btn.onclick = () => { loadLevel(i); showScreen('screen-game'); };
    grid.appendChild(btn);
  }
}

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function gotoTitle() { hideAllOverlays(); showScreen('screen-title'); }
function gotoLevels() { hideAllOverlays(); buildLevelGrid(); showScreen('screen-levels'); }

// ---- input -------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (!game || game.phase !== 'play' || !overlaysClosed()) return;
  const r = canvas.getBoundingClientRect();
  const x = (e.clientX - r.left) / r.width * CW;
  const y = (e.clientY - r.top) / r.height * CH;
  const { s, gap, ox, oy } = layout();
  const cx = Math.floor((x - ox) / (s + gap));
  const cy = Math.floor((y - oy) / (s + gap));
  if (cx < 0 || cy < 0 || cx >= game.cols || cy >= game.rows) return;
  // ignore taps that land in the gap
  if (x - ox - cx * (s + gap) > s || y - oy - cy * (s + gap) > s) return;
  flipTile(cy * game.cols + cx);
});

document.getElementById('btn-play').onclick = () => {
  loadLevel(Math.min(progress.unlocked - 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
document.getElementById('btn-levels').onclick = gotoLevels;
document.getElementById('btn-levels-back').onclick = gotoTitle;
document.getElementById('btn-game-menu').onclick = gotoTitle;
document.getElementById('btn-restart').onclick = () => { if (game) loadLevel(game.index); };
document.getElementById('btn-cleared-menu').onclick = gotoLevels;
document.getElementById('btn-cleared-retry').onclick = () => { hideAllOverlays(); loadLevel(game.index); showScreen('screen-game'); };
document.getElementById('btn-alldone-menu').onclick = gotoTitle;
document.getElementById('btn-next').onclick = () => {
  hideAllOverlays();
  loadLevel(Math.min(game.index + 1, LEVEL_COUNT - 1));
  showScreen('screen-game');
};
setupLanguageToggle(() => { if (game) updateHud(); buildLevelGrid(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) {
    if (overlaysClosed()) update(dt);
    render();
  }
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
