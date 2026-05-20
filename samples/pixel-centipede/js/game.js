// Pixel Centipede - screen flow, drag-to-move input, RAF loop, save.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-centipede:save';

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
  return { cleared: [], best: {} };
}
function persist() { try { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); } catch (_) {} }
function unlocked(i) { return i === 0 || save.cleared.includes(i - 1); }
function setLang(l) { lang = l; saveLang(l); paint(); }
function setScreen(s) { screen = s; paint(); }

function startWave(i) {
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
  if (dt > 0.05) dt = 0.05;
  // 240Hz substep for stable bullet vs centipede collisions.
  const sub = 1 / 240;
  let remain = dt;
  while (remain > 0 && !state.over) {
    const step = Math.min(sub, remain);
    tick(state, step);
    remain -= step;
  }
  if (state.over) {
    const key = String(state.waveIndex);
    const final = finalScore(state);
    if (state.won && !save.cleared.includes(state.waveIndex)) save.cleared.push(state.waveIndex);
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
    if (hit(p, titleStartRect())) { setScreen('waves'); return; }
    if (hit(p, langRect()))       { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    return;
  }
  if (screen === 'waves') {
    if (hit(p, backRect())) { setScreen('title'); return; }
    const idx = hitWaveCell(p);
    if (idx >= 0 && unlocked(idx)) startWave(idx);
    return;
  }
  if (screen === 'result') {
    if (hit(p, retryRect())) { startWave(state.waveIndex); return; }
    if (state.won && state.waveIndex + 1 < WAVE_COUNT && hit(p, nextRect())) {
      const nxt = state.waveIndex + 1;
      if (unlocked(nxt)) startWave(nxt); else setScreen('waves');
      return;
    }
    if (hit(p, menuRect())) { setScreen('waves'); return; }
    return;
  }
  if (screen === 'play') {
    if (hit(p, backRect())) { setScreen('waves'); return; }
    dragging = true;
    movePlayerTo(p);
  }
});
canvas.addEventListener('pointermove', (e) => {
  if (!dragging || screen !== 'play') return;
  movePlayerTo(relPoint(e));
});
canvas.addEventListener('pointerup', () => { dragging = false; stopPlayer(); });
canvas.addEventListener('pointercancel', () => { dragging = false; stopPlayer(); });

function movePlayerTo(p) {
  if (!state) return;
  // Convert canvas coords to grid cells; clamp by data.clampRow / clampCol.
  const tcol = (p.x - BOARD_OX) / CELL;
  const trow = (p.y - BOARD_OY) / CELL;
  // Drag-AWAY-free direct steering: head toward the touch point at most
  // 1 cell per tick by setting playerVX/playerVY proportional to the
  // offset. The tick clamps by speed * dt; we just normalise to [-1, 1].
  const dx = tcol - state.player.col;
  const dy = trow - state.player.row;
  const m = Math.hypot(dx, dy) || 1;
  state.playerVX = Math.max(-1, Math.min(1, dx / m * Math.min(1, m)));
  state.playerVY = Math.max(-1, Math.min(1, dy / m * Math.min(1, m)));
  if (Math.abs(dx) < 0.1) state.playerVX = 0;
  if (Math.abs(dy) < 0.1) state.playerVY = 0;
}
function stopPlayer() { if (state) { state.playerVX = 0; state.playerVY = 0; } }

window.addEventListener('keydown', (e) => {
  if (screen !== 'play' || !state) return;
  if (e.key === 'ArrowLeft' || e.key === 'a' || e.key === 'A')      state.playerVX = -1;
  else if (e.key === 'ArrowRight' || e.key === 'd' || e.key === 'D') state.playerVX = 1;
  else if (e.key === 'ArrowUp' || e.key === 'w' || e.key === 'W')    state.playerVY = -1;
  else if (e.key === 'ArrowDown' || e.key === 's' || e.key === 'S')  state.playerVY = 1;
});
window.addEventListener('keyup', (e) => {
  if (!state) return;
  if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'a' || e.key === 'd' || e.key === 'A' || e.key === 'D') state.playerVX = 0;
  if (e.key === 'ArrowUp' || e.key === 'ArrowDown' || e.key === 'w' || e.key === 's' || e.key === 'W' || e.key === 'S') state.playerVY = 0;
});

