// Pixel Hop - screen flow, input handling, save state. Browser only.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-hop:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';            // 'title' | 'levels' | 'play' | 'result'
let state = null;
let result = null;
let lastTime = 0;
let tilt = 0;
let dragging = false;
let dragOrigin = null;           // tracking last pointer y so we can detect taps
let pointerId = null;

const save = loadSave();

function loadSave() {
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) return JSON.parse(raw);
  } catch (_) {}
  return { cleared: [], best: {} };
}
function persist() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {}
}

function setLang(l) {
  lang = l; saveLang(l); paint();
}

function setScreen(s) { screen = s; paint(); }

function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }

function startLevel(i) {
  state = buildGame(i);
  state.tickCount = 0;
  result = null;
  tilt = 0;
  screen = 'play';
  lastTime = performance.now();
  requestAnimationFrame(loop);
  paint();
}

function loop(now) {
  const dt = Math.min(0.05, (now - lastTime) / 1000);
  lastTime = now;
  if (screen === 'play' && state) {
    state.tickCount = (state.tickCount || 0) + 1;
    // Auto-start the first jump on the very first frame so the player begins
    // already in flight instead of staring at a still scene.
    if (!state.started) flap(state);
    tick(state, dt, tilt);
    if (state.over) {
      const won = !!state.won;
      if (won) {
        if (!save.cleared.includes(state.levelIndex)) save.cleared.push(state.levelIndex);
        const key = String(state.levelIndex);
        const sc = state.score || (state.altitude | 0);
        if (!save.best[key] || save.best[key] < sc) save.best[key] = sc;
        persist();
      }
      result = {
        won, levelIndex: state.levelIndex,
        altitude: state.altitude | 0, gems: state.gemsCollected, score: state.score | 0,
      };
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
  const x = (e.clientX - r.left) * (CANVAS_W / r.width);
  const y = (e.clientY - r.top)  * (CANVAS_H / r.height);
  return { x, y };
}

canvas.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  const p = relPoint(e);
  if (screen === 'title') {
    if (hitButton(p, titleStartRect())) { setScreen('levels'); return; }
    if (hitButton(p, langRect())) { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'levels') {
    if (hitButton(p, backRect())) { setScreen('title'); return; }
    const idx = hitLevelCell(p);
    if (idx >= 0 && unlocked(idx)) startLevel(idx);
    return;
  }
  if (screen === 'result') {
    if (hitButton(p, retryRect())) { startLevel(result.levelIndex); return; }
    if (result.won && hitButton(p, nextRect())) {
      const next = result.levelIndex + 1;
      if (next < LEVEL_COUNT && unlocked(next)) startLevel(next);
      else setScreen('levels');
      return;
    }
    if (hitButton(p, menuRect())) { setScreen('levels'); return; }
    return;
  }
  // play screen: tap = also flap; drag = tilt
  pointerId = e.pointerId;
  dragging = true;
  dragOrigin = p;
  applyTiltFromPoint(p);
  if (state && !state.started) flap(state);
  canvas.setPointerCapture(e.pointerId);
});

canvas.addEventListener('pointermove', (e) => {
  if (!dragging || e.pointerId !== pointerId) return;
  e.preventDefault();
  applyTiltFromPoint(relPoint(e));
});

canvas.addEventListener('pointerup', (e) => {
  if (e.pointerId !== pointerId) return;
  e.preventDefault();
  dragging = false;
  pointerId = null;
  tilt = 0;
});
canvas.addEventListener('pointercancel', () => { dragging = false; tilt = 0; pointerId = null; });

function applyTiltFromPoint(p) {
  if (!state) return;
  // Steer relative to the player's screen x. The further away the touch, the
  // stronger the tilt. Clamp to [-1, 1].
  const px = state.player.x;
  let dx = p.x - px;
  if (dx >  CANVAS_W / 2) dx -= CANVAS_W;
  if (dx < -CANVAS_W / 2) dx += CANVAS_W;
  tilt = Math.max(-1, Math.min(1, dx / 60));
}

// Keyboard fallback for desktop users.
window.addEventListener('keydown', (e) => {
  if (screen === 'play') {
    if (e.key === 'ArrowLeft' || e.key === 'a') tilt = -1;
    else if (e.key === 'ArrowRight' || e.key === 'd') tilt = 1;
    else if (e.key === ' ' && state && !state.started) flap(state);
  }
});
window.addEventListener('keyup', (e) => {
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'ArrowRight' || e.key === 'd') tilt = 0;
});

