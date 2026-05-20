// Pixel Bowl - screen flow, input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-bowl:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';            // 'title' | 'play' | 'over'
let state = null;
let lastTime = 0;
let pointerId = null;

const save = loadSave();
function loadSave() {
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { best: 0 };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startGame() {
  state = buildGame();
  screen = 'play';
  lastTime = performance.now();
  requestAnimationFrame(loop);
  paint();
}

function loop(now) {
  const dt = Math.min(0.05, (now - lastTime) / 1000);
  lastTime = now;
  if (screen === 'play' && state) {
    tick(state, dt);
    if (state.over) {
      const score = gameScore(state);
      if (score > (save.best | 0)) { save.best = score; persist(); }
      screen = 'over';
      paint();
      return;
    }
    paint();
    requestAnimationFrame(loop);
  } else {
    paint();
  }
}

// ---- input -------------------------------------------------------------
function relPoint(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) * (CANVAS_W / r.width),
    y: (e.clientY - r.top)  * (CANVAS_H / r.height),
  };
}
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { startGame(); return; }
    if (hit(p, langRect())) { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'over') {
    if (hit(p, againRect())) { startGame(); return; }
    if (hit(p, menuRect())) { setScreen('title'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    pointerId = e.pointerId;
    if (state.awaitingNext) clearWait(state);
    startAim(state, p.x, p.y);
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play' || e.pointerId !== pointerId) return;
  e.preventDefault();
  const p = relPoint(e);
  updateAim(state, p.x, p.y);
  paint();
});
canvas.addEventListener('pointerup', (e) => {
  if (e.pointerId !== pointerId) return;
  e.preventDefault();
  pointerId = null;
  releaseAim(state);
  if (!state.ball) paint();      // aim was rejected; nothing to do
});
canvas.addEventListener('pointercancel', () => { pointerId = null; state && (state.aim = null); paint(); });

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title') paintTitle();
  else if (screen === 'play')  paintPlay();
  else if (screen === 'over')  paintOver();
}

function paintTitle() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Demo lane snippet.
  ctx.fillStyle = PALETTE.lane;
  ctx.fillRect(140, 170, 80, 100);
  ctx.fillStyle = PALETTE.pinHead;
  for (let i = 0; i < 3; i++) {
    ctx.fillRect(166 + i * 14, 180, 6, 8);
  }
  ctx.fillStyle = PALETTE.ball;
  ctx.beginPath(); ctx.arc(180, 250, 7, 0, Math.PI * 2); ctx.fill();
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 360);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 376);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 392);
  ctx.fillStyle = PALETTE.hudDim;
  ctx.fillText(t(lang, 'best') + ': ' + (save.best | 0), CANVAS_W / 2, 420);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintPlay() {
  drawScene(ctx, state, CANVAS_W, CANVAS_H, lang, save.best);
  // Throw-prompt label.
  if (!state.ball && !state.aim) {
    ctx.fillStyle = 'rgba(247, 230, 154, 0.85)';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(t(lang, 'aim'), CANVAS_W / 2, 405);
    ctx.textBaseline = 'alphabetic';
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintOver() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.7)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'final'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = 'bold 36px monospace';
  ctx.fillText(String(gameScore(state)), CANVAS_W / 2, 220);
  ctx.font = '12px monospace';
  ctx.fillText(t(lang, 'best') + ': ' + (save.best | 0), CANVAS_W / 2, 250);
  drawButton(t(lang, 'again'), againRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 290, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function againRect()      { return rect(110, 290, 140, 36); }
function menuRect()       { return rect(130, 340, 100, 32); }
function hit(p, r) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#54c47c';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#86df9d';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
