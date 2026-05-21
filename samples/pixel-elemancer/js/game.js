// Pixel Elemancer - screen flow, real-time render loop, input, save.

const canvas = document.getElementById('view');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-elemancer:save';
const G = {
  screen: 'title',          // title | select | battle
  battle: null,
  foeIndex: 0,
  fx: [],
  lock: 0,                  // input lock-out while feedback plays (seconds)
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
function hit(r, p) { return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h; }

// ---- fixed geometry ------------------------------------------------------
const startBtn = { x: 96, y: 322, w: 168, h: 48 };
const langBtn = { x: 296, y: 14, w: 50, h: 26 };
const backBtn = { x: 110, y: 424, w: 140, h: 38 };
function foeRow(i) { return { x: 30, y: 92 + i * 54, w: 300, h: 46 }; }
const cardX = 44, cardY = 122, cardW = 272, cardH = 224;
const rRetry = { x: cardX + 18, y: cardY + 150, w: 118, h: 36 };
const rNext = { x: cardX + 136, y: cardY + 150, w: 118, h: 36 };
const rRetryWide = { x: cardX + 18, y: cardY + 150, w: 236, h: 36 };
const rMenu = { x: cardX + 18, y: cardY + 194, w: 236, h: 32 };

// ---- floating combat text ------------------------------------------------
function addFx(x, y, text, color, size) {
  G.fx.push({ x, y, text, color, size: size || 16, life: 0.85, max: 0.85 });
}
function spawnFx(res) {
  if (res.hit) {
    const c = res.hit.eff === 'super' ? PAL.star : (res.hit.eff === 'weak' ? PAL.dim : PAL.text);
    addFx(180, 92, '-' + res.hit.amount, c, 19);
    if (res.hit.eff === 'super') addFx(180, 64, t('strike') + '!', PAL.star, 13);
  }
  if (res.healed) addFx(180, 250, '+' + res.healed, PAL.good, 16);
  if (res.foeHit) {
    addFx(180, 250, '-' + res.foeHit.amount, res.foeHit.charged ? PAL.bad : PAL.text, 17);
    if (res.foeHit.charged) addFx(180, 274, t('charged') + '!', PAL.bad, 12);
  }
  if (res.knockedOut) addFx(180, 230, 'K.O.', PAL.bad, 18);
}

// ---- screen renderers ----------------------------------------------------
function renderTitle(now) {
  drawTitleArt(ctx, now);
  fillText(ctx, t('title'), 180, 56, 25, PAL.text);
  fillText(ctx, t('subtitle'), 180, 84, 11, PAL.dim);
  drawActionBtn(ctx, startBtn, t('start'), null, PAL.good, true);
  drawActionBtn(ctx, langBtn, lang === 'en' ? '中文' : 'EN', null, PAL.panelHi, true);
  fillText(ctx, t('rules1'), 180, 386, 9, PAL.dim);
  fillText(ctx, t('rules2'), 180, 404, 9, PAL.dim);
  fillText(ctx, t('rules3'), 180, 422, 9, PAL.dim);
}

function renderSelect() {
  drawBackdrop(ctx);
  fillText(ctx, t('foeSelect'), 180, 54, 18, PAL.text);
  for (let i = 0; i < FOE_COUNT; i++) {
    const r = foeRow(i), open = unlocked(i), foe = FOES[i];
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, open ? PAL.panel : PAL.bg0);
    if (open) elemBadge(ctx, r.x + 12, r.y + 15, foe.elem);
    fillText(ctx, (i + 1) + '.', r.x + 40, r.y + r.h / 2, 12, open ? PAL.dim : PAL.panelHi, 'left');
    fillText(ctx, open ? L(foe.name).toUpperCase() : t('locked'),
      r.x + 58, r.y + r.h / 2, 13, open ? PAL.text : PAL.panelHi, 'left');
    if (open && G.save.cleared[i]) drawStars(r.x + r.w - 42, r.y + r.h / 2, G.save.stars[i] || 1);
  }
  drawActionBtn(ctx, backBtn, t('back'), null, PAL.panelHi, true);
}

