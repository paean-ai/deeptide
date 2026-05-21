// Pixel Foray - all canvas drawing: dungeon room, foes, telegraph, title.

const PAL = {
  bg0: '#15121d', bg1: '#221b2e', frame: '#0b0911',
  floor: '#2c2740', floorAlt: '#332d4a', wall: '#564f68', wallDk: '#332f44', wallHi: '#6f6884',
  panel: '#2b2440', panelHi: '#3d3356', ink: '#0b0911',
  text: '#f3f1e6', dim: '#9a90af', good: '#7bd88f', star: '#ffe27a',
  heart: '#ff5d6c', heartDk: '#48232f',
  danger: '#ff5d6c', ghost: '#cdbff0',
  hero: '#54a0ec', heroDk: '#2c5f9c', heroHi: '#bfe0ff',
  grunt: '#6fae54', gruntDk: '#3c6e30',
  brute: '#d2603c', bruteDk: '#852f1f',
  archer: '#b07ce0', archerDk: '#5d3c8c',
};

function px(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w | 0, h | 0); }
function fillText(ctx, str, x, y, size, color, align) {
  ctx.fillStyle = color;
  ctx.font = size + 'px monospace';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(str, x, y);
}

// ---- layout --------------------------------------------------------------
const BG = { n: 7, cell: 44, ox: 26, oy: 78 };
function cellRect(cell) {
  return { x: BG.ox + (cell % 7) * BG.cell, y: BG.oy + ((cell / 7 | 0)) * BG.cell, w: BG.cell, h: BG.cell };
}
function cellAt(p) {
  const c = (p.x - BG.ox) / BG.cell | 0, r = (p.y - BG.oy) / BG.cell | 0;
  if (r < 0 || c < 0 || r >= 7 || c >= 7) return -1;
  if (p.x < BG.ox || p.y < BG.oy) return -1;
  return r * 7 + c;
}
const RESTART_BTN = { x: 110, y: 396, w: 140, h: 40 };

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, 480);
  g.addColorStop(0, PAL.bg0); g.addColorStop(1, PAL.bg1);
  ctx.fillStyle = g; ctx.fillRect(0, 0, 360, 480);
}

// ---- figures -------------------------------------------------------------
function drawHero(ctx, cx, cy, u) {
  px(ctx, cx - 4 * u, cy - 2 * u, 8 * u, 6 * u, PAL.hero);          // body
  px(ctx, cx - 4 * u, cy - 2 * u, 8 * u, 2 * u, PAL.heroHi);
  px(ctx, cx - 3 * u, cy - 7 * u, 6 * u, 5 * u, '#e8c79a');         // head
  px(ctx, cx - 3 * u, cy - 8 * u, 6 * u, 2 * u, PAL.heroDk);        // helm
  px(ctx, cx - 4 * u, cy + 4 * u, 3 * u, 3 * u, PAL.heroDk);
  px(ctx, cx + 1 * u, cy + 4 * u, 3 * u, 3 * u, PAL.heroDk);
  px(ctx, cx + 4 * u, cy - 6 * u, 2 * u, 9 * u, PAL.heroHi);        // blade
}
function drawFoe(ctx, cx, cy, u, type) {
  if (type === 'grunt') {
    px(ctx, cx - 3 * u, cy - 4 * u, 6 * u, 8 * u, PAL.grunt);
    px(ctx, cx - 3 * u, cy - 4 * u, 6 * u, 2 * u, '#9be88a');
    px(ctx, cx - 3 * u, cy + 4 * u, 2 * u, 2 * u, PAL.gruntDk);
    px(ctx, cx + 1 * u, cy + 4 * u, 2 * u, 2 * u, PAL.gruntDk);
    px(ctx, cx - 2 * u, cy - 2 * u, 1 * u, 1 * u, PAL.ink);
    px(ctx, cx + 1 * u, cy - 2 * u, 1 * u, 1 * u, PAL.ink);
  } else if (type === 'brute') {
    px(ctx, cx - 5 * u, cy - 5 * u, 10 * u, 10 * u, PAL.brute);
    px(ctx, cx - 5 * u, cy - 5 * u, 10 * u, 2 * u, '#f0a07a');
    px(ctx, cx - 6 * u, cy - 6 * u, 2 * u, 3 * u, PAL.bruteDk);
    px(ctx, cx + 4 * u, cy - 6 * u, 2 * u, 3 * u, PAL.bruteDk);
    px(ctx, cx - 3 * u, cy - 2 * u, 2 * u, 2 * u, PAL.ink);
    px(ctx, cx + 1 * u, cy - 2 * u, 2 * u, 2 * u, PAL.ink);
  } else {
    px(ctx, cx - 3 * u, cy - 4 * u, 6 * u, 8 * u, PAL.archer);
    px(ctx, cx - 3 * u, cy - 4 * u, 6 * u, 2 * u, '#d6b8f5');
    px(ctx, cx - 5 * u, cy - 5 * u, 2 * u, 10 * u, PAL.archerDk);   // bow
    px(ctx, cx - 2 * u, cy - 2 * u, 1 * u, 1 * u, PAL.ink);
    px(ctx, cx + 1 * u, cy - 2 * u, 1 * u, 1 * u, PAL.ink);
  }
}
function foeColor(type) { return type === 'grunt' ? PAL.grunt : type === 'brute' ? PAL.brute : PAL.archer; }

function drawHpPips(ctx, cx, y, hp, max, color) {
  for (let i = 0; i < max; i++) {
    px(ctx, cx - max * 3 + i * 6, y, 4, 3, i < hp ? color : PAL.ink);
  }
}

