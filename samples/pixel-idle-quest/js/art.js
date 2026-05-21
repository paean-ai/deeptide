// Pixel-art rendering for Pixel Idle Quest. 360x480 world units.

const PALETTE = {
  wall:     '#241f30',
  wallHi:   '#352e44',
  floor:    '#3a3142',
  floorHi:  '#4a4056',
  hero:     '#46b8e8',
  heroHi:   '#9be0ff',
  heroDk:   '#1f6a96',
  skin:     '#e8b890',
  blade:    '#d6dae8',
  hpBg:     '#2a1820',
  hp:       '#e8554f',
  hpHi:     '#ff8a82',
  gold:     '#f4c44a',
  relic:    '#a06fd0',
  hud:      '#0e0b16',
  hudText:  '#f3f1e6',
  panel:    '#2c2740',
  panelHi:  '#3e3858',
  btn:      '#46b8e8',
  btnOff:   '#39354e',
  ascend:   '#a06fd0',
  good:     '#5fc06e',
  flash:    'rgba(255,240,180,0.5)',
};
const MON_BODY = ['#5fc06e', '#e8554f', '#4aa6e0', '#f0883a', '#a06fd0', '#7a9a5a'];
const MON_DARK = ['#327a44', '#8a2a22', '#235e7e', '#9c4e16', '#5f3e8a', '#4a5e35'];

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.wall;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.wallHi;
  for (let x = 12; x < VW; x += 40) ctx.fillRect(x, 40, 2, 200);
  for (let y = 60; y < 240; y += 36) ctx.fillRect(0, y, VW, 2);
  // Floor band the fighters stand on.
  ctx.fillStyle = PALETTE.floor;
  ctx.fillRect(0, 200, VW, 44);
  ctx.fillStyle = PALETTE.floorHi;
  ctx.fillRect(0, 200, VW, 3);
}

function drawHero(ctx, x, y) {
  // Legs.
  ctx.fillStyle = PALETTE.heroDk;
  ctx.fillRect(x - 7, y - 10, 6, 10);
  ctx.fillRect(x + 1, y - 10, 6, 10);
  // Body.
  ctx.fillStyle = PALETTE.hero;
  ctx.fillRect(x - 9, y - 32, 18, 24);
  ctx.fillStyle = PALETTE.heroHi;
  ctx.fillRect(x - 9, y - 32, 5, 24);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect(x - 6, y - 44, 12, 12);
  ctx.fillStyle = PALETTE.heroDk;
  ctx.fillRect(x - 7, y - 47, 14, 4);
  // Blade raised.
  ctx.fillStyle = PALETTE.blade;
  ctx.fillRect(x + 10, y - 50, 3, 30);
  ctx.fillStyle = PALETTE.heroDk;
  ctx.fillRect(x + 8, y - 22, 7, 3);
}

function drawMonster(ctx, cx, cy, scale, stage) {
  const i = (stage - 1) % 6;
  const body = MON_BODY[i], dark = MON_DARK[i];
  const elite = stage % 5 === 0;
  const r = 30 * scale;
  if (elite) {
    ctx.fillStyle = PALETTE.gold;
    for (let k = -1; k <= 1; k++) {
      ctx.beginPath();
      ctx.moveTo(cx + k * 12, cy - r - 6);
      ctx.lineTo(cx + k * 12 - 5, cy - r + 6);
      ctx.lineTo(cx + k * 12 + 5, cy - r + 6);
      ctx.closePath(); ctx.fill();
    }
  }
  ctx.fillStyle = dark;
  ctx.fillRect((cx - r) | 0, (cy - r) | 0, (r * 2) | 0, (r * 2) | 0);
  ctx.fillStyle = body;
  ctx.fillRect((cx - r + 3) | 0, (cy - r + 3) | 0, (r * 2 - 6) | 0, (r * 2 - 6) | 0);
  ctx.fillStyle = 'rgba(255,255,255,0.18)';
  ctx.fillRect((cx - r + 3) | 0, (cy - r + 3) | 0, (r * 2 - 6) | 0, 4);
  // Eyes.
  ctx.fillStyle = '#fff';
  ctx.fillRect((cx - r * 0.5) | 0, (cy - r * 0.3) | 0, 8, 8);
  ctx.fillRect((cx + r * 0.5 - 8) | 0, (cy - r * 0.3) | 0, 8, 8);
  ctx.fillStyle = '#1a1320';
  ctx.fillRect((cx - r * 0.5 + 2) | 0, (cy - r * 0.3 + 2) | 0, 4, 5);
  ctx.fillRect((cx + r * 0.5 - 6) | 0, (cy - r * 0.3 + 2) | 0, 4, 5);
  // Fangs.
  ctx.fillStyle = '#fff';
  ctx.fillRect((cx - 6) | 0, (cy + r * 0.4) | 0, 4, 6);
  ctx.fillRect((cx + 2) | 0, (cy + r * 0.4) | 0, 4, 6);
}

function drawArena(ctx, lang, s) {
  // Hero on the left, monster on the right.
  drawHero(ctx, 78, 236);
  const punch = s.flash > 0 ? (1 - s.flash / 0.25) * 6 : 0;
  drawMonster(ctx, 244 + punch, 162, 1, s.stage);
  // Monster HP bar.
  const bx = 150, bw = 180, by = 60;
  ctx.fillStyle = PALETTE.hpBg;
  ctx.fillRect(bx - 2, by - 2, bw + 4, 16);
  const frac = Math.max(0, Math.min(1, s.monsterHp / s.monsterMax));
  ctx.fillStyle = PALETTE.hp;
  ctx.fillRect(bx, by, bw * frac, 12);
  ctx.fillStyle = PALETTE.hpHi;
  ctx.fillRect(bx, by, bw * frac, 3);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 10px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(fmt(Math.max(0, s.monsterHp)) + ' / ' + fmt(s.monsterMax), bx + bw / 2, by + 6);
  if (s.stage % 5 === 0) {
    ctx.fillStyle = PALETTE.gold;
    ctx.fillText(t(lang, 'elite'), bx + bw / 2, by + 24);
  }
  if (s.flash > 0) {
    ctx.fillStyle = 'rgba(255,240,180,' + (s.flash / 0.25 * 0.35) + ')';
    ctx.fillRect(0, 44, VW, 200);
  }
}

function drawHud(ctx, lang, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 40);
  ctx.textBaseline = 'middle';
  ctx.font = 'bold 13px monospace';
  ctx.textAlign = 'left';
  ctx.fillStyle = PALETTE.hudText;
  ctx.fillText(t(lang, 'stage') + ' ' + s.stage, 8, 13);
  ctx.fillStyle = '#9a93b0';
  ctx.font = '10px monospace';
  ctx.fillText(t(lang, 'stage') + ' ' + s.kills + '/10', 8, 29);
  ctx.textAlign = 'right';
  ctx.fillStyle = PALETTE.gold;
  ctx.font = 'bold 13px monospace';
  ctx.fillText(fmt(s.gold) + ' G', VW - 8, 13);
  if (s.relics > 0) {
    ctx.fillStyle = PALETTE.relic;
    ctx.font = '10px monospace';
    ctx.fillText('◆ ' + s.relics, VW - 8, 29);
  }
}

function drawTitleArt(ctx, cx, cy) {
  drawHero(ctx, cx - 56, cy + 38);
  drawMonster(ctx, cx + 44, cy, 1, 3);
}
