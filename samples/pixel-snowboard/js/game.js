// Pixel Snowboard - screen flow, drag-to-carve + keyboard, RAF, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-snowboard:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;
let keyDir = 0;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { cleared: [], best: {} };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startLevel(i) {
  state = buildGame(i);
  keyDir = 0;
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
  // Keyboard steering nudges the input column.
  if (keyDir && state && !state.over) {
    setRiderX(state, state.rider.x + keyDir * 240 * dt);
  }
  tick(state, dt);
  if (state.over) {
    const key = String(state.levelIndex);
    const final = finalScore(state);
    if (state.won && !save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    if (!save.best[key] || save.best[key] < final) save.best[key] = final;
    persist();
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

let dragging = false;
canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { setScreen('levels'); return; }
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'levels') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    const idx = hitLevelCell(p);
    if (idx >= 0 && unlocked(idx)) startLevel(idx);
    return;
  }
  if (screen === 'result') {
    if (hit(p, retryRect())) { startLevel(state.levelIndex); return; }
    if (state.won && state.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      const nxt = state.levelIndex + 1;
      if (unlocked(nxt)) startLevel(nxt); else setScreen('levels');
      return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('levels'); return; }
    dragging = true;
    setRiderX(state, p.x);
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (!dragging || screen !== 'play') return;
  setRiderX(state, relPoint(e).x);
});
canvas.addEventListener('pointerup', () => { dragging = false; });
canvas.addEventListener('pointercancel', () => { dragging = false; });

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A')      keyDir = -1;
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') keyDir = 1;
});
window.addEventListener('keyup', (e) => {
  if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' ||
      e.key === 'a' || e.key === 'd' || e.key === 'A' || e.key === 'D') keyDir = 0;
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx, null);
  ctx.fillStyle = '#1c2438';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#5a6478';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 124);
  // Inline demo: a tree, a gate, the rider.
  drawTree(ctx, 110, 230);
  drawGate(ctx, 250, 230, 70, false);
  drawRider(ctx, { rider: { x: 180, air: 0, hitFlash: 0, alive: true } });
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#5a6478';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 336);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 352);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  drawBackdrop(ctx, null);
  ctx.fillStyle = '#1c2438';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#d4dcea' : '#b8c0d0';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#e8554f' : '#8a92a4');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#1c2438' : '#7a8294';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#5a6478' : '#8a92a4';
    ctx.fillText((LEVELS[i].length / 100 | 0) * 10 + 'm · ×' + LEVELS[i].density.toFixed(1), r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      ctx.fillStyle = '#1c2438';
      ctx.fillText(t(lang, 'high') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 34);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx, state);
  drawObstacles(ctx, state);
  drawRider(ctx, state);
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.55)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = state.won ? '#5fc06e' : '#ff7a7a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(state.won ? t(lang, 'win') : t(lang, 'lose'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + finalScore(state), CANVAS_W / 2, 210);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'high') + ': ' + (save.best[String(state.levelIndex)] || 0), CANVAS_W / 2, 236);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry --------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 380, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#5a6478' : '#e8554f';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#7a8498' : '#ff8a6a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