// ---- the room ------------------------------------------------------------
function drawRoom(ctx, s) {
  drawBackdrop(ctx);
  const danger = dangerCells(s);
  // floor + walls
  for (let i = 0; i < 49; i++) {
    const r = cellRect(i), isWall = s.walls.has(i);
    px(ctx, r.x, r.y, r.w, r.h, isWall ? PAL.wall : (((i + (i / 7 | 0)) % 2) ? PAL.floorAlt : PAL.floor));
    px(ctx, r.x, r.y, r.w, 1, PAL.ink);
    px(ctx, r.x, r.y, 1, r.h, PAL.ink);
    if (isWall) {
      px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, PAL.wallHi);
      px(ctx, r.x + 2, r.y + r.h - 5, r.w - 4, 3, PAL.wallDk);
    }
  }
  // danger tint
  if (!s.over) {
    for (const c of danger) {
      const r = cellRect(c);
      ctx.globalAlpha = 0.34; px(ctx, r.x + 1, r.y + 1, r.w - 2, r.h - 2, PAL.danger);
      ctx.globalAlpha = 1;
      px(ctx, r.x + 3, r.y + 3, 4, 4, PAL.danger);
      px(ctx, r.x + r.w - 7, r.y + r.h - 7, 4, 4, PAL.danger);
    }
  }
  // telegraph ghosts for melee plans
  if (!s.over) {
    for (const e of s.enemies) {
      if (!e.plan || !e.plan.move || e.plan.move === e.cell) continue;
      const r = cellRect(e.plan.move);
      ctx.globalAlpha = 0.5;
      const m = 9;
      px(ctx, r.x + m, r.y + m, r.w - 2 * m, r.h - 2 * m, foeColor(e.type));
      ctx.globalAlpha = 1;
    }
  }
  // hero-reach hints: faint outline on tap-able neighbour cells
  if (!s.over) {
    for (const c of legalMoves(s)) {
      if (c === s.hero.cell) continue;
      const r = cellRect(c);
      px(ctx, r.x + 1, r.y + 1, r.w - 2, 2, PAL.star);
    }
  }
  // enemies
  for (const e of s.enemies) {
    const r = cellRect(e.cell);
    drawFoe(ctx, r.x + r.w / 2, r.y + r.h / 2, 2.4, e.type);
    drawHpPips(ctx, r.x + r.w / 2, r.y + r.h - 7, e.hp, ETYPES[e.type].hp, foeColor(e.type));
  }
  // hero
  const hr = cellRect(s.hero.cell);
  drawHero(ctx, hr.x + hr.w / 2, hr.y + hr.h / 2, 2.6);
}

// ---- HUD -----------------------------------------------------------------
function drawHud(ctx, s) {
  fillText(ctx, L(s.room.name).toUpperCase(), 180, 20, 16, PAL.text);
  for (let i = 0; i < HERO_HP; i++) {
    const on = i < s.hero.hp;
    px(ctx, 18 + i * 16, 42, 12, 6, on ? PAL.heart : PAL.heartDk);
    px(ctx, 20 + i * 16, 40, 8, 3, on ? PAL.heart : PAL.heartDk);
  }
  fillText(ctx, t('foes') + ' ' + s.enemies.length, 342, 44, 12, PAL.dim, 'right');
}

// ---- buttons + stars -----------------------------------------------------
function drawBtn(ctx, r, label, color, active) {
  px(ctx, r.x, r.y, r.w, r.h, PAL.ink);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, r.h - 4, active ? color : PAL.panel);
  px(ctx, r.x + 2, r.y + 2, r.w - 4, 3, active ? PAL.text : PAL.panelHi);
  fillText(ctx, label, r.x + r.w / 2, r.y + r.h / 2, r.h > 46 ? 16 : 13, active ? PAL.ink : color);
}
function drawStars(ctx, cx, cy, n, size) {
  for (let i = 0; i < 3; i++) {
    const x = cx + (i - 1) * (size + 6), on = i < n;
    px(ctx, x - size / 2, cy - size / 2, size, size, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy - size / 2 - 3, 6, 3, on ? PAL.star : PAL.panel);
    px(ctx, x - 3, cy + size / 2, 6, 3, on ? PAL.star : PAL.panel);
  }
}

// ---- title art -----------------------------------------------------------
function drawTitleArt(ctx, now) {
  drawBackdrop(ctx);
  const g = { cell: 40, ox: 100, oy: 120 };
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    const x = g.ox + c * g.cell, y = g.oy + r * g.cell;
    px(ctx, x, y, g.cell, g.cell, ((r + c) % 2) ? PAL.floorAlt : PAL.floor);
    px(ctx, x, y, g.cell, 1, PAL.ink);
    px(ctx, x, y, 1, g.cell, PAL.ink);
  }
  const pulse = 0.3 + 0.25 * Math.sin(now / 300);
  ctx.globalAlpha = pulse;
  px(ctx, g.ox + g.cell + 1, g.oy + g.cell + 1, g.cell - 2, g.cell - 2, PAL.danger);
  ctx.globalAlpha = 1;
  drawFoe(ctx, g.ox + g.cell * 2.5, g.oy + g.cell * 0.5, 2.8, 'grunt');
  drawFoe(ctx, g.ox + g.cell * 0.5, g.oy + g.cell * 2.5, 2.8, 'archer');
  drawHero(ctx, g.ox + g.cell * 1.5, g.oy + g.cell * 2.5, 3.0);
}
