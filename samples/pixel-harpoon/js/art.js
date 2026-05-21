// Pixel-art rendering for Pixel Harpoon. 360x480 world units.

const PALETTE = {
  cave:     '#1a1530',
  caveLo:   '#120e22',
  wall:     '#3a3358',
  wallHi:   '#544a78',
  floor:    '#4a3a2c',
  floorHi:  '#63503c',
  ceil:     '#2a2440',
  orbBody:  ['#5fd0e8', '#f4c84a', '#f0883a', '#e8554f'],
  orbHi:    ['#bff0fa', '#fce8a8', '#ffc488', '#ff9a8e'],
  orbLo:    ['#2c7e96', '#a07e1e', '#9c4e16', '#8a2a22'],
  wire:     '#d6dae8',
  wireGlow: '#8fa0d0',
  spear:    '#ffe27a',
  diver:    '#46c2b6',
  diverHi:  '#7fe3d8',
  diverDk:  '#1f6c64',
  skin:     '#e8b890',
  hud:      '#0e0b1c',
  hudText:  '#f3f1e6',
  heart:    '#ff4a5a',
  accent:   '#f4c44a',
  win:      '#5fd07a',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.cave;
  ctx.fillRect(0, 0, VW, VH);
  // Cavern speckle.
  ctx.fillStyle = PALETTE.caveLo;
  for (let i = 0; i < 70; i++) {
    const x = (i * 71 + 17) % VW, y = CEIL_Y + (i * 53 + 11) % (FLOOR_Y - CEIL_Y);
    ctx.fillRect(x, y, 2, 2);
  }
  // Ceiling.
  ctx.fillStyle = PALETTE.ceil;
  ctx.fillRect(0, 0, VW, CEIL_Y);
  ctx.fillStyle = PALETTE.wallHi;
  ctx.fillRect(0, CEIL_Y - 2, VW, 2);
  // Side walls.
  ctx.fillStyle = PALETTE.wall;
  ctx.fillRect(0, CEIL_Y, WALL_L, FLOOR_Y - CEIL_Y);
  ctx.fillRect(WALL_R, CEIL_Y, VW - WALL_R, FLOOR_Y - CEIL_Y);
  ctx.fillStyle = PALETTE.wallHi;
  ctx.fillRect(WALL_L - 2, CEIL_Y, 2, FLOOR_Y - CEIL_Y);
  ctx.fillRect(WALL_R, CEIL_Y, 2, FLOOR_Y - CEIL_Y);
  // Floor.
  ctx.fillStyle = PALETTE.floor;
  ctx.fillRect(0, FLOOR_Y, VW, VH - FLOOR_Y);
  ctx.fillStyle = PALETTE.floorHi;
  ctx.fillRect(0, FLOOR_Y, VW, 3);
  ctx.fillStyle = PALETTE.caveLo;
  for (let x = 6; x < VW; x += 16) ctx.fillRect(x, FLOOR_Y + 6, 3, 2);
}

function drawOrb(ctx, b) {
  ctx.fillStyle = PALETTE.orbLo[b.size];
  ctx.beginPath(); ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.orbBody[b.size];
  ctx.beginPath(); ctx.arc(b.x, b.y, b.r - 2, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.orbHi[b.size];
  ctx.beginPath();
  ctx.arc(b.x - b.r * 0.32, b.y - b.r * 0.34, Math.max(2, b.r * 0.34), 0, Math.PI * 2);
  ctx.fill();
}

function drawHarpoon(ctx, s) {
  if (!s.harpoon) return;
  const h = s.harpoon;
  ctx.fillStyle = PALETTE.wireGlow;
  ctx.fillRect((h.x - 2) | 0, h.tipY | 0, 4, FLOOR_Y - h.tipY);
  ctx.fillStyle = PALETTE.wire;
  ctx.fillRect((h.x - 1) | 0, h.tipY | 0, 2, FLOOR_Y - h.tipY);
  // Spearhead.
  ctx.fillStyle = PALETTE.spear;
  ctx.beginPath();
  ctx.moveTo(h.x, h.tipY - 8);
  ctx.lineTo(h.x - 5, h.tipY + 3);
  ctx.lineTo(h.x + 5, h.tipY + 3);
  ctx.closePath();
  ctx.fill();
}

function drawPlayer(ctx, s) {
  const p = s.player;
  if (s.invuln > 0 && Math.floor(s.invuln * 12) % 2 === 0) return;   // blink
  const x = p.x, top = FLOOR_Y - p.h;
  // Legs.
  ctx.fillStyle = PALETTE.diverDk;
  ctx.fillRect((x - 7) | 0, FLOOR_Y - 9, 6, 9);
  ctx.fillRect((x + 1) | 0, FLOOR_Y - 9, 6, 9);
  // Body.
  ctx.fillStyle = PALETTE.diver;
  ctx.fillRect((x - p.w / 2) | 0, top, p.w, p.h - 8);
  ctx.fillStyle = PALETTE.diverHi;
  ctx.fillRect((x - p.w / 2) | 0, top, 4, p.h - 8);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect((x - 6) | 0, (top - 11) | 0, 12, 11);
  ctx.fillStyle = PALETTE.diverDk;
  ctx.fillRect((x - 7) | 0, (top - 13) | 0, 14, 4);
  // The harpoon launcher mounted on top.
  ctx.fillStyle = PALETTE.wire;
  ctx.fillRect((x - 2) | 0, (top - 18) | 0, 4, 7);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, CEIL_Y);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 18);
  for (let i = 0; i <= s.lives; i++) drawHeart(ctx, 150 + i * 13, 18);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 8, 18);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(0.5, s.flash);
  ctx.fillStyle = s.over && s.won ? `rgba(95,208,122,${a})` : `rgba(255,80,80,${a})`;
  ctx.fillRect(0, CEIL_Y, VW, FLOOR_Y - CEIL_Y);
}

// A small bouncing orb + harpoon for the title screen.
function drawTitleArt(ctx, cx, cy) {
  drawOrb(ctx, { x: cx - 44, y: cy, r: ORB_R[3], size: 3 });
  drawOrb(ctx, { x: cx + 30, y: cy + 14, r: ORB_R[1], size: 1 });
  drawOrb(ctx, { x: cx + 56, y: cy - 20, r: ORB_R[0], size: 0 });
  ctx.fillStyle = PALETTE.wire;
  ctx.fillRect(cx + 2, cy - 6, 2, 56);
  ctx.fillStyle = PALETTE.spear;
  ctx.beginPath();
  ctx.moveTo(cx + 3, cy - 14); ctx.lineTo(cx - 2, cy - 4); ctx.lineTo(cx + 8, cy - 4);
  ctx.closePath(); ctx.fill();
}
