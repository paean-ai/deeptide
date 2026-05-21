// Pixel Leap - screen flow, control pad, RAF loop, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-leap:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null;

let keyL = false, keyR = false, keyJump = false;
const padPointers = {};               // pointerId -> 'L' | 'R' | 'J' | 'D'

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
  resetHero(state);
  for (const k in padPointers) delete padPointers[k];
  keyL = keyR = keyJump = false;
  screen = 'play';
  if (rafId) cancelAnimationFrame(rafId);
  lastT = 0;
  rafId = requestAnimationFrame(loop);
}

function padHas(btn) {
  for (const k in padPointers) if (padPointers[k] === btn) return true;
  return false;
}

function loop(now) {
  rafId = requestAnimationFrame(loop);
  if (screen !== 'play') { lastT = now; return; }
  if (!lastT) lastT = now;
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  const left = keyL || padHas('L'), right = keyR || padHas('R');
  setMove(state, (right ? 1 : 0) - (left ? 1 : 0));
  state.jumpHeld = keyJump || padHas('J');
  tick(state, dt);
  if (state.over) {
    if (state.won) {
      if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
      const key = String(state.levelIndex);
      const got = state.gems.filter(g => g.got).length;
      const score = got * 100 - state.deaths * 5;
      if (save.best[key] === undefined || save.best[key] < score) save.best[key] = score;
      persist();
    }
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
  if (hit(p, jumpRect()))  return 'J';
  if (hit(p, dashRect()))  return 'D';
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
    if (b) {
      padPointers[e.pointerId] = b;
      if (b === 'J') jump(state);
      else if (b === 'D') dash(state);
    }
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (screen !== 'play' || !(e.pointerId in padPointers)) return;
  const b = padButtonAt(relPoint(e));
  if (b === 'L' || b === 'R') padPointers[e.pointerId] = b;
});
function releasePointer(e) { delete padPointers[e.pointerId]; }
canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);

window.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') keyL = true;
  if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') keyR = true;
  if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W' || e.key === ' ') {
    if (screen === 'play' && !keyJump) jump(state);
    keyJump = true;
    if (screen === 'play') e.preventDefault();
  }
  if ((e.key === 'Shift' || e.key === 'x' || e.key === 'X') && screen === 'play') dash(state);
});
window.addEventListener('keyup', (e) => {
  if (e.key === 'ArrowLeft'  || e.key === 'a' || e.key === 'A') keyL = false;
  if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') keyR = false;
  if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W' || e.key === ' ') keyJump = false;
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
  ctx.font = 'bold 24px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 104);
  ctx.fillStyle = '#aab2cc';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 128);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#aab2cc';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 372);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 388);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 404);
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
    ctx.fillStyle = can ? '#272d48' : '#1a1d2c';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? PALETTE.good : (can ? PALETTE.accent : '#3a3d4c');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? PALETTE.hudText : '#5b6072';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('L' + (i + 1) + ' ' + LEVELS[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 20);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#aab2cc' : '#5b6072';
    if (cleared) ctx.fillText(t(lang, 'cleared'), r.x + 8, r.y + 36);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + 8, r.y + 36);
    else ctx.fillText(t(lang, 'play'), r.x + 8, r.y + 36);
  }
  drawButton(t(lang, 'backToMenu'), lvBackRect(), true);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawLevel(ctx, state);
  drawHud(ctx, lang, state);
  drawPad();
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.66)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = PALETTE.good;
  ctx.font = 'bold 23px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'win'), CANVAS_W / 2, 176);
  ctx.fillStyle = '#f3f1e6';
  ctx.font = '13px monospace';
  const got = state.gems.filter(g => g.got).length;
  ctx.fillText(t(lang, 'deaths') + ': ' + state.deaths + '   ◆ ' + got + '/' + state.gems.length,
               CANVAS_W / 2, 212);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

function drawPad() {
  drawPadBtn('◀', leftRect(),  padHas('L') || keyL);
  drawPadBtn('▶', rightRect(), padHas('R') || keyR);
  drawPadBtn('▲', jumpRect(),  padHas('J') || keyJump);
  drawPadBtn('»', dashRect(),  state && state.hero && state.hero.dashT > 0);
  drawPadBtn('≡', quitRect(),  false);
}
function drawPadBtn(label, r, active) {
  ctx.fillStyle = active ? '#46b8e8' : '#272d48';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = active ? '#9be0ff' : '#3a4264';
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
function langRect()       { return rect(285, 8,   65,  20); }
function lvBackRect()     { return rect(130, 432, 100, 30); }
function retryRect()      { return rect(60,  300, 100, 36); }
function nextRect()       { return rect(200, 300, 100, 36); }
function menuRect()       { return rect(130, 350, 100, 32); }
function leftRect()       { return rect(10,  404, 66,  68); }
function rightRect()      { return rect(82,  404, 66,  68); }
function quitRect()       { return rect(160, 404, 40,  68); }
function jumpRect()       { return rect(212, 404, 66,  68); }
function dashRect()       { return rect(284, 404, 66,  68); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 58, 155, 50);
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) if (hit(p, levelCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#272d48' : '#46b8e8';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3a4264' : '#9be0ff';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f3f1e6' : '#0e1018';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