// ---- painting ---------------------------------------------------------
function paint() {
  if      (screen === 'title')  paintTitle();
  else if (screen === 'waves')  paintWaves();
  else if (screen === 'play')   paintPlay();
  else if (screen === 'result') paintResult();
}

function paintTitle() {
  ctx.fillStyle = '#0a0d1e';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#7fd84a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 96);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 122);
  // Inline demo: a centipede strip + mushroom
  for (let i = 0; i < 5; i++) {
    const seg = { col: 5 + i, row: 10 };
    drawSegment(ctx, seg, i === 0);
  }
  drawBullet(ctx, { col: 10, row: 12 });
  // Mushroom
  const mockMush = Array.from({length:ROWS}, () => new Array(COLS).fill(0));
  mockMush[12][13] = 3;
  drawMushrooms(ctx, mockMush);
  drawButton(t(lang, 'start'), titleStartRect());
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), true);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 320);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 336);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 352);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 460);
}

function paintWaves() {
  ctx.fillStyle = '#0a0d1e';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = '#7fd84a';
  ctx.font = 'bold 18px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'waveSelect'), CANVAS_W / 2, 36);
  for (let i = 0; i < WAVE_COUNT; i++) {
    const r = waveCellRect(i);
    const cleared = save.cleared.includes(i);
    const can = unlocked(i);
    ctx.fillStyle = can ? '#1c2240' : '#101630';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = cleared ? '#5fc06e' : (can ? '#a0a8b8' : '#5a6188');
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = can ? '#f8f5e8' : '#7a8099';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(t(lang, 'wave') + ' ' + (i + 1) + ' ' + WAVES[i].name[lang === 'zh' ? 1 : 0], r.x + 8, r.y + 18);
    ctx.font = '10px monospace';
    ctx.fillStyle = can ? '#a0a8b8' : '#5a6188';
    ctx.fillText(WAVES[i].len + ' segs · x' + WAVES[i].speed.toFixed(1), r.x + 8, r.y + 34);
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
  drawMushrooms(ctx, state.mushrooms);
  // Mark the head of each contiguous worm: a segment is a head if the
  // previous index isn't adjacent in the chain or it's index 0.
  for (let i = 0; i < state.segments.length; i++) {
    const head = i === 0
      || Math.abs(state.segments[i].col - state.segments[i-1].col) > 1
      || state.segments[i].row !== state.segments[i-1].row;
    drawSegment(ctx, state.segments[i], head);
  }
  for (const b of state.bullets) drawBullet(ctx, b);
  if (state.spider) drawSpider(ctx, state.spider);
  drawPlayer(ctx, state.player);
  drawFlash(ctx, state);
  drawHud(ctx, lang, state, save.best[String(state.waveIndex)] || 0);
  drawButton(t(lang, 'backToMenu'), backRect(), true);
}

function paintResult() {
  paintPlay();
  ctx.fillStyle = 'rgba(0,0,0,0.62)';
  ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);
  ctx.fillStyle = state.won ? '#f4d27b' : '#ff7a7a';
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(state.won ? t(lang, 'win') : t(lang, 'lose'), CANVAS_W / 2, 170);
  ctx.fillStyle = '#f8f5e8';
  ctx.font = '14px monospace';
  ctx.fillText(t(lang, 'score') + ': ' + finalScore(state), CANVAS_W / 2, 210);
  ctx.fillStyle = '#a0a8b8';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'high') + ': ' + (save.best[String(state.waveIndex)] || 0), CANVAS_W / 2, 236);
  drawButton(t(lang, 'retry'), retryRect());
  if (state.won && state.waveIndex + 1 < WAVE_COUNT) drawButton(t(lang, 'next'), nextRect());
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
function waveCellRect(i) {
  const col = i % 2, row = (i / 2) | 0;
  return rect(20 + col * 165, 60 + row * 56, 155, 48);
}
function hitWaveCell(p) {
  for (let i = 0; i < WAVE_COUNT; i++) if (hit(p, waveCellRect(i))) return i;
  return -1;
}
function drawButton(label, r, dim) {
  ctx.fillStyle = dim ? '#28315c' : '#7fd84a';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = dim ? '#3c4576' : '#bce088';
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = dim ? '#f8f5e8' : '#0a0d1e';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
