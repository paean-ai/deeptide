// Pixel Aegis - screen flow, real-time loop, drag-to-aim input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-aegis:save';
const G = {
  screen: 'title',          // title | select | play
  game: null,
  stageIndex: 0,
  overDelay: 0,
  pointerDown: false,
  save: { cleared: [], stars: [] },
};

function loadSave() {
  try {
    const o = JSON.parse(localStorage.getItem(SAVE_KEY) || '{}');
    if (Array.isArray(o.cleared)) G.save.cleared = o.cleared;
    if (Array.isArray(o.stars)) G.save.stars = o.stars;
  } catch (e) { /* fresh save */ }
}
function persist() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(G.save)); } catch (e) { /* ignore */ }
}
function unlocked(i) { return i === 0 || G.save.cleared[i - 1] === true; }
function hitR(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// ---- geometry ------------------------------------------------------------
const startBtn = { x: 96, y: 322, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 424, w: 140, h: 38 };
function stageRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }
const cardX = 44, cardY = 132, cardW = 272, cardH = 214;
const rRetry = { x: cardX + 18, y: cardY + 140, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 140, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 140, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 184, w: 236, h: 30 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 56, 28, PAL.text);
  fillText(ctx, t('subtitle'), 180, 86, 10, PAL.dim);
  drawBtn(ctx, startBtn, t('start'), PAL.good, true);
  drawBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', PAL.star, false);
  fillText(ctx, t('rules1'), 180, 392, 9, PAL.dim);
  fillText(ctx, t('rules2'), 180, 410, 9, PAL.dim);
  fillText(ctx, t('rules3'), 180, 428, 9, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('stageSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < STAGE_COUNT; i++) {
    const r = stageRow(i), open = unlocked(i), st = STAGES[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    fillText(ctx, (i + 1) + '.', r.x + 22, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(st.name).toUpperCase() : t('locked'),
      r.x + 44, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, st.shooters.length + ' foes', r.x + r.w - 104, r.y + r.h / 2, 10, PAL.dim, 'left');
    if (open && G.save.cleared[i]) drawStars(ctx, r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function renderPlay(now) {
  const s = G.game;
  drawArena(ctx, s, now);
  drawHud(ctx, s);
  drawPulseBtn(ctx, s);
  if (s.over && G.overDelay <= 0) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.76; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, s.won ? t('win') : t('lose'), cx, cardY + 36, 18, s.won ? PAL.good : PAL.bad);
  const last = G.stageIndex >= STAGE_COUNT - 1;
  if (s.won) {
    drawStars(ctx, cx, cardY + 80, stars(s), 20);
    fillText(ctx, t('core') + ' ' + s.coreHp + ' / ' + CORE_HP, cx, cardY + 116, 12, PAL.dim);
  } else {
    fillText(ctx, s.shooters.length + ' shooters still stood', cx, cardY + 86, 11, PAL.dim);
  }
  if (s.won && !last) {
    drawBtn(ctx, rRetry, t('retry'), PAL.dim, false);
    drawBtn(ctx, rNext, t('next'), PAL.good, true);
  } else {
    drawBtn(ctx, rRetryWide, t('retry'), PAL.good, true);
  }
  drawBtn(ctx, rMenu, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startStage(i) {
  G.stageIndex = i;
  G.game = newGame(i);
  G.overDelay = 0;
  G.screen = 'play';
}
function finishStage() {
  const s = G.game;
  if (s.won) {
    G.save.cleared[G.stageIndex] = true;
    const st = stars(s);
    if (!(G.save.stars[G.stageIndex] >= st)) G.save.stars[G.stageIndex] = st;
    persist();
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
function aimAt(p) {
  if (G.screen !== 'play' || !G.game || G.game.over) return;
  setAim(G.game, Math.atan2(p.y - CORE_Y, p.x - CORE_X));
}
function onTap(p) {
  if (G.screen === 'title') {
    if (hitR(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hitR(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hitR(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < STAGE_COUNT; i++) {
      if (hitR(stageRow(i), p) && unlocked(i)) { startStage(i); return; }
    }
  } else if (G.screen === 'play') {
    const s = G.game;
    if (s.over && G.overDelay <= 0) {
      const last = G.stageIndex >= STAGE_COUNT - 1;
      if (hitR(rMenu, p)) { G.screen = 'select'; return; }
      if (s.won && !last) {
        if (hitR(rRetry, p)) { startStage(G.stageIndex); return; }
        if (hitR(rNext, p)) { startStage(G.stageIndex + 1); return; }
      } else if (hitR(rRetryWide, p)) { startStage(G.stageIndex); return; }
      return;
    }
    if (!s.over) {
      if (hitR(PULSE_BTN, p)) { pulse(s); return; }
      aimAt(p);
    }
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); G.pointerDown = true; onTap(eventPos(e)); });
canvas.addEventListener('pointermove', (e) => {
  e.preventDefault();
  if (G.pointerDown && G.screen === 'play') {
    const p = eventPos(e);
    if (!hitR(PULSE_BTN, p)) aimAt(p);
  }
});
canvas.addEventListener('pointerup', (e) => { e.preventDefault(); G.pointerDown = false; });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.game && !G.game.over && (e.key === ' ')) { pulse(G.game); e.preventDefault(); }
  else if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) G.screen = 'select';
  else if (G.screen === 'play' && G.game && G.game.over && G.overDelay <= 0 && e.key === 'Enter') {
    const last = G.stageIndex >= STAGE_COUNT - 1;
    startStage(G.game.won && !last ? G.stageIndex + 1 : G.stageIndex);
  }
});

// ---- loop ----------------------------------------------------------------
let last = 0;
function frame(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  if (G.screen === 'play') {
    const s = G.game;
    if (!s.over) tick(s, dt);
    else {
      if (G.overDelay === 0) { G.overDelay = 0.8; finishStage(); }
      else G.overDelay = Math.max(0, G.overDelay - dt);
      tick(s, dt);
    }
    renderPlay(now);
  } else if (G.screen === 'title') renderTitle(now);
  else renderSelect();
  requestAnimationFrame(frame);
}

loadLang();
loadSave();
requestAnimationFrame(frame);
