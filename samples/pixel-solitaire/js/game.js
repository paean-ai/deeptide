// Pixel Solitaire - Klondike rules, one-tap moves, undo, win detection.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-solitaire-save';
function loadStats() {
  try {
    const d = JSON.parse(localStorage.getItem(SAVE_KEY));
    if (d && typeof d.won === 'number') return d;
  } catch (e) { /* ignore */ }
  return { won: 0, bestTime: 0, bestMoves: 0 };
}
let stats = loadStats();

let G = null;
let sparkles = [];

// ---- deal ----------------------------------------------------------------
function newGame() {
  const deck = [];
  for (let s = 0; s < 4; s++) {
    for (let r = 1; r <= 13; r++) deck.push({ rank: r, suit: s, faceUp: false });
  }
  for (let i = deck.length - 1; i > 0; i--) {
    const j = (Math.random() * (i + 1)) | 0;
    [deck[i], deck[j]] = [deck[j], deck[i]];
  }
  G = {
    stock: [], waste: [], foundations: [[], [], [], []],
    tableau: [[], [], [], [], [], [], []],
    moves: 0, time: 0, won: false, history: [],
  };
  let idx = 0;
  for (let col = 0; col < 7; col++) {
    for (let k = 0; k <= col; k++) {
      const card = deck[idx++];
      card.faceUp = (k === col);
      G.tableau[col].push(card);
    }
  }
  while (idx < deck.length) { deck[idx].faceUp = false; G.stock.push(deck[idx++]); }
  sparkles = [];
  updateHud();
}

// ---- undo snapshots ------------------------------------------------------
function snapshot() {
  G.history.push(JSON.stringify({
    stock: G.stock, waste: G.waste, foundations: G.foundations,
    tableau: G.tableau, moves: G.moves,
  }));
  if (G.history.length > 300) G.history.shift();
}
function undo() {
  if (!G || G.won || !G.history.length) return;
  const s = JSON.parse(G.history.pop());
  G.stock = s.stock;
  G.waste = s.waste;
  G.foundations = s.foundations;
  G.tableau = s.tableau;
  G.moves = s.moves;
  updateHud();
}

// ---- layout --------------------------------------------------------------
function colLayout(col) {
  const pile = G.tableau[col];
  let downs = 0;
  for (const c of pile) if (!c.faceUp) downs++;
  const ups = pile.length - downs;
  let upOff = MAX_UP_OFF;
  if (ups > 1) {
    const room = (TABLEAU_BOTTOM - TABLEAU_Y - CARD_H - downs * DOWN_OFF) / (ups - 1);
    upOff = Math.max(7, Math.min(MAX_UP_OFF, room));
  }
  const ys = [];
  let y = TABLEAU_Y;
  for (let i = 0; i < pile.length; i++) {
    ys.push(y);
    y += pile[i].faceUp ? upOff : DOWN_OFF;
  }
  return ys;
}
function inRect(x, y, rx, ry) {
  return x >= rx && x < rx + CARD_W && y >= ry && y < ry + CARD_H;
}

// ---- move rules ----------------------------------------------------------
function foundationAccepts(card) {
  return G.foundations[card.suit].length + 1 === card.rank;
}
function findTableauDest(lead, excludeCol) {
  for (let d = 0; d < 7; d++) {
    if (d === excludeCol) continue;
    const dp = G.tableau[d];
    if (dp.length === 0) {
      if (lead.rank === 13) return d;
    } else {
      const top = dp[dp.length - 1];
      if (top.faceUp && top.rank === lead.rank + 1 && oppositeColor(top, lead)) return d;
    }
  }
  return -1;
}
function flipAfter(col) {
  const pile = G.tableau[col];
  if (pile.length && !pile[pile.length - 1].faceUp) pile[pile.length - 1].faceUp = true;
}

function moveWaste() {
  const card = G.waste[G.waste.length - 1];
  if (foundationAccepts(card)) {
    snapshot();
    G.waste.pop();
    G.foundations[card.suit].push(card);
    G.moves++;
    afterMove();
    return;
  }
  const dest = findTableauDest(card, -1);
  if (dest >= 0) {
    snapshot();
    G.waste.pop();
    G.tableau[dest].push(card);
    G.moves++;
    afterMove();
  }
}

function moveTableau(col, idx) {
  const pile = G.tableau[col];
  const group = pile.slice(idx);
  if (group.length === 1 && foundationAccepts(group[0])) {
    snapshot();
    pile.pop();
    G.foundations[group[0].suit].push(group[0]);
    flipAfter(col);
    G.moves++;
    afterMove();
    return;
  }
  const dest = findTableauDest(group[0], col);
  if (dest >= 0) {
    snapshot();
    pile.splice(idx);
    for (const c of group) G.tableau[dest].push(c);
    flipAfter(col);
    G.moves++;
    afterMove();
  }
}

