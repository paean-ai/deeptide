// Pixel Elemancer - all canvas drawing: foe, elemancer, form chips, title.

const PAL = {
  bg0: '#171229', bg1: '#241b3a', frame: '#0c0916',
  panel: '#2a2240', panelHi: '#3c3158', ink: '#0c0916',
  text: '#f3f1e6', dim: '#9a90b0', good: '#7bd88f', bad: '#ff6470',
  star: '#ffe27a', hpHi: '#7bd88f', hpMid: '#f2c83a', hpLow: '#ff6470', hpBg: '#1c1730',
};
// element palettes: 0 fire, 1 grass, 2 storm, 3 water
const ELEM = [
  { base: '#f0703a', dark: '#9c3a1c', hi: '#ffb079' },
  { base: '#5fc06e', dark: '#2f7a3c', hi: '#9be88a' },
  { base: '#9a6cd8', dark: '#5a3c8c', hi: '#c39bf2' },
  { base: '#4a9be8', dark: '#235f96', hi: '#8fc8ff' },
];

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}
function hpColor(frac) { return frac > 0.5 ? PAL.hpHi : (frac > 0.25 ? PAL.hpMid : PAL.hpLow); }
function drawBar(ctx, x, y, w, h, frac) {
  px(ctx, x - 2, y - 2, w + 4, h + 4, PAL.ink);
  px(ctx, x, y, w, h, PAL.hpBg);
  const f = Math.max(0, Math.min(1, frac));
  if (f > 0) px(ctx, x, y, Math.max(1, w * f), h, hpColor(f));
  px(ctx, x, y, Math.max(1, w * f), 2, '#ffffff44');
}

// ---- layout --------------------------------------------------------------
const CHIP_Y = 292, CHIP_H = 56;
function chipRect(i) { return { x: 12 + i * 85, y: CHIP_Y, w: 79, h: CHIP_H }; }
const STRIKE_BTN = { x: 12, y: 358, w: 110, h: 104 };
const GUARD_BTN = { x: 126, y: 358, w: 108, h: 104 };
const SPECIAL_BTN = { x: 238, y: 358, w: 110, h: 104 };

// ---- backdrop ------------------------------------------------------------
function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

function elemBadge(ctx, x, y, elem) {
  const e = ELEM[elem];
  px(ctx, x, y, 16, 16, PAL.ink);
  px(ctx, x + 2, y + 2, 12, 12, e.base);
  px(ctx, x + 2, y + 2, 12, 3, e.hi);
}

// ---- a foe creature ------------------------------------------------------
function drawCreature(ctx, cx, cy, u, elem, variant, now) {
  const e = ELEM[elem];
  const bob = Math.sin(now / 360 + variant) * u;
  cy += bob;
  const bw = (10 + variant) * u, bh = (9 + variant) * u;
  // spikes / crown grow with the variant
  for (let i = 0; i <= variant; i++) {
    const sx = cx - bw / 2 + (i + 0.5) * (bw / (variant + 1));
    px(ctx, sx - u, cy - bh / 2 - 3 * u, 2 * u, 3 * u, e.dark);
  }
  px(ctx, cx - bw / 2, cy - bh / 2, bw, bh, e.base);
  px(ctx, cx - bw / 2, cy - bh / 2, bw, 3 * u, e.hi);
  px(ctx, cx - bw / 2, cy + bh / 2 - 2 * u, bw, 2 * u, e.dark);
  // eyes
  const eyeN = variant >= 3 ? 3 : 2;
  for (let i = 0; i < eyeN; i++) {
    const ex = cx - (eyeN - 1) * 2.4 * u + i * 4.8 * u;
    px(ctx, ex - u, cy - u, 2 * u, 2 * u, PAL.ink);
    px(ctx, ex - u, cy - u, u, u, PAL.text);
  }
  // little legs
  px(ctx, cx - bw / 2 + u, cy + bh / 2, 2 * u, 2 * u, e.dark);
  px(ctx, cx + bw / 2 - 3 * u, cy + bh / 2, 2 * u, 2 * u, e.dark);
}

// ---- the elemancer hero --------------------------------------------------
function drawElemancer(ctx, cx, baseY, u, elem, now, guarding) {
  const e = ELEM[elem];
  const bob = Math.sin(now / 420) * (u * 0.5);
  baseY += bob;
  // robe
  px(ctx, cx - 6 * u, baseY - 16 * u, 12 * u, 16 * u, e.base);
  px(ctx, cx - 6 * u, baseY - 16 * u, 12 * u, 3 * u, e.hi);
  px(ctx, cx - 6 * u, baseY - 4 * u, 12 * u, 4 * u, e.dark);
  // head + hood
  px(ctx, cx - 4 * u, baseY - 24 * u, 8 * u, 8 * u, e.dark);
  px(ctx, cx - 3 * u, baseY - 22 * u, 6 * u, 4 * u, '#e8c79a');
  px(ctx, cx - 4 * u, baseY - 25 * u, 8 * u, 2 * u, e.hi);
  // a floating elemental orb
  const ox = cx + 9 * u, oy = baseY - 14 * u + Math.sin(now / 240) * 2 * u;
  px(ctx, ox - 2 * u, oy - 2 * u, 4 * u, 4 * u, e.hi);
  px(ctx, ox - u, oy - u, 2 * u, 2 * u, PAL.text);
  if (guarding) {
    ctx.globalAlpha = 0.5;
    px(ctx, cx - 9 * u, baseY - 24 * u, 18 * u, 24 * u, e.hi);
    ctx.globalAlpha = 1;
  }
}