// ---- painting ----------------------------------------------------------
function paint() {
  if (screen === 'title')      paintTitle();
  else if (screen === 'levels') paintLevels();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  ctx.fillStyle = '#0c1230';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  // Decorative tiny stars.
  ctx.fillStyle = '#e8eaff';
  for (let i = 0; i < 40; i++) {
    ctx.fillRect((i * 41) % CANVAS_W, (i * 73 + 17) % CANVAS_H, 1, 1);
  }
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 28px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 110);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '12px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 140);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  // Mini hop animation cue.
  ctx.fillStyle = '#54c47c';
  ctx.fillRect(160, 320, 40, 6);
  ctx.fillStyle = '#ffd86a';
  ctx.fillRect(174, 305, 12, 12);
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'instructions').slice(0, 60), CANVAS_W / 2, 380);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintLevels() {
  ctx.fillStyle = '#0c1230';
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
    ctx.fillStyle = cleared ? '#54c47c' : (can ? '#bfc7e6' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`L${i + 1} ${LEVELS[i].name[lang === 'zh' ? 1 : 0]}`, r.x + 8, r.y + 18);
    ctx.fillStyle = can ? '#bfc7e6' : '#5a6188';
    ctx.font = '10px monospace';
    ctx.fillText(`${t(lang, 'target')}: ${LEVELS[i].target}m`, r.x + 8, r.y + 34);
    ctx.textAlign = 'right';
    if (cleared) ctx.fillText(`${t(lang, 'cleared')} · ${save.best[String(i)] || 0}`, r.x + r.w - 8, r.y + 34);
    else if (!can) ctx.fillText(t(lang, 'locked'), r.x + r.w - 8, r.y + 34);
  }
  drawButton(t(lang, 'backToMenu'), backRect(), true);
  // Legend
  ctx.fillStyle = '#bfc7e6';
  ctx.font = '10px monospace';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'legend') + ':', 12, CANVAS_H - 64);
  drawLegendIcon(ctx, 12,  CANVAS_H - 52, 'static'); ctx.fillText(t(lang, 'legStatic'), 42, CANVAS_H - 46);
  drawLegendIcon(ctx, 12,  CANVAS_H - 40, 'mover');  ctx.fillText(t(lang, 'legMover'),  42, CANVAS_H - 34);
  drawLegendIcon(ctx, 200, CANVAS_H - 52, 'spring'); ctx.fillText(t(lang, 'legSpring'), 230, CANVAS_H - 46);
  drawLegendIcon(ctx, 200, CANVAS_H - 40, 'cloud');  ctx.fillText(t(lang, 'legCloud'),  230, CANVAS_H - 34);
  drawLegendIcon(ctx, 200, CANVAS_H - 28, 'gem');    ctx.fillText(t(lang, 'legGem'),    230, CANVAS_H - 22);
}

function paintPlay() {
  if (!state) return;
  drawScene(ctx, state, CANVAS_W, CANVAS_H);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.6)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#f7e69a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, result.won ? 'win' : 'lose'), CANVAS_W / 2, 160);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '12px monospace';
  ctx.fillText(`${t(lang, 'altitude')}: ${result.altitude}m`, CANVAS_W / 2, 200);
  ctx.fillText(`${t(lang, 'gems')}: ${result.gems}`, CANVAS_W / 2, 220);
  if (result.won) ctx.fillText(`${t(lang, 'score')}: ${result.score}`, CANVAS_W / 2, 240);
  drawButton(t(lang, 'retry'), retryRect());
  if (result.won && result.levelIndex + 1 < LEVEL_COUNT) drawButton(t(lang, 'next'), nextRect());
  drawButton(t(lang, 'backToMenu'), menuRect(), true);
}

// ---- button geometry ---------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function titleStartRect() { return rect(110, 220, 140, 40); }
function langRect()       { return rect(285, 8,   65,  20); }
function backRect()       { return rect(12,  CANVAS_H - 24, 60, 18); }
function retryRect()      { return rect(60,  330, 100, 36); }
function nextRect()       { return rect(200, 330, 100, 36); }
function menuRect()       { return rect(130, 380, 100, 32); }
function levelCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 46);
}
function hitButton(p, r) {
  return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h;
}
function hitLevelCell(p) {
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelCellRect(i);
    if (hitButton(p, r)) return i;
  }
  return -1;
}

function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#54c47c';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#86df9d';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = '#0c1230';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = '#f8f5e8';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
}

paint();
