// Pixel Harpoon - screen flow, control pad, RAF loop, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-harpoon:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;

// Input: keyboard flags + a map of active pad pointers.
let keyL = false, keyR = false;
const padPointers = {};         // pointerId -> 'L' | 'R' | 'F'

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
  for (const k in padPointers) delete padPointers[k];
  keyL = keyR = false;
  screen = 'play';
  if (rafId) cancelAnimationFrame(rafId);
  lastT = 0;
  rafId = requestAnimationFrame(loop);
}

function padDir() {
  let d = 0;
  for (const k in padPointers) {
    if (padPointers[k] === 'L') d--;
    else if (padPointers[k] === 'R') d++;
  }
  return d;
}

function loop(now) {
  rafId = requestAnimationFrame(loop);
  if (screen !== 'play') { lastT = now; return; }
  if (!lastT) lastT = now;
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  const keyDir = (keyR ? 1 : 0) - (keyL ? 1 : 0);
  setMove(state, keyDir !== 0 ? keyDir : padDir());
  tick(state, dt);
  if (state.over) {
    const key = String(state.levelIndex);
    const fs = finalScore(state);
    if (state.won && !save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    if (!save.best[key] || save.best[key] < fs) save.best[key] = fs;
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

function padButtonAt(p) {
  if (hit(p, leftRect()))  return 'L';
  if (hit(p, rightRect())) return 'R';
  if (hit(p, fireRect()))  return 'F';
  return null;
}

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { setScreen('levels'); return; }
    if (hit(p, langRect())) { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'levels') {
    if (hit(p, lvBackRect())) { setScreen('title'); return; }
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
    if (hit(p, quitRect())) { setScreen('levels'); return; }
    const b = padButtonAt(p);
    if (b === 'F') { fire(state); }
    else if (b) { padPointers[e.pointerId] = b; }
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play') return;
  if (!(e.pointerId in padPointers)) return;
  const b = padButtonAt(relPoint(e));
  if (b === 'L' || b === 'R') padPointers[e.pointerId] = b;
});
function releasePointer(e) { delete padPointers[e.pointerId]; }
canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);

window.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') keyL = true;
  if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') keyR = true;
  if ((e.key === ' ' || e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W') && screen === 'play') {
    e.preventDefault(); fire(state);
  }
});
window.addEventListener('keyup', (e) => {
  if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') keyL = false;
  if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') keyR = false;
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  drawBackdrop(ctx);
  drawTitleArt(ctx, CANVAS_W / 2, 250);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 110);
  ctx.fillStyle = '#b8bcd8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 134);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#b8bcd8';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 392);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 408);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 424);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 462);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = PALETTE.accent;
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 40);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#2a2444' : '#1a1730';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.win : (can ? PALETTE.accent : '#444');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#6b6886';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#b8bcd8' : '#6b6886';
    ctx.fillText(LEVELS[i].orbs.length + ' orbs', r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'best') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), lvBackRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  for (const b of state.balls) drawOrb(ctx, b);
  drawHarpoon(ctx, state);
  drawPlayer(ctx, state);
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawPad();
  drawPadButton('≡', quitRect(), false);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.textAlign = 'center';
  ctx.fillStyle = state.won ? PALETTE.win : '#ff7a7a';
  ctx.font = 'bold 24px monospace';
  ctx.fillText(t(lang, state.won ? 'win' : 'lose'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + finalScore(state), CANVAS_W / 2, 210);
  ctx.fillStyle = '#a8acc8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'best') + ': ' + (save.best[String(state.levelIndex)] || 0), CANVAS_W / 2, 234);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

function drawPad() {
  const d = padDir(), keyDir = (keyR ? 1 : 0) - (keyL ? 1 : 0);
  const eff = keyDir !== 0 ? keyDir : d;
  drawPadButton('◀', leftRect(),  eff < 0);
  drawPadButton('▶', rightRect(), eff > 0);
  drawPadButton('▲', fireRect(),  !!state.harpoon);
}
function drawPadButton(label, r, active) {
  ctx.fillStyle = active ? '#46c2b6' : '#2a2444';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = active ? '#7fe3d8' : '#3e3866';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#f3f1e6';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2 + 1);
  ctx.textBaseline = 'alphabetic';
}

// ---- geometry ---------------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 300, 140, 42); }
function langRect()       { return rect(286, 8,   66,  20); }
function lvBackRect()     { return rect(130, 432, 100, 30); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 348, 100, 32); }
function leftRect()       { return rect(12,  430, 68,  44); }
function rightRect()      { return rect(86,  430, 68,  44); }
function quitRect()       { return rect(160, 430, 38,  44); }
function fireRect()       { return rect(204, 430, 144, 44); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 64 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#2a2444' : '#46c2b6';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3e3866' : '#7fe3d8';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#0e0b1c';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
