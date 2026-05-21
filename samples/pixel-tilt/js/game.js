// Pixel Tilt - screen flow, swipe / d-pad input, slide animation, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-tilt:save';
const ANIM_DUR = 0.16;
const G = {
  screen: 'title',          // title | select | play
  play: null,
  levelIndex: 0,
  anim: null,               // { from:[cells], to:[cells], t }
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
const startBtn = { x: 96, y: 320, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 422, w: 140, h: 38 };
function levelRow(i) { return { x: 36, y: 92 + i * 52, w: 288, h: 44 }; }
const cardX = 44, cardY = 124, cardW = 272, cardH = 224;
const rRetry = { x: cardX + 18, y: cardY + 150, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 150, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 150, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 194, w: 236, h: 32 };

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 52, 30, PAL.text);
  fillText(ctx, t('subtitle'), 180, 82, 10.5, PAL.dim);
  drawBtn(ctx, startBtn, t('start'), PAL.good, true);
  drawBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', PAL.star, false);
  fillText(ctx, t('rules1'), 180, 392, 9.5, PAL.dim);
  fillText(ctx, t('rules2'), 180, 410, 9.5, PAL.dim);
  fillText(ctx, t('rules3'), 180, 428, 9.5, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('levelSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < LEVEL_COUNT; i++) {
    const r = levelRow(i), open = unlocked(i), lv = LEVELS[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    px(ctx, r.x + 2, r.y + 2, 5, r.h - 4, open ? GEMS[i % GEMS.length].base : PAL.panelHi);
    fillText(ctx, (i + 1) + '.', r.x + 24, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(lv.name).toUpperCase() : t('locked'),
      r.x + 46, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    fillText(ctx, lv.n + '×' + lv.n, r.x + r.w - 104, r.y + r.h / 2, 11, PAL.dim, 'left');
    if (open && G.save.cleared[i]) drawStars(ctx, r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(ctx, backBtn, t('back'), PAL.dim, false);
}

function easeOut(x) { return 1 - (1 - x) * (1 - x); }

function renderPlay() {
  drawBackdrop(ctx);
  const s = G.play;
  drawHud(ctx, s);
  let crystalPx = null;
  if (G.anim) {
    const g = boardGeom(s.n), e = easeOut(Math.min(1, G.anim.t / ANIM_DUR));
    crystalPx = G.anim.from.map((fc, i) => {
      const a = cellXY(g, fc), b = cellXY(g, G.anim.to[i]);
      return {
        x: a.x + (b.x - a.x) * e + g.cell / 2,
        y: a.y + (b.y - a.y) * e + g.cell / 2,
      };
    });
  }
  drawBoard(ctx, s, crystalPx);
  const canAct = !s.over && !G.anim;
  drawArrowBtn(ctx, DPAD.U, 'U', canAct);
  drawArrowBtn(ctx, DPAD.D, 'D', canAct);
  drawArrowBtn(ctx, DPAD.L, 'L', canAct);
  drawArrowBtn(ctx, DPAD.R, 'R', canAct);
  drawBtn(ctx, UNDO_BTN, t('undo'), PAL.dim, s.history.length > 0 && !s.over);
  drawBtn(ctx, RESTART_BTN, t('restart'), PAL.dim, s.moves > 0);
  if (s.over && !G.anim) renderResultCard(s);
}

function renderResultCard(s) {
  ctx.globalAlpha = 0.74; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, t('win'), cx, cardY + 40, 18, PAL.good);
  drawStars(ctx, cx, cardY + 90, stars(s.moves, s.level.par), 22);
  fillText(ctx, t('moves') + ' ' + s.moves + '   ' + t('par') + ' ' + s.level.par,
    cx, cardY + 128, 12, PAL.dim);
  const last = G.levelIndex >= LEVEL_COUNT - 1;
  if (!last) {
    drawBtn(ctx, rRetry, t('retry'), PAL.dim, false);
    drawBtn(ctx, rNext, t('next'), PAL.good, true);
  } else {
    drawBtn(ctx, rRetryWide, t('retry'), PAL.good, true);
  }
  drawBtn(ctx, rMenu, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startLevel(i) {
  G.levelIndex = i;
  G.play = newPlay(i);
  G.anim = null;
  G.screen = 'play';
}
function doTilt(dir) {
  const s = G.play;
  if (s.over || G.anim) return;
  const from = s.pos.slice();
  if (!tilt(s, dir)) return;            // null tilt - nothing moved
  G.anim = { from, to: s.pos.slice(), t: 0 };
  if (s.over && s.won) {
    G.save.cleared[G.levelIndex] = true;
    const st = stars(s.moves, s.level.par);
    if (!(G.save.stars[G.levelIndex] >= st)) G.save.stars[G.levelIndex] = st;
    persist();
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
let pointerStart = null;

function onTap(p) {
  if (G.screen === 'title') {
    if (hitR(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hitR(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hitR(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < LEVEL_COUNT; i++) {
      if (hitR(levelRow(i), p) && unlocked(i)) { startLevel(i); return; }
    }
  } else if (G.screen === 'play') {
    const s = G.play;
    if (s.over && !G.anim) {
      const last = G.levelIndex >= LEVEL_COUNT - 1;
      if (hitR(rMenu, p)) { G.screen = 'select'; return; }
      if (!last) {
        if (hitR(rRetry, p)) { startLevel(G.levelIndex); return; }
        if (hitR(rNext, p)) { startLevel(G.levelIndex + 1); return; }
      } else if (hitR(rRetryWide, p)) { startLevel(G.levelIndex); return; }
      return;
    }
    if (G.anim) return;
    if (hitR(UNDO_BTN, p)) { undo(s); return; }
    if (hitR(RESTART_BTN, p)) { restart(s); return; }
    if (hitR(DPAD.U, p)) doTilt('U');
    else if (hitR(DPAD.D, p)) doTilt('D');
    else if (hitR(DPAD.L, p)) doTilt('L');
    else if (hitR(DPAD.R, p)) doTilt('R');
  }
}

canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); pointerStart = eventPos(e); });
canvas.addEventListener('pointerup', (e) => {
  e.preventDefault();
  if (!pointerStart) return;
  const up = eventPos(e);
  const dx = up.x - pointerStart.x, dy = up.y - pointerStart.y;
  const start = pointerStart;
  pointerStart = null;
  if (Math.abs(dx) > 24 || Math.abs(dy) > 24) {
    if (G.screen === 'play') {
      if (Math.abs(dx) > Math.abs(dy)) doTilt(dx > 0 ? 'R' : 'L');
      else doTilt(dy > 0 ? 'D' : 'U');
    }
  } else {
    onTap(start);
  }
});

window.addEventListener('keydown', (e) => {
  if (G.screen === 'play' && G.play) {
    const s = G.play, k = e.key;
    if (s.over) {
      if (k === 'Enter') {
        const last = G.levelIndex >= LEVEL_COUNT - 1;
        startLevel(last ? G.levelIndex : G.levelIndex + 1);
      }
      return;
    }
    if (k === 'ArrowUp' || k === 'w' || k === 'W') doTilt('U');
    else if (k === 'ArrowDown' || k === 's' || k === 'S') doTilt('D');
    else if (k === 'ArrowLeft' || k === 'a' || k === 'A') doTilt('L');
    else if (k === 'ArrowRight' || k === 'd' || k === 'D') doTilt('R');
    else if (k === 'u' || k === 'U') undo(s);
    else if (k === 'r' || k === 'R') restart(s);
    if (k.indexOf('Arrow') === 0) e.preventDefault();
  } else if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) {
    G.screen = 'select';
  }
});

// ---- loop ----------------------------------------------------------------
let last = 0;
function frame(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  if (G.anim) {
    G.anim.t += dt;
    if (G.anim.t >= ANIM_DUR) G.anim = null;
  }
  if (G.screen === 'title') renderTitle(now);
  else if (G.screen === 'select') renderSelect();
  else if (G.screen === 'play') renderPlay();
  requestAnimationFrame(frame);
}

loadLang();
loadSave();
requestAnimationFrame(frame);
