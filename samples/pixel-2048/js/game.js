// Pixel 2048 - screen flow, swipe + keyboard input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-2048:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { best: 0, current: null };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startGame(fresh) {
  if (fresh || !save.current) state = newGame();
  else {
    state = newGame();
    state.grid = save.current.grid.map(r => r.slice());
    state.score = save.current.score;
    state.moves = save.current.moves;
    state.won = save.current.won;
    state.keepPlaying = save.current.keepPlaying;
  }
  screen = 'play';
  paint();
}

function persistRun() {
  save.current = state.over ? null : {
    grid: state.grid.map(r => r.slice()),
    score: state.score,
    moves: state.moves,
    won: state.won,
    keepPlaying: state.keepPlaying,
  };
  if (state.score > save.best) save.best = state.score;
  persist();
}

function attemptMove(dir) {
  if (!state || screen !== 'play') return;
  if (state.won && !state.keepPlaying) return;     // win modal blocks input
  const ok = move(state, dir);
  if (ok) { persistRun(); paint(); }
}

// ---- input ------------------------------------------------------------
function relPoint(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (CANVAS_W / r.width),
    y: (e.clientY - r.top)  * (CANVAS_H / r.height),
  };
}
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

let dragStart = null;
canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { startGame(false); return; }
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect()))    { setScreen('title'); return; }
    if (hit(p, undoBtnRect())) { if (undo(state)) { persistRun(); paint(); } return; }
    if (hit(p, newBtnRect()))  { startGame(true); return; }
    if (state.won && !state.keepPlaying) {
      if (hit(p, keepRect())) { state.keepPlaying = true; paint(); return; }
      if (hit(p, restartRect())) { startGame(true); return; }
    }
    if (state.over) {
      if (hit(p, restartRect())) { startGame(true); return; }
      if (hit(p, menuRect())) { setScreen('title'); return; }
    }
    dragStart = p;
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (!dragStart || screen !== 'play') return;
  const p = relPoint(e);
  const dx = p.x - dragStart.x, dy = p.y - dragStart.y;
  const TH = 28;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < TH) return;
  if (Math.abs(dx) > Math.abs(dy)) attemptMove(dx > 0 ? 1 : 3);
  else attemptMove(dy > 0 ? 2 : 0);
  dragStart = null;
});
canvas.addEventListener('pointerup', () => { dragStart = null; });
canvas.addEventListener('pointercancel', () => { dragStart = null; });

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')        attemptMove(0);
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') attemptMove(1);
  else if (e.key === 'ArrowDown'  || e.key === 's' || e.key === 'S') attemptMove(2);
  else if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') attemptMove(3);
  else if (e.key === 'z' || e.key === 'Z') { if (undo(state)) { persistRun(); paint(); } }
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title') paintTitle();
  else if (screen === 'play')  paintPlay();
}

function paintTitle() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#f4d27b';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 110);
  ctx.fillStyle = '#a89c92';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 138);
  // Inline tile demo: two 2s + an arrow + a 4.
  drawTile(ctx, { x: 80,  y: 200, w: 48, h: 48 }, 2, false);
  drawTile(ctx, { x: 140, y: 200, w: 48, h: 48 }, 2, false);
  ctx.fillStyle = '#a89c92';
  ctx.font = 'bold 24px monospace';
  ctx.fillText('→', 220, 230);
  drawTile(ctx, { x: 240, y: 200, w: 48, h: 48 }, 4, true);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  if (save.current) {
    ctx.fillStyle = '#a89c92';
    ctx.font = '10px monospace';
    ctx.fillText(`${t(lang, 'score')} ${save.current.score} · ${t(lang, 'moves')} ${save.current.moves}`,
      CANVAS_W / 2, 365);
  }
  ctx.fillStyle = '#a89c92';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 392);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 408);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 424);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawHud(ctx, lang, state, save.best);
  drawBoard(ctx, state);
  // Bottom controls bar.
  drawButton(t(lang, 'undo'),    undoBtnRect(),    true);
  drawButton(t(lang, 'newGame'), newBtnRect(),     true);
  drawButton(t(lang, 'backToMenu'), backRect(),    true);
  // Win / over modal.
  if (state.won && !state.keepPlaying) drawModal(t(lang, 'win'), 'win');
  else if (state.over) drawModal(t(lang, 'over'), 'over');
}

function drawModal(msg, kind) {
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 100, CANVAS_W, 320);
  ctx.fillStyle = kind === 'win' ? '#f4d27b' : '#ff7a7a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(msg, CANVAS_W / 2, 200);
  ctx.fillStyle = '#fbf7ef';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'score') + ' ' + state.score, CANVAS_W / 2, 232);
  if (kind === 'win') {
    drawButton(t(lang, 'keepPlaying'), keepRect());
    drawButton(t(lang, 'newGame'), restartRect(), true);
  } else {
    drawButton(t(lang, 'newGame'), restartRect());
    drawButton(t(lang, 'backToMenu'), menuRect(), true);
  }
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 320, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(244, CANVAS_H - 36, 100, 28); }
function undoBtnRect()    { return rect(20,  CANVAS_H - 36, 100, 28); }
function newBtnRect()     { return rect(132, CANVAS_H - 36, 100, 28); }
function keepRect()       { return rect(60,  280, 100, 36); }
function restartRect()    { return rect(200, 280, 100, 36); }
function menuRect()       { return rect(130, 330, 100, 32); }
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#3a3331' : '#f0c570';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#5a4d47' : '#fff0c8';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#fbf7ef' : '#1c1717';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
