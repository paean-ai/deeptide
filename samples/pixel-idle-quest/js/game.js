// Pixel Idle Quest - screen flow, RAF loop, save with offline catch-up.

const CANVAS_W = 360, CANVAS_H = 480;
const SAVE_KEY = 'pixel-idle-quest:save';

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

let lang = (typeof loadLang === 'function') ? loadLang() : 'en';
let screen = 'title';
let state = null;
let lastT = 0, rafId = null, saveT = 0, bannerT = 0, offlineGold = 0;

function loadSave() {
  try { const raw = localStorage.getItem(SAVE_KEY); if (raw) return JSON.parse(raw); } catch (_) {}
  return null;
}
function persist() {
  try {
    localStorage.setItem(SAVE_KEY, JSON.stringify({
      game: {
        gold: state.gold, stage: state.stage, kills: state.kills,
        blade: state.blade, squire: state.squire, relics: state.relics,
        bestStage: state.bestStage, lifetimeBest: state.lifetimeBest,
      },
      time: Date.now(),
    }));
  } catch (_) {}
}
function setLang(l) { lang = l; saveLang(l); paint(); }

// Restore the save and grant the gold the squires ground out while away.
{
  const saved = loadSave();
  state = buildGame(saved ? saved.game : null);
  if (saved && saved.time) {
    offlineGold = offlineEarnings(state, (Date.now() - saved.time) / 1000);
    if (offlineGold > 0) state.gold += offlineGold;
  }
}

function startGame() {
  screen = 'play';
  if (offlineGold > 0) bannerT = 6;
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
  if (dt > 0.25) dt = 0.25;
  tick(state, dt);
  if (bannerT > 0) bannerT = Math.max(0, bannerT - dt);
  saveT += dt;
  if (saveT >= 3) { saveT = 0; persist(); }
  paint();
}
window.addEventListener('pagehide', () => { if (state) persist(); });

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
  if (screen === 'play') {
    if (hit(p, bladeRect()))  { buyBlade(state); persist(); paint(); return; }
    if (hit(p, squireRect())) { buySquire(state); persist(); paint(); return; }
    if (hit(p, ascendRect())) { if (ascend(state)) { persist(); paint(); } return; }
    if (hit(p, langRect2()))  { setLang(lang === 'en' ? 'zh' : 'en'); return; }
    if (p.y >= 44 && p.y <= 244) { tap(state); paint(); }   // strike the monster
  }
});

// ---- painting ---------------------------------------------------------
function paint() {
  if (screen === 'title') paintTitle();
  else                    paintPlay();
}

function paintTitle() {
  drawBackdrop(ctx);
  drawTitleArt(ctx, CANVAS_W / 2, 250);
  ctx.fillStyle = PALETTE.gold;
  ctx.font = 'bold 22px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(t(lang, 'title'), CANVAS_W / 2, 110);
  ctx.fillStyle = '#bcb6ce';
  ctx.font = '11px monospace';
  ctx.fillText(t(lang, 'subtitle'), CANVAS_W / 2, 134);
  drawButton(t(lang, 'start'), startRect(), '#46b8e8', '#9be0ff');
  drawButton(lang === 'en' ? '中文' : 'English', langRect(), PALETTE.btnOff, '#52507a');
  ctx.fillStyle = '#bcb6ce';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'rulesTxt1'), CANVAS_W / 2, 388);
  ctx.fillText(t(lang, 'rulesTxt2'), CANVAS_W / 2, 404);
  ctx.fillText(t(lang, 'rulesTxt3'), CANVAS_W / 2, 420);
  ctx.fillText('© 2025-2026 a8e · MIT', CANVAS_W / 2, 462);
}

function paintPlay() {
  drawBackdrop(ctx);
  drawArena(ctx, lang, state);
  drawHud(ctx, lang, state);
  // Stats strip.
  ctx.fillStyle = '#9a93b0';
  ctx.font = '11px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(t(lang, 'tapDmg') + ' ' + fmt(tapDamage(state)) + '    ' +
               t(lang, 'dps') + ' ' + fmt(autoDps(state)), CANVAS_W / 2, 262);
  // Upgrade buttons.
  drawUpgrade(t(lang, 'blade'),  t(lang, 'bladeDesc'),  state.blade,  bladeCost(state),  bladeRect());
  drawUpgrade(t(lang, 'squire'), t(lang, 'squireDesc'), state.squire, squireCost(state), squireRect());
  // Ascend.
  const ar = ascendRect();
  const can = canAscend(state);
  ctx.fillStyle = can ? PALETTE.ascend : PALETTE.btnOff;
  ctx.fillRect(ar.x, ar.y, ar.w, ar.h);
  ctx.fillStyle = can ? '#c79be8' : '#52507a';
  ctx.fillRect(ar.x, ar.y, ar.w, 2);
  ctx.fillStyle = can ? '#fff' : '#7a7596';
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(can ? (t(lang, 'ascend') + '  +' + relicsFor(state.bestStage) + ' ◆')
                   : t(lang, 'ascendLocked'), ar.x + ar.w / 2, ar.y + ar.h / 2);
  // Lang toggle.
  drawButton(lang === 'en' ? '中文' : 'EN', langRect2(), PALETTE.btnOff, '#52507a');
  // Offline banner.
  if (bannerT > 0) {
    ctx.fillStyle = 'rgba(12,10,20,0.92)';
    ctx.fillRect(40, 250, 280, 26);
    ctx.fillStyle = PALETTE.gold;
    ctx.font = 'bold 11px monospace';
    ctx.fillText(t(lang, 'offline') + ' ' + fmt(offlineGold) + ' G', CANVAS_W / 2, 263);
  }
}

function drawUpgrade(name, desc, level, cost, r) {
  const can = state.gold >= cost;
  ctx.fillStyle = can ? PALETTE.panel : '#221f30';
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = can ? PALETTE.btn : PALETTE.btnOff;
  ctx.fillRect(r.x, r.y, 4, r.h);
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillStyle = can ? PALETTE.hudText : '#7a7596';
  ctx.font = 'bold 13px monospace';
  ctx.fillText(name + '  Lv ' + level, r.x + 12, r.y + 15);
  ctx.fillStyle = '#9a93b0';
  ctx.font = '10px monospace';
  ctx.fillText(desc, r.x + 12, r.y + 32);
  ctx.textAlign = 'right';
  ctx.fillStyle = can ? PALETTE.gold : '#6a6480';
  ctx.font = 'bold 12px monospace';
  ctx.fillText(fmt(cost) + ' G', r.x + r.w - 12, r.y + r.h / 2);
}

// ---- geometry ---------------------------------------------------------
function rect(x, y, w, h) { return { x, y, w, h }; }
function startRect()  { return rect(110, 300, 140, 42); }
function langRect()   { return rect(285, 8,  66,  20); }
function langRect2()  { return rect(316, 444, 38, 24); }
function bladeRect()  { return rect(16, 280, 328, 46); }
function squireRect() { return rect(16, 332, 328, 46); }
function ascendRect() { return rect(16, 388, 328, 40); }
function drawButton(label, r, col, hi) {
  ctx.fillStyle = col;
  ctx.fillRect(r.x, r.y, r.w, r.h);
  ctx.fillStyle = hi;
  ctx.fillRect(r.x, r.y, r.w, 2);
  ctx.fillStyle = col === PALETTE.btnOff ? '#f3f1e6' : '#0e0b16';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, r.x + r.w / 2, r.y + r.h / 2);
  ctx.textBaseline = 'alphabetic';
}

paint();