function drawStars(cx, cy, n) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * 14, on = i < n;
    px(ctx, x - 5, cy - 5, 10, 10, on ? PAL.star : PAL.panel);
    px(ctx, x - 2, cy - 8, 4, 3, on ? PAL.star : PAL.panel);
    px(ctx, x - 2, cy + 5, 4, 3, on ? PAL.star : PAL.panel);
  }
}

function renderBattle(now) {
  const b = G.battle;
  drawBattle(ctx, b, now);
  if (!b.over) {
    const cur = b.forms[b.current];
    if (b.mustShift) {
      px(ctx, 12, 358, 336, 104, PAL.ink);
      px(ctx, 14, 360, 332, 100, PAL.panel);
      fillText(ctx, t('chooseForm'), 180, 400, 15, PAL.star);
      fillText(ctx, '▲ ' + t('chooseForm'), 180, 424, 9, PAL.dim);
    } else {
      drawActionBtn(ctx, STRIKE_BTN, t('strike'), '~' + strikeEstimate(b), PAL.bad, true);
      drawActionBtn(ctx, GUARD_BTN, t('guard'), '50% + ✦', ELEM[cur.elem].base, true);
      const spName = L(SPECIAL_NAME[cur.special]);
      drawActionBtn(ctx, SPECIAL_BTN, t('special'),
        cur.specialUsed ? t('used') : spName, PAL.star, !cur.specialUsed);
    }
  }
  drawFx(ctx, G.fx);
  if (b.over && G.lock <= 0) renderResultCard(b);
}

function renderResultCard(b) {
  ctx.globalAlpha = 0.74; px(ctx, 0, 0, 360, 480, PAL.bg0); ctx.globalAlpha = 1;
  px(ctx, cardX - 3, cardY - 3, cardW + 6, cardH + 6, PAL.ink);
  px(ctx, cardX, cardY, cardW, cardH, PAL.panel);
  px(ctx, cardX, cardY, cardW, 4, PAL.panelHi);
  const cx = cardX + cardW / 2;
  fillText(ctx, b.won ? t('win') : t('lose'), cx, cardY + 40, 19, b.won ? PAL.good : PAL.bad);
  const last = G.foeIndex >= FOE_COUNT - 1;
  if (b.won) {
    const st = stars(b);
    for (let i = 0; i < 3; i++) {
      const x = cx + (i - 1) * 30, on = i < st;
      px(ctx, x - 11, cardY + 78, 22, 22, on ? PAL.star : PAL.panel);
      px(ctx, x - 5, cardY + 70, 10, 6, on ? PAL.star : PAL.panel);
      px(ctx, x - 5, cardY + 100, 10, 6, on ? PAL.star : PAL.panel);
    }
    fillText(ctx, formsAlive(b) + ' / 4 forms standing', cx, cardY + 128, 11, PAL.dim);
  } else {
    fillText(ctx, 'all four forms fell', cx, cardY + 96, 12, PAL.dim);
  }
  if (b.won && !last) {
    drawActionBtn(ctx, rRetry, t('retry'), null, PAL.panelHi, true);
    drawActionBtn(ctx, rNext, t('next'), null, PAL.good, true);
  } else {
    drawActionBtn(ctx, rRetryWide, t('retry'), null, PAL.good, true);
  }
  drawActionBtn(ctx, rMenu, t('back'), null, PAL.panelHi, true);
}

// ---- transitions ---------------------------------------------------------
function startBattle(i) {
  G.foeIndex = i;
  G.battle = newBattle(i);
  G.fx = [];
  G.lock = 0;
  G.screen = 'battle';
}
function doAction(action) {
  const b = G.battle;
  const res = act(b, action);
  if (!res) return;
  spawnFx(res);
  G.lock = res.free ? 0.15 : 0.6;
  if (b.over && b.won) {
    G.save.cleared[G.foeIndex] = true;
    const st = stars(b);
    if (!(G.save.stars[G.foeIndex] >= st)) G.save.stars[G.foeIndex] = st;
    persist();
  }
}

