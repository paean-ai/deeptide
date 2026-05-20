// Pixel-art rendering for Pixel Knife. 360x480 world units.

const PALETTE = {
  bg:         '#1d2240',
  bgFloor:    '#262d54',
  wood:       '#a36835',
  woodDark:   '#7d4f29',
  woodGrain:  '#5a3819',
  knifeBlade: '#dde6f2',
  knifeEdge:  '#a8b1c0',
  knifeHilt:  '#3d2a18',
  knifeBoss:  '#7c5a30',
  apple:      '#e8554f',
  appleStem:  '#3a2a18',
  appleLeaf:  '#5fc06e',
  hud:        '#0d1228',
  hudText:    '#f8f5e8',
  hudDim:     '#9aa6cc',
  ok:         '#54c47c',
  warn:       '#f7e69a',
  bad:        '#e8554f',
};

function drawScene(ctx, s, viewW, viewH) {
  // Background.
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, viewW, viewH);
  // Floor band where unused knives wait.
  ctx.fillStyle = PALETTE.bgFloor;
  ctx.fillRect(0, viewH - 60, viewW, 60);
  drawDisk(ctx, s);
  drawApples(ctx, s);
  drawStuckKnives(ctx, s);
  drawFlyingKnife(ctx, s);
  drawQueue(ctx, s);
  drawHud(ctx, s);
}

function drawDisk(ctx, s) {
  // Wood disk with grain rings.
  ctx.fillStyle = PALETTE.woodDark;
  ctx.beginPath();
  ctx.arc(180, 170, 66, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = PALETTE.wood;
  ctx.beginPath();
  ctx.arc(180, 170, 62, 0, Math.PI * 2);
  ctx.fill();
  // Grain.
  ctx.strokeStyle = PALETTE.woodGrain;
  ctx.lineWidth = 1;
  for (let r = 16; r < 60; r += 12) {
    ctx.beginPath();
    ctx.arc(180, 170, r, 0, Math.PI * 2);
    ctx.stroke();
  }
  // Bullseye dot in centre tied to disk rotation so spin is readable.
  ctx.save();
  ctx.translate(180, 170);
  ctx.rotate(s.disk.angle);
  ctx.fillStyle = PALETTE.woodGrain;
  ctx.fillRect(-3, -3, 6, 6);
  ctx.fillRect(-1, -22, 2, 8);
  ctx.restore();
}

function drawApples(ctx, s) {
  for (const a of s.apples) {
    if (!a.alive) continue;
    const ang = s.disk.angle + a.relAngle - Math.PI / 2;   // local
    const r = 46;
    const ax = 180 + Math.cos(ang) * r;
    const ay = 170 + Math.sin(ang) * r;
    // Body.
    ctx.fillStyle = PALETTE.apple;
    ctx.beginPath();
    ctx.arc(ax | 0, ay | 0, 7, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#fff';
    ctx.fillRect((ax - 2) | 0, (ay - 2) | 0, 1, 1);
    // Stem + leaf.
    ctx.fillStyle = PALETTE.appleStem;
    ctx.fillRect((ax) | 0, (ay - 9) | 0, 1, 3);
    ctx.fillStyle = PALETTE.appleLeaf;
    ctx.fillRect((ax + 1) | 0, (ay - 8) | 0, 3, 2);
  }
}

function drawStuckKnives(ctx, s) {
  for (const k of s.stuck) {
    const ang = s.disk.angle + k.relAngle - Math.PI / 2;   // -π/2 puts 0 at top
    const r = 64;
    const tipX = 180 + Math.cos(ang) * r;
    const tipY = 170 + Math.sin(ang) * r;
    drawKnife(ctx, tipX, tipY, ang);
  }
}

function drawFlyingKnife(ctx, s) {
  if (!s.flying) return;
  // Flies straight upward; tip at flying.y, knife rotated so it points up.
  drawKnife(ctx, 180, s.flying.y, -Math.PI / 2);
}

// Draw a single knife with its tip at (tipX, tipY) pointing along angle.
function drawKnife(ctx, tipX, tipY, angle) {
  ctx.save();
  ctx.translate(tipX, tipY);
  ctx.rotate(angle + Math.PI / 2);    // sprite is oriented top-up locally
  // Sprite local coords: tip at (0, 0), blade extending down (+y), hilt below.
  // Blade.
  ctx.fillStyle = PALETTE.knifeEdge;
  ctx.fillRect(-3,  0,  6, 36);
  ctx.fillStyle = PALETTE.knifeBlade;
  ctx.fillRect(-2,  0,  4, 36);
  ctx.fillStyle = '#fff';
  ctx.fillRect(-1,  4,  1, 18);
  // Guard.
  ctx.fillStyle = PALETTE.knifeBoss;
  ctx.fillRect(-5, 36, 10, 4);
  // Hilt.
  ctx.fillStyle = PALETTE.knifeHilt;
  ctx.fillRect(-3, 40, 6, 14);
  ctx.fillStyle = PALETTE.knifeBoss;
  ctx.fillRect(-3, 52, 6, 2);
  ctx.restore();
}

function drawQueue(ctx, s) {
  // Tiny knife icons at the bottom showing remaining throws.
  const total = s.cfg.knives;
  const w = 14;
  const startX = 180 - (total * w) / 2 + w / 2;
  const y = 430;
  for (let i = 0; i < total; i++) {
    const launched = i < (s.cfg.knives - s.queue);
    const stuck = i < s.thrown;
    const x = startX + i * w;
    ctx.fillStyle = stuck ? PALETTE.ok : (launched ? PALETTE.bad : PALETTE.knifeBlade);
    ctx.fillRect((x - 2) | 0, y - 16, 4, 22);
    ctx.fillStyle = PALETTE.knifeHilt;
    ctx.fillRect((x - 2) | 0, y + 6, 4, 6);
  }
}

function drawHud(ctx, s) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText(`${s.thrown}/${s.cfg.knives}`, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(`SCORE ${s.score}`, 352, 16);
}
