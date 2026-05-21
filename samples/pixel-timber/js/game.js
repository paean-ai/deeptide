// Pixel Timber - screen flow, tap input, RAF loop, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-timber:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { best: 0 };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function setLang(l) { lang = l; saveLang(l); paint(); }

function startGame() {
  state = buildGame(0);
  screen = 'play';
  if (rafId) cancelAnimationFrame(rafId);
  lastT = 0;
  rafId = requestAnimationFrame(loop);
}

function loop(now) {
  rafId = requestAnimationFrame(loop);
  if (screen !== 'play' && screen !== 'result') { lastT = now; return; }
  if (!lastT) lastT = now;
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  const wasOver = state.over;
  tick(state, dt);
  if (state.over && !wasOver) {
    if (state.score > save.best) { save.best = state.score; persist(); }
    screen = 'result';
  }
  paint();
}

function doChop(side) {
  if (screen !== 'play' || state.over) return;
  chop(state, side);
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

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, startRect())) { startGame(); return; }
    if (hit(p, langRect()))  { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'result') {
    if (hit(p, retryRect())) { startGame(); return; }
    if (hit(p, menuRect()))  { screen = 'title'; paint(); return; }
    return;
  }
  if (screen === 'play') {
    doChop(p.x < CANVAS_W / 2 ? SIDE_LEFT : SIDE_RIGHT);
  }
});

window.addEventListener('keydown', (e) => {
  if (screen === 'play') {
    if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') doChop(SIDE_LEFT);
    else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') doChop(SIDE_RIGHT);
  } else if (screen === 'title' && (e.key === ' ' || e.key === 'Enter')) {
    startGame();
  } else if (screen === 'result' && (e.key === ' ' || e.key === 'Enter')) {
    startGame();
  }
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  // A short demo trunk.
  const cx = CANVAS_W / 2;
  drawLog(ctx, cx, 250, BR_NONE, false);
  drawLog(ctx, cx, 250 - SEG_H, BR_RIGHT, false);
  drawLog(ctx, cx, 250 + SEG_H, BR_NONE, true);
  ctx.fillStyle = '#1d2a22';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), cx, 86);
  ctx.fillStyle = '#3a5240';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), cx, 110);
  drawButton(t(lang, 'start'), startRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#3a5240';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), cx, 392);
  ctx.fillText(t(lang, 'rulesTxt2'), cx, 408);
  ctx.fillText(t(lang, 'rulesTxt3'), cx, 424);
  ctx.fillText('© 2025-2026 a8e · MIT', cx, 462);
}

function paintPlay() {
  ctx.save();
  if (state.shake > 0) {
    const m = state.shake * 10;
    ctx.translate((Math.random() - 0.5) * m, (Math.random() - 0.5) * m);
  }
  drawBackdrop(ctx);
  drawTrunk(ctx, state);
  drawFlyLog(ctx, state);
  drawLumberjack(ctx, state);
  ctx.restore();
  drawHud(ctx, lang, state, save.best);
  if (state.score === 0 && !state.over) {
    ctx.fillStyle = '#1d2a22';
    ctx.font = '11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t(lang, 'tapToChop'), CANVAS_W / 2, 360);
  }
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.6)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.textAlign = 'center';
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 28px monospace';
  ctx.fillText(t(lang, state.hitSide < 0 ? 'timeUp' : 'gameOver'), CANVAS_W / 2, 168);
  ctx.fillStyle = '#f4f3e6';
  ctx.font = '15px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + finalScore(state), CANVAS_W / 2, 208);
  ctx.fillStyle = '#bcd0bf';
  ctx.font = '12px monospace';
  ctx.fillText(t(lang, 'best') + ': ' + save.best, CANVAS_W / 2, 232);
  drawButton(t(lang, 'retry'), retryRect());
  drawButton(t(lang, 'menu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function startRect() { return rect(110, 320, 140, 42); }
function langRect()  { return rect(285, 6,  68,  20); }
function retryRect() { return rect(70,  280, 100, 38); }
function menuRect()  { return rect(190, 280, 100, 38); }
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#3a5240' : '#d8453f';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#52704f' : '#f06a5a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#f4f3e6';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