// ---- input ---------------------------------------------------------------
function eventPos(e) {
  const r = canvas.getBoundingClientRect();
  return { x: (e.clientX - r.left) / r.width * 360, y: (e.clientY - r.top) / r.height * 480 };
}
function onTap(p) {
  if (G.screen === 'title') {
    if (hit(langBtn, p)) { lang = lang === 'en' ? 'zh' : 'en'; saveLang(); return; }
    if (hit(startBtn, p)) { G.screen = 'select'; return; }
  } else if (G.screen === 'select') {
    if (hit(backBtn, p)) { G.screen = 'title'; return; }
    for (let i = 0; i < FOE_COUNT; i++) {
      if (hit(foeRow(i), p) && unlocked(i)) { startBattle(i); return; }
    }
  } else if (G.screen === 'battle') {
    const b = G.battle;
    if (b.over) {
      if (G.lock > 0) return;
      const last = G.foeIndex >= FOE_COUNT - 1;
      if (hit(rMenu, p)) { G.screen = 'select'; return; }
      if (b.won && !last) {
        if (hit(rRetry, p)) { startBattle(G.foeIndex); return; }
        if (hit(rNext, p)) { startBattle(G.foeIndex + 1); return; }
      } else if (hit(rRetryWide, p)) { startBattle(G.foeIndex); return; }
      return;
    }
    if (G.lock > 0) return;
    for (let i = 0; i < b.forms.length; i++) {
      if (hit(chipRect(i), p)) { doAction({ type: 'shift', form: i }); return; }
    }
    if (b.mustShift) return;
    if (hit(STRIKE_BTN, p)) doAction({ type: 'strike' });
    else if (hit(GUARD_BTN, p)) doAction({ type: 'guard' });
    else if (hit(SPECIAL_BTN, p)) doAction({ type: 'special' });
  }
}
canvas.addEventListener('pointerdown', (e) => { e.preventDefault(); onTap(eventPos(e)); });

window.addEventListener('keydown', (e) => {
  if (G.screen === 'battle' && G.battle && !G.battle.over && G.lock <= 0) {
    const k = e.key;
    if (k >= '1' && k <= '4') doAction({ type: 'shift', form: parseInt(k, 10) - 1 });
    else if (!G.battle.mustShift) {
      if (k === 'z' || k === 'Z' || k === ' ') doAction({ type: 'strike' });
      else if (k === 'x' || k === 'X') doAction({ type: 'guard' });
      else if (k === 'c' || k === 'C') doAction({ type: 'special' });
    }
    if (k === ' ') e.preventDefault();
  } else if (G.screen === 'title' && (e.key === ' ' || e.key === 'Enter')) {
    G.screen = 'select';
  } else if (G.screen === 'battle' && G.battle && G.battle.over && G.lock <= 0 && e.key === 'Enter') {
    const b = G.battle, last = G.foeIndex >= FOE_COUNT - 1;
    if (b.won && !last) startBattle(G.foeIndex + 1); else startBattle(G.foeIndex);
  }
});

// ---- loop ----------------------------------------------------------------
let last = 0;
function frame(now) {
  const dt = last ? Math.min(0.05, (now - last) / 1000) : 0;
  last = now;
  if (G.lock > 0) G.lock = Math.max(0, G.lock - dt);
  for (let i = G.fx.length - 1; i >= 0; i--) {
    G.fx[i].life -= dt;
    if (G.fx[i].life <= 0) G.fx.splice(i, 1);
  }
  if (G.screen === 'title') renderTitle(now);
  else if (G.screen === 'select') renderSelect();
  else if (G.screen === 'battle') renderBattle(now);
  requestAnimationFrame(frame);
}

loadLang();
loadSave();
requestAnimationFrame(frame);