function drawStock() {
  snapshot();
  if (G.stock.length) {
    const card = G.stock.pop();
    card.faceUp = true;
    G.waste.push(card);
  } else if (G.waste.length) {
    while (G.waste.length) {
      const c = G.waste.pop();
      c.faceUp = false;
      G.stock.push(c);
    }
  } else {
    G.history.pop(); // nothing happened — discard the snapshot
  }
  updateHud();
}

function afterMove() {
  updateHud();
  if (G.foundations.every(f => f.length === 13)) winGame();
}

function winGame() {
  G.won = true;
  stats.won++;
  if (!stats.bestTime || G.time < stats.bestTime) stats.bestTime = Math.floor(G.time);
  if (!stats.bestMoves || G.moves < stats.bestMoves) stats.bestMoves = G.moves;
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(stats)); } catch (e) { /* ignore */ }
  for (let i = 0; i < 60; i++) {
    sparkles.push({
      x: Math.random() * VW, y: VH * 0.4 + Math.random() * 80,
      vx: (Math.random() - 0.5) * 160, vy: -80 - Math.random() * 160,
      life: 0.8 + Math.random() * 0.7,
      c: ['#f2cf3f', '#6fd08a', '#d23b3b', '#fff'][(Math.random() * 4) | 0],
    });
  }
  setTimeout(() => {
    document.getElementById('win-line').textContent = t('wonLine', fmtTime(G.time), G.moves);
    document.getElementById('overlay-win').classList.remove('hidden');
  }, 750);
}

// ---- input ---------------------------------------------------------------
function onTap(px, py) {
  if (!G || G.won) return;
  if (inRect(px, py, colX(0), TOP_Y)) { drawStock(); return; }      // stock
  if (G.waste.length && inRect(px, py, colX(1), TOP_Y)) { moveWaste(); return; }
  for (let col = 0; col < 7; col++) {
    const cx = colX(col);
    if (px < cx || px >= cx + CARD_W) continue;
    const pile = G.tableau[col];
    if (!pile.length) return;
    const ys = colLayout(col);
    for (let i = 0; i < pile.length; i++) {
      const top = i === pile.length - 1;
      const ext = top ? CARD_H : ys[i + 1] - ys[i];
      if (py >= ys[i] && py < ys[i] + ext) {
        if (pile[i].faceUp) moveTableau(col, i);
        return;
      }
    }
    return;
  }
}
canvas.addEventListener('pointerdown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  const rect = canvas.getBoundingClientRect();
  onTap((e.clientX - rect.left) * VW / rect.width, (e.clientY - rect.top) * VH / rect.height);
});

// ---- update / render -----------------------------------------------------
function update(dt) {
  if (!G) return;
  if (!G.won) G.time += dt;
  for (const s of sparkles) {
    s.life -= dt; s.x += s.vx * dt; s.y += s.vy * dt; s.vy += 320 * dt;
  }
  sparkles = sparkles.filter(s => s.life > 0);
}

function render() {
  drawBackground(ctx);
  if (!G) return;
  // stock
  if (G.stock.length) drawCard(ctx, { faceUp: false }, colX(0), TOP_Y);
  else drawSlot(ctx, colX(0), TOP_Y);
  // waste
  if (G.waste.length) drawCard(ctx, G.waste[G.waste.length - 1], colX(1), TOP_Y);
  else drawSlot(ctx, colX(1), TOP_Y);
  // foundations
  for (let s = 0; s < 4; s++) {
    const f = G.foundations[s];
    if (f.length) drawCard(ctx, f[f.length - 1], colX(3 + s), TOP_Y);
    else drawSlot(ctx, colX(3 + s), TOP_Y, SUITS[s].id);
  }
  // tableau
  for (let col = 0; col < 7; col++) {
    const pile = G.tableau[col];
    if (!pile.length) { drawSlot(ctx, colX(col), TABLEAU_Y); continue; }
    const ys = colLayout(col);
    for (let i = 0; i < pile.length; i++) drawCard(ctx, pile[i], colX(col), ys[i]);
  }
  for (const s of sparkles) {
    ctx.globalAlpha = Math.min(1, s.life * 1.6);
    ctx.fillStyle = s.c;
    ctx.fillRect(s.x | 0, s.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;
}

// ---- HUD -----------------------------------------------------------------
function fmtTime(s) {
  const m = Math.floor(s / 60), sec = Math.floor(s % 60);
  return m + ':' + String(sec).padStart(2, '0');
}
function updateHud() {
  if (!G) return;
  document.getElementById('hud-time').textContent = fmtTime(G.time);
  document.getElementById('hud-moves').textContent = 'MOVES ' + G.moves;
}

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  document.getElementById('title-stats').textContent =
    t('statsLine', stats.won, stats.bestTime ? fmtTime(stats.bestTime) : '—');
}
function startGame() { newGame(); showScreen('screen-game'); }

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-again').onclick = startGame;
document.getElementById('btn-new').onclick = startGame;
document.getElementById('btn-undo').onclick = undo;
document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };
document.getElementById('btn-win-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => { refreshTitle(); updateHud(); });

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
showScreen('screen-title');
requestAnimationFrame(loop);
