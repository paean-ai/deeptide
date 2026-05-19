// Pixel Vanguard - rendering.

const COL = {
  tileA: '#26324a', tileB: '#222d42', grid: '#161d2e',
  wall: '#11151f', wallEdge: '#3a4256',
  building: '#5fc8e8', buildingDim: '#2a5566',
  hero: '#5fd36e', heroDone: '#3a6e44', heroEye: '#0c1a10',
  enemy: '#ff6e7a', enemyEye: '#2a0c10',
  reach: 'rgba(95,211,110,0.26)', target: 'rgba(255,110,122,0.42)',
  sel: '#ffe07a', danger: 'rgba(255,90,90,0.34)', dangerEdge: '#ff5a5a',
  hpBack: '#0c0f18', hpHero: '#5fd36e', hpEnemy: '#ff6e7a',
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1a2236');
  g.addColorStop(1, '#0b0e18');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let y = 0; y < VH; y += 6) ctx.fillRect(0, y, VW, 1);
}

function boardGeom(s) {
  const cell = 48;
  return { cell, gx: Math.round((VW - cell * s.w) / 2), gy: 104 };
}
function tileXY(geom, r, c) {
  return { x: geom.gx + c * geom.cell, y: geom.gy + r * geom.cell };
}

function drawBoard(ctx, s, geom, ui) {
  const { cell } = geom;
  // tiles
  for (let r = 0; r < s.h; r++) for (let c = 0; c < s.w; c++) {
    const p = tileXY(geom, r, c);
    ctx.fillStyle = (r + c) % 2 ? COL.tileA : COL.tileB;
    ctx.fillRect(p.x, p.y, cell, cell);
    ctx.strokeStyle = COL.grid;
    ctx.lineWidth = 1;
    ctx.strokeRect(p.x + 0.5, p.y + 0.5, cell - 1, cell - 1);
  }
  // reachable / target overlays
  if (ui.reach) for (const k in ui.reach) {
    const [r, c] = k.split(',').map(Number);
    const p = tileXY(geom, r, c);
    ctx.fillStyle = COL.reach;
    ctx.fillRect(p.x + 3, p.y + 3, cell - 6, cell - 6);
  }
  // danger telegraphs
  for (const e of s.enemies) {
    if (!e.facing) continue;
    const tr = e.r + e.facing.dr, tc = e.c + e.facing.dc;
    if (tr < 0 || tc < 0 || tr >= s.h || tc >= s.w) continue;
    const p = tileXY(geom, tr, tc);
    ctx.fillStyle = COL.danger;
    ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
    ctx.strokeStyle = COL.dangerEdge;
    ctx.lineWidth = 2;
    ctx.strokeRect(p.x + 3.5, p.y + 3.5, cell - 7, cell - 7);
  }
  // walls
  for (const [r, c] of s.walls) {
    const p = tileXY(geom, r, c);
    ctx.fillStyle = COL.wall;
    ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
    ctx.strokeStyle = COL.wallEdge;
    ctx.strokeRect(p.x + 3.5, p.y + 3.5, cell - 7, cell - 7);
  }
  // buildings
  for (const b of s.buildings) {
    const p = tileXY(geom, b.r, b.c);
    ctx.fillStyle = s.core > 0 ? COL.building : COL.buildingDim;
    ctx.fillRect(p.x + 6, p.y + 6, cell - 12, cell - 12);
    ctx.fillStyle = COL.hpBack;
    ctx.fillRect(p.x + 12, p.y + 12, cell - 24, cell - 24);
  }
  // attack-target rings
  if (ui.targets) for (const id of ui.targets) {
    const e = s.enemies.find(x => x.id === id);
    if (!e) continue;
    const p = tileXY(geom, e.r, e.c);
    ctx.fillStyle = COL.target;
    ctx.fillRect(p.x + 2, p.y + 2, cell - 4, cell - 4);
  }
  // enemies
  for (const e of s.enemies) drawUnit(ctx, geom, e, false, e.facing);
  // heroes
  for (const h of s.heroes) {
    drawUnit(ctx, geom, h, true, null, ui.selected === h.id, h.acted);
  }
}

function drawUnit(ctx, geom, u, isHero, facing, selected, done) {
  const { cell } = geom;
  const p = tileXY(geom, u.r, u.c);
  const x = p.x + 7, y = p.y + 7, sz = cell - 14;
  if (selected) {
    ctx.strokeStyle = COL.sel;
    ctx.lineWidth = 3;
    ctx.strokeRect(p.x + 2.5, p.y + 2.5, cell - 5, cell - 5);
  }
  ctx.fillStyle = isHero ? (done ? COL.heroDone : COL.hero) : COL.enemy;
  ctx.fillRect(x, y, sz, sz);
  // eyes
  ctx.fillStyle = isHero ? COL.heroEye : COL.enemyEye;
  ctx.fillRect(x + sz * 0.22, y + sz * 0.3, sz * 0.18, sz * 0.18);
  ctx.fillRect(x + sz * 0.6, y + sz * 0.3, sz * 0.18, sz * 0.18);
  // enemy facing pip
  if (facing) {
    ctx.fillStyle = COL.dangerEdge;
    const cx = x + sz / 2 + facing.dc * sz * 0.42;
    const cy = y + sz / 2 + facing.dr * sz * 0.42;
    ctx.fillRect(cx - 2, cy - 2, 4, 4);
  }
  // hp pips
  const hp = u.hp, mx = u.maxhp;
  const pw = sz / mx;
  for (let i = 0; i < mx; i++) {
    ctx.fillStyle = i < hp ? (isHero ? COL.hpHero : COL.hpEnemy) : COL.hpBack;
    ctx.fillRect(x + i * pw + 1, p.y + cell - 6, pw - 1.5, 3);
  }
}
