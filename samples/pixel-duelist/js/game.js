// Pixel Duelist - screen flow, real-time loop, input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-duelist:save';
const G = {
  screen: 'title',          // title | select | duel | result
  duel: null,
  bossIndex: 0,
  overDelay: 0,
  save: { cleared: [], stars: [] },
};

function loadSave() {
  try {
    const raw = localStorage.getItem(SAVE_KEY);
    if (raw) {
      const o = JSON.parse(raw);
      if (Array.isArray(o.cleared)) G.save.cleared = o.cleared;
      if (Array.isArray(o.stars)) G.save.stars = o.stars;
    }
  } catch (e) { /* fresh save */ }
}
function persist() {
  try { localStorage.setItem(SAVE_KEY, JSON.stringify(G.save)); } catch (e) { /* ignore */ }
}
function unlocked(i) { return i === 0 || G.save.cleared[i - 1] === true; }

// ---- buttons -------------------------------------------------------------
function drawBtn(r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 70 ? 17 : 13,
    active ? PAL.ink : color);
}
function hit(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

const startBtn = { x: 96, y: 322, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 418, w: 140, h: 40 };
const r1Btn = { x: 40, y: 300, w: 132, h: 44 };
const r2Btn = { x: 188, y: 300, w: 132, h: 44 };
const r3Btn = { x: 110, y: 356, w: 140, h: 44 };

function bossRow(i) { return { x: 40, y: 96 + i * 50, w: 280, h: 42 }; }

// ---- stars ---------------------------------------------------------------
function drawStars(cx, cy, n, size) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * (size + 6);
    const on = i < n;
    px(ctx, x - size / 2, cy - size / 2, size, size, on ? PAL.perfect : PAL.panel);
    px(ctx, x - 2, cy - size / 2 - 3, 4, 3, on ? PAL.perfect : PAL.panel);
    px(ctx, x - 2, cy + size / 2, 4, 3, on ? PAL.perfect : PAL.panel);
  }
}

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 60, 30, PAL.text);
  fillText(ctx, t('subtitle'), 180, 90, 12, PAL.dim);
  drawBtn(startBtn, t('start'), PAL.good, true);
  drawBtn(langBtn, lang === 'en' ? '中文' : 'EN', PAL.slash, false);
  fillText(ctx, t('rules1'), 180, 392, 9.5, PAL.slash);
  fillText(ctx, t('rules2'), 180, 410, 9.5, PAL.thrust);
  fillText(ctx, t('rules3'), 180, 432, 9.5, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('bossSelect'), 180, 56, 18, PAL.text);
  for (let i = 0; i < BOSS_COUNT; i++) {
    const r = bossRow(i), open = unlocked(i), cfg = BOSSES[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    px(ctx, r.x + 2, r.y + 2, 4, r.h - 4, open ? PAL.boss : PAL.panel);
    fillText(ctx, (i + 1) + '.', r.x + 22, r.y + r.h / 2, 13, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? cfg.name[lang === 'zh' ? 1 : 0].toUpperCase() : t('locked'),
      r.x + 44, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    if (open && G.save.cleared[i]) drawStars(r.x + r.w - 44, r.y + r.h / 2, G.save.stars[i] || 1, 10);
  }
  drawBtn(backBtn, t('back'), PAL.dim, false);
}

function renderDuel(now) {
  drawDuelScene(ctx, G.duel, now);
  drawTrack(ctx, G.duel);
  drawHud(ctx, G.duel);
  const s = G.duel;
  if (s.phase === 'stagger') {
    drawBtn(LAYOUT.exec, t('execute'), PAL.exec, true);
  } else {
    const atkSlash = s.phase === 'attack' && s.atk && s.atk.kind === 'slash';
    const atkThrust = s.phase === 'attack' && s.atk && s.atk.kind === 'thrust';
    drawBtn(LAYOUT.parry, t('parry'), PAL.slash, atkSlash);
    drawBtn(LAYOUT.dodge, t('dodge'), PAL.thrust, atkThrust);
  }
  drawFlash(ctx, s);
}

function renderResult() {
  drawBackdrop(ctx);
  const s = G.duel, won = s.won;
  fillText(ctx, won ? t('win') : t('lose'), 180, 96, 32, won ? PAL.perfect : PAL.hurt);
  fillText(ctx, BOSSES[G.bossIndex].name[lang === 'zh' ? 1 : 0].toUpperCase(),
    180, 134, 14, PAL.dim);
  if (won) {
    drawStars(180, 196, stars(s.hitsTaken), 22);
    if (s.hitsTaken === 0) fillText(ctx, t('flawless'), 180, 240, 14, PAL.perfect);
    fillText(ctx, t('combo') + ' x' + s.bestCombo, 180, 266, 12, PAL.dim);
  } else {
    fillText(ctx, t('combo') + ' x' + s.bestCombo, 180, 210, 12, PAL.dim);
  }
  const last = G.bossIndex >= BOSS_COUNT - 1;
  drawBtn(r1Btn, t('retry'), PAL.thrust, false);
  if (won && !last) drawBtn(r2Btn, t('next'), PAL.good, true);
  else fillText(ctx, won ? '—' : '', 254, 322, 12, PAL.dim);
  drawBtn(r3Btn, t('back'), PAL.dim, false);
}

// ---- transitions ---------------------------------------------------------
function startDuel(i) {
  G.bossIndex = i;
  G.duel = newDuel(i);
  G.overDelay = 0;
  G.screen = 'duel';
}
function finishDuel() {
  const s = G.duel;
  if (s.won) {
    G.save.cleared[G.bossIndex] = true;
    const st = stars(s.hitsTaken);
    if (!(G.save.stars[G.bossIndex] >= st)) G.save.stars[G.bossIndex] = st;
    persist();
  }
  G.screen = 'result';
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return {
    x: (e.clientX - r.left) / r.width * 360,
    y: (e.clientY - r.top) / r.height * 480,
  };
}
function onTap(p) {
  if (G.screen === 'title') {
    if (hit(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hit(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hit(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < BOSS_COUNT; i++) {
      if (hit(bossRow(i), p) && unlocked(i)) { startDuel(i); return; }
    }
  } else if (G.screen === 'duel') {
    const s = G.duel;
    if (s.over) return;
    if (s.phase === 'stagger') { input(s, 'execute'); return; }
    // the whole lower half is a generous tap zone, split at the centre line
    if (p.y >= 296) input(s, p.x < 180 ? 'parry' : 'dodge');
  } else if (G.screen === 'result') {
    if (hit(r1Btn, p)) { startDuel(G.bossIndex); return; }
    if (hit(r3Btn, p)) { G.screen = 'select'; return; }
    if (G.duel.won && G.bossIndex < BOSS_COUNT - 1 && hit(r2Btn, p)) {
      startDuel(G.bossIndex + 1);
    }
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'duel' && !G.duel.over) {
    const s = G.duel;
    if (s.phase === 'stagger') {
      if (e.key === ' ' || e.key === 'Enter') input(s, 'execute');
    } else if (e.key === 'ArrowLeft' || e.key === 'z' || e.key === 'Z') {
      input(s, 'parry');
    } else if (e.key === 'ArrowRight' || e.key === 'x' || e.key === 'X') {
      input(s, 'dodge');
    }
    if (['ArrowLeft', 'ArrowRight', ' '].includes(e.key)) e.preventDefault();
  } else if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) {
    G.screen = 'select';
  } else if (G.screen === 'result' && (e.key === 'r' || e.key === 'R')) {
    startDuel(G.bossIndex);
  }
});

// ---- main loop -----------------------------------------------------------
let last = 0;
function frame(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  if (G.screen === 'duel') {
    tick(G.duel, dt);
    if (G.duel.over) {
      G.overDelay += dt;
      if (G.overDelay > 0.9) finishDuel();
    }
  }
  if (G.screen === 'title') renderTitle(now);
  else if (G.screen === 'select') renderSelect();
  else if (G.screen === 'duel') renderDuel(now);
  else if (G.screen === 'result') renderResult();
  requestAnimationFrame(frame);
}

loadLang();
loadSave();
requestAnimationFrame(frame);
