// Pixel Barkeep - screen flow, tap/key input, RAF loop, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-barkeep:save';

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
  if (screen !== 'play') { lastT = now; return; }
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
    const lane = Math.floor((p.y - LANE_TOP) / LANE_H);
    if (lane >= 0 && lane < LANES) serveLane(state, lane);
  }
});

window.addEventListener('keydown', (e) => {
  if (screen === 'play') {
    if (e.key === 'ArrowUp'   || e.key === 'w' || e.key === 'W') moveTo(state, state.barkeepLane - 1);
    else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S') moveTo(state, state.barkeepLane + 1);
    else if (e.key === ' ') { e.preventDefault(); pour(state); }
  } else if (e.key === ' ' || e.key === 'Enter') {
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
  drawTitleArt(ctx, CANVAS_W / 2, 250);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#c8bfa8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 124);
  drawButton(t(lang, 'start'), startRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#c8bfa8';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 366);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 382);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 398);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 462);
}

function paintPlay() {
  drawScene(ctx, lang, state, save.best);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.64)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.textAlign = 'center';
  ctx.fillStyle = '#ff9a7a';
  ctx.font = 'bold 25px monospace';
  ctx.fillText(t(lang, 'gameOver'), CANVAS_W / 2, 168);
  ctx.fillStyle = '#f3eee0';
  ctx.font = '15px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + finalScore(state), CANVAS_W / 2, 208);
  ctx.fillStyle = '#c8bfa8';
  ctx.font = '12px monospace';
  ctx.fillText(t(lang, 'round') + ': ' + state.round + '   ' + t(lang, 'served') + ': ' + state.served,
               CANVAS_W / 2, 230);
  ctx.fillText(t(lang, 'best') + ': ' + save.best, CANVAS_W / 2, 248);
  drawButton(t(lang, 'retry'), retryRect());
  drawButton(t(lang, 'menu'), menuRect(), true);
}

// ---- geometry ---------------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function startRect()  { return rect(110, 312, 140, 42); }
function langRect()   { return rect(285, 6,  68,  20); }
function retryRect()  { return rect(70,  286, 100, 38); }
function menuRect()   { return rect(190, 286, 100, 38); }
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#3c3142' : '#f4c44a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#52465c' : '#ffe08a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3eee0' : '#15101a';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
