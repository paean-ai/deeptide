// Pixel Keg - rendering + on-screen controls.

const COL = {
  floorA: '#2d2b22', floorB: '#26241c', wallTop: '#7e6b4a', wallSide: '#52432a',
  brickA: '#a05030', brickB: '#7a3a22', brickEdge: '#3e1c10',
  bomb: '#1d1a14', bombHi: '#ffd86b', bombFlash: '#ff6e3a',
  flame: '#ff7a3a', flameHot: '#ffd86b', flameDim: '#aa3a10',
  exit: '#9aff7c', exitGlow: 'rgba(154,255,124,0.5)',
  player: '#5fcfd6', playerDark: '#1f5e63', playerEye: '#0c1014',
  enemy: '#c84a8a', enemyDark: '#6b1f43', enemyEye: '#fff',
  pad: '#3b3358', padOn: '#ffd86b', padIco: '#e8ecf6',
  bombBtn: '#ff6e3a', bombBtnDark: '#7a2810',
};
const PAD = {
  up:    { x: 44, y: 372, w: 34, h: 34, dir: 0 },
  left:  { x: 8,  y: 408, w: 34, h: 34, dir: 3 },
  right: { x: 80, y: 408, w: 34, h: 34, dir: 1 },
  down:  { x: 44, y: 444, w: 34, h: 34, dir: 2 },
};
const BOMB_BTN = { x: 222, y: 392, w: 90, h: 62 };

function drawBackground(ctx, flash) {
  ctx.fillStyle = '#0a0814';
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
  if (flash > 0) {
    ctx.fillStyle = `rgba(255,140,80,${flash * 0.5})`;
    ctx.fillRect(0, 0, VW, VH);
  }
}

function boardGeom() {
  const cell = 26;
  return { cell, gx: Math.round((VW - cell * N) / 2), gy: 50 };
}
function tileXY(g, i) {
  return { x: g.gx + (i % N) * g.cell, y: g.gy + ((i / N) | 0) * g.cell };
}