// ---- battle scene --------------------------------------------------------
function drawBattle(ctx, b, now) {
  drawBackdrop(ctx);
  // foe panel
  const foe = b.foe;
  fillText(ctx, L(foe.name).toUpperCase(), 180, 20, 15, PAL.text);
  elemBadge(ctx, 92, 12, foe.elem);
  drawCreature(ctx, 180, 86, 3.4, foe.elem, b.foeIndex, now);
  drawBar(ctx, 60, 132, 240, 12, foe.hp / foe.maxHP);
  fillText(ctx, foe.hp + ' / ' + foe.maxHP, 180, 138, 9, PAL.ink);
  if (foe.chargeEvery > 0) {
    const charging = ((foe.turnsTaken + 1) % foe.chargeEvery) === 0;
    fillText(ctx, charging ? '⚡ ' + t('charged') : '', 180, 152, 10, PAL.bad);
  }

  // elemancer
  const cur = b.forms[b.current];
  drawElemancer(ctx, 180, 268, 3.0, cur.elem, now, b.guarding);
  // effectiveness + threat readouts
  const est = strikeEstimate(b);
  const eff = effLabel(cur.elem, foe.elem);
  const effC = eff === 'super' ? PAL.good : (eff === 'weak' ? PAL.bad : PAL.dim);
  fillText(ctx, 'STRIKE ~' + est + (eff === 'super' ? ' !' : (eff === 'weak' ? ' ▼' : '')),
    96, 198, 11, effC);
  fillText(ctx, t('threat') + ' ~' + foeThreat(b), 264, 198, 11, PAL.dim);
  if (b.empowered) fillText(ctx, '✦ ' + t('empowered'), 180, 216, 11, PAL.star);

  drawFormChips(ctx, b);
}

function drawFormChips(ctx, b) {
  for (let i = 0; i < b.forms.length; i++) {
    const f = b.forms[i], r = chipRect(i), e = ELEM[f.elem];
    const isCur = i === b.current;
    px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
    px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, f.exhausted ? PAL.bg0 : PAL.panel);
    if (isCur && !b.over) {
      px(ctx, r.x, r.y, r.w, 4, PAL.star);
      px(ctx, r.x, r.y, 4, r.h, PAL.star);
    }
    // element swatch
    px(ctx, r.x + 6, r.y + 7, 14, 14, f.exhausted ? PAL.panelHi : e.base);
    if (!f.exhausted) px(ctx, r.x + 6, r.y + 7, 14, 3, e.hi);
    fillText(ctx, L(f.name), r.x + r.w / 2 + 8, r.y + 15, 9.5,
      f.exhausted ? PAL.dim : PAL.text);
    if (f.exhausted) {
      fillText(ctx, 'K.O.', r.x + r.w / 2, r.y + 38, 11, PAL.bad);
    } else {
      drawBar(ctx, r.x + 8, r.y + 34, r.w - 16, 8, f.hp / f.maxHP);
      fillText(ctx, f.hp + '/' + f.maxHP, r.x + r.w / 2, r.y + 49, 8, PAL.dim);
    }
    if (!f.exhausted && f.specialUsed) px(ctx, r.x + r.w - 9, r.y + 6, 5, 5, PAL.dim);
  }
}

// ---- action bar ----------------------------------------------------------
function drawActionBtn(ctx, r, label, sub, color, enabled) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, enabled ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, enabled ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2 - 8, 15, enabled ? PAL.ink : PAL.dim);
  if (sub) fillText(ctx, sub, r.x + r.w / 2, r.y + r.h / 2 + 14, 8.5,
    enabled ? PAL.ink : PAL.dim);
}

// ---- floating combat text ------------------------------------------------
function drawFx(ctx, fx) {
  for (const f of fx) {
    const a = Math.max(0, f.life / f.max);
    ctx.globalAlpha = a;
    fillText(ctx, f.text, f.x, f.y - (1 - a) * 22, f.size || 16, f.color);
    ctx.globalAlpha = 1;
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  // four elemental orbs orbiting
  for (let i = 0; i < 4; i++) {
    const ang = now / 700 + i * Math.PI / 2;
    const ox = 180 + Math.cos(ang) * 64;
    const oy = 168 + Math.sin(ang) * 40;
    const e = ELEM[i];
    px(ctx, ox - 9, oy - 9, 18, 18, e.base);
    px(ctx, ox - 9, oy - 9, 18, 4, e.hi);
    px(ctx, ox - 4, oy - 4, 8, 8, PAL.text);
  }
  drawElemancer(ctx, 180, 210, 3.4, (Math.floor(now / 900) % 4), now, false);
}
