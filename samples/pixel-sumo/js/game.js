// Pixel Sumo - screen flow, input, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-sumo:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let result = null;
let lastTime = 0;
let pointerId = null;

const save = loadSave();
function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return { cleared: [] };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startLevel(i) {
  state = buildGame(i);
  result = null;
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
      result = { won: !!state.won, levelIndex: state.levelIndex };
      if (result.won && !save.cleared.includes(state.levelIndex)) {
        save.cleared.push(state.levelIndex);
        persist();
      }
      screen = 'result';
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
    if (hit(p, retryRect())) { startLevel(result.levelIndex); return; }
    if (result.won && result.levelIndex + 1 < LEVEL_COUNT && hit(p, nextRect())) {
      startLevel(result.levelIndex + 1); return;
    }
    if (hit(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('levels'); return; }
    pointerId = e.pointerId;
    startAim(state, p.x, p.y);
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play' || e.pointerId !== pointerId) return;
  e.preventDefault();
  const p = relPoint(e);
  updateAim(state, p.x, p.y);
});
canvas.addEventListener('pointerup', (e) => {
  if (e.pointerId !== pointerId) return;
  pointerId = null;
  releaseAim(state);
});
canvas.addEventListener('pointercancel', () => { pointerId = null; if (state) state.aim = null; });

// ---- painting ----------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  ctx.fillStyle = PALETTE.outside;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 26px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Demo: tiny ring + two wrestlers facing off.
  ctx.fillStyle = PALETTE.ringEdge;
  ctx.beginPath(); ctx.arc(180, 240, 60, 0, Math.PI*2); ctx.fill();
  ctx.fillStyle = PALETTE.ring;
  ctx.beginPath(); ctx.arc(180, 240, 56, 0, Math.PI*2); ctx.fill();
  ctx.fillStyle = PALETTE.player;
  ctx.beginPath(); ctx.arc(155, 240, 13, 0, Math.PI*2); ctx.fill();
  ctx.fillStyle = PALETTE.ai;
  ctx.beginPath(); ctx.arc(205, 240, 13, 0, Math.PI*2); ctx.fill();
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 340);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 356);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 372);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#28315c' : '#1c2240';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.ok : (can ? '#bfc7e6' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.fillStyle = can ? '#bfc7e6' : '#5a6188';
    ctx.font = '10px monospace';
    ctx.fillText('cd ' + LEVELS[i].aiCd + 's · pow ' + LEVELS[i].aiPower, r.x + 8, r.y + 36);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(t(lang, 'cleared'), r.x + r.w - 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawScene(ctx, state, lang);
  if (!state.started) {
    ctx.fillStyle = 'rgba(247, 230, 154, 0.85)';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(t(lang, 'aim'), CANVAS_W / 2, 450);
    ctx.textBaseline = 'alphabetic';
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.65)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = result.won ? '#f7e69a' : '#e8554f';
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, result.won ? 'win' : 'lose'), CANVAS_W / 2, 170);
  drawButton(t(lang, 'retry'), retryRect());
  if (result.won && result.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 320, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  220, 100, 34); }
function nextRect()       { return rect(200, 220, 100, 34); }
function menuRect()       { return rect(130, 270, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
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