function drawBoard(ctx, s, geom, clock) {
  const { cell } = geom;
  // tiles
  for (let i = 0; i < N * N; i++) {
    const p = tileXY(geom, i);
    const t = s.grid[i];
    if (t === WALL) drawWall(ctx, p.x, p.y, cell);
    else {
      const r = (i / N) | 0, c = i % N;
      ctx.fillStyle = (r + c) % 2 ? COL.floorA : COL.floorB;
      ctx.fillRect(p.x, p.y, cell, cell);
      if (t === BRICK) drawBrick(ctx, p.x, p.y, cell);
    }
  }
  // exit (if revealed) on the floor under it
  if (s.exitRevealed) {
    const p = tileXY(geom, s.exit);
    const pulse = 0.5 + 0.5 * Math.sin(clock * 4);
    ctx.fillStyle = `rgba(154,255,124,${0.3 + pulse * 0.4})`;
    ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
    ctx.fillStyle = COL.exit;
    ctx.fillRect(p.x + cell / 2 - 4, p.y + 6, 8, cell - 12);
    ctx.fillRect(p.x + 6, p.y + cell / 2 - 4, cell - 12, 8);
  }
  // bombs
  for (const b of s.bombs) {
    const p = tileXY(geom, b.idx);
    const tick = 1 - b.fuse / 2.0;
    const pulse = 0.6 + 0.4 * Math.sin(clock * (4 + tick * 12));
    ctx.fillStyle = COL.bomb;
    ctx.beginPath();
    ctx.arc(p.x + cell / 2, p.y + cell / 2 + 1, cell * 0.36, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = b.fuse < 0.5 ? COL.bombFlash : COL.bombHi;
    ctx.globalAlpha = pulse;
    ctx.beginPath();
    ctx.arc(p.x + cell / 2 + cell * 0.18, p.y + cell / 2 - cell * 0.18, 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalAlpha = 1;
  }
  // flames
  for (const f of s.flames) {
    const p = tileXY(geom, f.idx);
    const a = Math.min(1, f.life / 0.42);
    ctx.fillStyle = COL.flameDim;
    ctx.globalAlpha = a * 0.8;
    ctx.fillRect(p.x + 1, p.y + 1, cell - 2, cell - 2);
    ctx.fillStyle = COL.flame;
    ctx.globalAlpha = a;
    ctx.fillRect(p.x + 4, p.y + 4, cell - 8, cell - 8);
    ctx.fillStyle = COL.flameHot;
    ctx.fillRect(p.x + cell / 2 - 3, p.y + cell / 2 - 3, 6, 6);
    ctx.globalAlpha = 1;
  }
  // enemies
  for (const e of s.enemies) {
    if (!e.alive) continue;
    const p = tileXY(geom, e.idx);
    drawCreature(ctx, p.x, p.y, cell, COL.enemy, COL.enemyDark, COL.enemyEye);
  }
  // player
  if (s.player.alive) {
    const p = tileXY(geom, s.player.idx);
    drawCreature(ctx, p.x, p.y, cell, COL.player, COL.playerDark, COL.playerEye);
  }
}

function drawWall(ctx, x, y, cell) {
  ctx.fillStyle = COL.wallSide;
  ctx.fillRect(x, y, cell, cell);
  ctx.fillStyle = COL.wallTop;
  ctx.fillRect(x + 2, y + 2, cell - 4, cell - 5);
  ctx.fillStyle = COL.wallSide;
  ctx.fillRect(x + 4, y + 4, 3, 3);
  ctx.fillRect(x + cell - 7, y + cell - 9, 3, 3);
}
function drawBrick(ctx, x, y, cell) {
  ctx.fillStyle = COL.brickEdge;
  ctx.fillRect(x + 1, y + 1, cell - 2, cell - 2);
  ctx.fillStyle = COL.brickB;
  ctx.fillRect(x + 2, y + 2, cell - 4, cell - 5);
  ctx.fillStyle = COL.brickA;
  ctx.fillRect(x + 3, y + 3, cell - 8, 3);
  ctx.fillRect(x + 3, y + cell / 2, cell - 8, 3);
}
function drawCreature(ctx, x, y, cell, main, dark, eye) {
  const m = 3, sz = cell - m * 2;
  ctx.fillStyle = dark;
  ctx.fillRect(x + m, y + m + sz - 3, sz, 3);
  ctx.fillStyle = main;
  ctx.fillRect(x + m, y + m, sz, sz - 3);
  ctx.fillStyle = eye;
  ctx.fillRect(x + m + sz * 0.22, y + m + sz * 0.3, 4, 4);
  ctx.fillRect(x + m + sz * 0.6, y + m + sz * 0.3, 4, 4);
}

function drawControls(ctx, pressed, bombPressed) {
  for (const k in PAD) {
    const b = PAD[k];
    ctx.fillStyle = pressed[k] ? COL.padOn : COL.pad;
    ctx.fillRect(b.x, b.y, b.w, b.h);
    ctx.strokeStyle = '#1a1a26';
    ctx.lineWidth = 1;
    ctx.strokeRect(b.x + 0.5, b.y + 0.5, b.w - 1, b.h - 1);
    ctx.fillStyle = COL.padIco;
    ctx.beginPath();
    const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
    if (k === 'up') { ctx.moveTo(cx, cy - 8); ctx.lineTo(cx - 8, cy + 6); ctx.lineTo(cx + 8, cy + 6); }
    if (k === 'down') { ctx.moveTo(cx, cy + 8); ctx.lineTo(cx - 8, cy - 6); ctx.lineTo(cx + 8, cy - 6); }
    if (k === 'left') { ctx.moveTo(cx - 8, cy); ctx.lineTo(cx + 6, cy - 8); ctx.lineTo(cx + 6, cy + 8); }
    if (k === 'right') { ctx.moveTo(cx + 8, cy); ctx.lineTo(cx - 6, cy - 8); ctx.lineTo(cx - 6, cy + 8); }
    ctx.closePath();
    ctx.fill();
  }
  ctx.fillStyle = bombPressed ? '#ffe89a' : COL.bombBtn;
  ctx.fillRect(BOMB_BTN.x, BOMB_BTN.y, BOMB_BTN.w, BOMB_BTN.h);
  ctx.strokeStyle = COL.bombBtnDark;
  ctx.lineWidth = 2;
  ctx.strokeRect(BOMB_BTN.x + 1, BOMB_BTN.y + 1, BOMB_BTN.w - 2, BOMB_BTN.h - 2);
  ctx.fillStyle = '#1a0a06';
  ctx.font = 'bold 17px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('BOMB', BOMB_BTN.x + BOMB_BTN.w / 2, BOMB_BTN.y + BOMB_BTN.h / 2 + 1);
}
