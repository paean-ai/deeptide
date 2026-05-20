// Pixel Frogger - screen flow, input (tap/swipe + keyboard), save, RAF loop.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-frogger:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0;
let rafId = null;

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
  if (dt > 0.08) dt = 0.08;
  // Respawn beat.
  if (state && !state.frog.alive && !state.over) {
    state.frog.respawn = (state.frog.respawn || 0) - dt;
    if (state.frog.respawn <= 0) resetFrog(state);
  }
  // 240Hz substep for stable lane collisions.
  const sub = 1 / 240;
  let remain = dt;
  while (remain > 0) {
    const step = Math.min(sub, remain);
    tick(state, step);
    remain -= step;
    if (state.over) break;
  }
  if (state.over) {
    const key = String(state.levelIndex);
    const final = levelScore(state);
    if (state.won && !save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
    if (!save.best[key] || save.best[key] < final) save.best[key] = final;
    persist();
    screen = 'result';
  }
  paint();
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

let dragStart = null;
canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hit(p, titleStartRect())) { setScreen('levels'); return; }
    if (hit(p, langRect())) { setLang(lang === 'en' ? 'zh' : 'en'); return; }
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
    dragStart = p;
    // Tap-to-hop: pick direction by which screen edge the tap is closest to,
    // measured from the frog's current cell — works on phones without swipe.
    const fr = cellRectF(state.frog.x, state.frog.y);
    const fcx = fr.x + fr.w / 2, fcy = fr.y + fr.h / 2;
    const dx = p.x - fcx, dy = p.y - fcy;
    if (Math.abs(dx) < 24 && Math.abs(dy) < 24) return; // dead zone
    if (Math.abs(dx) > Math.abs(dy)) hop(state, dx > 0 ? 1 : -1, 0);
    else hop(state, 0, dy > 0 ? 1 : -1);
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play' || !dragStart) return;
  const p = relPoint(e);
  const dx = p.x - dragStart.x, dy = p.y - dragStart.y;
  const TH = 32;
  if (Math.max(Math.abs(dx), Math.abs(dy)) >= TH) {
    if (Math.abs(dx) > Math.abs(dy)) hop(state, dx > 0 ? 1 : -1, 0);
    else hop(state, 0, dy > 0 ? 1 : -1);
    dragStart = p;
  }
});
canvas.addEventListener('pointerup', () => { dragStart = null; });
canvas.addEventListener('pointercancel', () => { dragStart = null; });

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')    hop(state, 0, -1);
  else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S') hop(state, 0, 1);
  else if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A') hop(state, -1, 0);
  else if (e.key === 'ArrowRight'|| e.key === 'd' || e.key === 'D') hop(state, 1, 0);
});

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  ctx.fillStyle = '#0a0a0a';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#7fd84a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 96);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 122);
  // Inline demo: a single frog beside a log + car
  drawLog(ctx, 60, 200, 3);
  drawCar(ctx, 200, 200, 1, 1, 0);
  drawFrog(ctx, 50, 248, 0, false, 0);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 336);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 352);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  ctx.fillStyle = '#0a0a0a';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#7fd84a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#1c2240' : '#101630';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#a0a8b8' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText('×' + LEVELS[i].speedMul.toFixed(2) + ' · ' + LEVELS[i].timeLimit + 's', r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      ctx.fillStyle = '#f4d27b';
      ctx.fillText(t(lang, 'high') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 34);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawGoalRow(ctx, state.pads);
  for (const lane of state.lanes) drawLane(ctx, lane);
  // Frog over the playfield.
  if (state.frog.alive) {
    const r = cellRectF(state.frog.x, state.frog.y);
    drawFrog(ctx, r.x + r.w / 2, r.y + r.h / 2, state.frog.face, false, state.frog.hop / 0.18);
  }
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = state.won ? '#f4d27b' : '#ff7a7a';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(state.won ? t(lang, 'win') : t(lang, 'lose'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  const final = levelScore(state);
  ctx.fillText(t(lang, 'score') + ': ' + final, CANVAS_W / 2, 210);
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
  ctx.fillStyle = dim ? '#28315c' : '#7fd84a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#bce088';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#0a0a0a';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
