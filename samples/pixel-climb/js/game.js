// Pixel Climb - screen flow, on-screen + keyboard input, RAF, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-climb:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;
let touchHeld = {};         // map control-key -> active pointer id

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
  touchHeld = {};
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
    // On-screen control hit-test.
    const rs = controlRects();
    for (const key of Object.keys(rs)) {
      if (hit(p, rs[key])) {
        touchHeld[key] = e.pointerId;
        setInput(state, key, true);
        return;
      }
    }
  }
});
canvas.addEventListener('pointerup', (e) => {
  for (const key of Object.keys(touchHeld)) {
    if (touchHeld[key] === e.pointerId) {
      delete touchHeld[key];
      setInput(state, key, false);
    }
  }
});
canvas.addEventListener('pointercancel', (e) => {
  for (const key of Object.keys(touchHeld)) {
    if (touchHeld[key] === e.pointerId) {
      delete touchHeld[key];
      setInput(state, key, false);
    }
  }
});

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A')      setInput(state, 'left', true);
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') setInput(state, 'right', true);
  else if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')    setInput(state, 'up', true);
  else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S')  setInput(state, 'down', true);
  else if (e.key === ' ' || e.key === 'z' || e.key === 'Z')          setInput(state, 'jump', true);
});
window.addEventListener('keyup', (e) => {
  if (!state) return;
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A')      setInput(state, 'left', false);
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') setInput(state, 'right', false);
  else if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')    setInput(state, 'up', false);
  else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S')  setInput(state, 'down', false);
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
  ctx.fillStyle = '#e85a3a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 100);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  // Inline demo: render a partial tower.
  const demo = buildGame(0);
  drawTower(ctx, demo);
  drawPlayer(ctx, demo.player);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
}

function paintLevels() {
  drawBackdrop(ctx);
  ctx.fillStyle = '#e85a3a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'levelSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#2a1d3a' : '#1a1224';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#e85a3a' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText('cd ' + LEVELS[i].barrelCd.toFixed(1) + 's · sp ' + LEVELS[i].barrelSpeed, r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) {
      ctx.fillStyle = '#e85a3a';
      ctx.fillText(t(lang, 'high') + ' ' + (save.best[String(i)] || 0), r.x + r.w - 8, r.y + 34);
    } else if (!can) {
      ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
    }
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawTower(ctx, state);
  for (const b of state.barrels) drawBarrel(ctx, b, state.elapsed || 0);
  drawPlayer(ctx, state.player);
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.levelIndex)] || 0);
  drawControls(ctx, state.input);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = state.won ? '#ff7fb8' : '#ff7a7a';
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
function titleStartRect() { return rect(110, 388, 140, 38); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(8,   CANVAS_H - 18, 60, 16); }
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
  ctx.fillStyle = dim ? '#28315c' : '#e85a3a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#ff8a6a';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#1a1224';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
