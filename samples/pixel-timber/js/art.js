// Pixel-art rendering for Pixel Timber. 360x480 world units.

const PALETTE = {
  skyTop:   '#9bd6e6',
  skyLow:   '#d8f0e4',
  forest:   '#3f7a4e',
  forestDk: '#2c5d3b',
  ground:   '#6b4a2a',
  groundHi: '#7d5832',
  bark:     '#b07b3e',
  barkHi:   '#c89a5a',
  barkDk:   '#7c5226',
  ring:     '#8a5e30',
  ringHi:   '#e8c98a',
  limb:     '#7c5226',
  leaf:     '#4ea357',
  leafHi:   '#74c87e',
  leafDk:   '#327a44',
  skin:     '#e8b890',
  shirt:    '#d8453f',
  shirtHi:  '#f06a5a',
  pants:    '#34406a',
  boot:     '#2a2030',
  axeHead:  '#cdd6e0',
  axeEdge:  '#f2f6fb',
  haft:     '#6a4424',
  hud:      '#1d2a22',
  hudText:  '#f4f3e6',
  stamina:  '#5fd06a',
  staminaLo:'#e85b4a',
  staminaBg:'#13201a',
  accent:   '#f4c44a',
  shadow:   'rgba(0,0,0,0.22)',
};

const ART_BASE = 430;         // y of the bottom of trunk log 0
const SEG_DRAW = 7;           // logs drawn on screen
const TRUNK_HW = 32;          // trunk half-width

function segTop(i) { return ART_BASE - (i + 1) * SEG_H; }

function drawBackdrop(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, PALETTE.skyTop);
  g.addColorStop(1, PALETTE.skyLow);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
  // Far tree line.
  ctx.fillStyle = PALETTE.forestDk;
  for (let x = -10; x < VW + 20; x += 46) {
    ctx.beginPath();
    ctx.moveTo(x, 360); ctx.lineTo(x + 23, 286); ctx.lineTo(x + 46, 360);
    ctx.closePath(); ctx.fill();
  }
  ctx.fillStyle = PALETTE.forest;
  for (let x = 12; x < VW + 20; x += 46) {
    ctx.beginPath();
    ctx.moveTo(x, 392); ctx.lineTo(x + 25, 312); ctx.lineTo(x + 50, 392);
    ctx.closePath(); ctx.fill();
  }
  // Ground.
  ctx.fillStyle = PALETTE.ground;
  ctx.fillRect(0, 392, VW, VH - 392);
  ctx.fillStyle = PALETTE.groundHi;
  ctx.fillRect(0, 392, VW, 4);
}

function drawTrunk(ctx, s) {
  const cx = VW / 2;
  for (let i = SEG_DRAW - 1; i >= 0; i--) {
    const seg = s.segments[i];
    if (!seg) continue;
    const top = segTop(i);
    drawLog(ctx, cx, top, seg.branch, i === 0);
  }
  // A stump under log 0.
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(cx - TRUNK_HW - 2, ART_BASE, TRUNK_HW * 2 + 4, 6);
}

function drawLog(ctx, cx, top, branch, isBottom) {
  const x = cx - TRUNK_HW;
  // Bark body.
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(x, top, TRUNK_HW * 2, SEG_H);
  ctx.fillStyle = PALETTE.bark;
  ctx.fillRect(x + 2, top, TRUNK_HW * 2 - 4, SEG_H);
  ctx.fillStyle = PALETTE.barkHi;
  ctx.fillRect(x + 4, top, 4, SEG_H);
  // Bark grain.
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(x + 18, top + 8, 2, SEG_H - 16);
  ctx.fillRect(x + 40, top + 4, 2, SEG_H - 10);
  // Seam between logs.
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(x, top + SEG_H - 2, TRUNK_HW * 2, 2);
  if (isBottom) {
    // End-grain rings on the log that is next to be chopped.
    ctx.fillStyle = PALETTE.ring;
    ctx.fillRect(x + 6, top + 10, TRUNK_HW * 2 - 12, SEG_H - 20);
    ctx.fillStyle = PALETTE.ringHi;
    ctx.fillRect(x + 14, top + 18, TRUNK_HW * 2 - 28, SEG_H - 36);
    ctx.fillStyle = PALETTE.ring;
    ctx.fillRect(cx - 3, top + SEG_H / 2 - 3, 6, 6);
  }
  if (branch !== BR_NONE) drawBranch(ctx, cx, top + SEG_H / 2, branch);
}

function drawBranch(ctx, cx, midY, branch) {
  const dir = branch === BR_LEFT ? -1 : 1;
  const rootX = cx + dir * TRUNK_HW;
  // Woody limb.
  ctx.fillStyle = PALETTE.limb;
  ctx.fillRect(Math.min(rootX, rootX + dir * 40), midY - 5, 40, 9);
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(Math.min(rootX, rootX + dir * 40), midY + 2, 40, 2);
  // Leaf clump at the tip.
  const tipX = rootX + dir * 44;
  ctx.fillStyle = PALETTE.leafDk;
  ctx.fillRect(tipX - 14, midY - 17, 28, 32);
  ctx.fillStyle = PALETTE.leaf;
  ctx.fillRect(tipX - 12, midY - 15, 24, 26);
  ctx.fillStyle = PALETTE.leafHi;
  ctx.fillRect(tipX - 10, midY - 13, 9, 7);
  ctx.fillRect(tipX + 1, midY - 2, 7, 6);
}

function drawLumberjack(ctx, s) {
  const onLeft = s.side === SIDE_LEFT;
  const cx = VW / 2;
  const x = cx + (onLeft ? -TRUNK_HW - 30 : TRUNK_HW + 30);
  const face = onLeft ? 1 : -1;          // +1 looks right (toward trunk)
  const y = 392;                          // feet on the ground
  // Shadow.
  ctx.fillStyle = PALETTE.shadow;
  ctx.fillRect(x - 14, y - 3, 28, 5);
  const chopping = s.chopT > 0;
  // Legs.
  ctx.fillStyle = PALETTE.pants;
  ctx.fillRect(x - 8, y - 16, 7, 16);
  ctx.fillRect(x + 1, y - 16, 7, 16);
  ctx.fillStyle = PALETTE.boot;
  ctx.fillRect(x - 9, y - 4, 9, 4);
  ctx.fillRect(x + 1, y - 4, 9, 4);
  // Torso.
  ctx.fillStyle = PALETTE.shirt;
  ctx.fillRect(x - 10, y - 36, 20, 21);
  ctx.fillStyle = PALETTE.shirtHi;
  ctx.fillRect(x - 10, y - 36, 5, 21);
  // Head.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect(x - 6, y - 50, 13, 13);
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(x - 7, y - 53, 15, 4);   // hair / cap
  ctx.fillStyle = '#1d1518';
  ctx.fillRect(x + (face > 0 ? 2 : -3) + 1, y - 45, 2, 2);  // eye
  // Arms + axe: raised when idle, swung through when chopping.
  ctx.save();
  ctx.translate(x + face * 7, y - 30);
  ctx.rotate(face * (chopping ? 0.95 : -0.55));
  // Arm.
  ctx.fillStyle = PALETTE.skin;
  ctx.fillRect(-3, -3, 16, 6);
  // Axe haft.
  ctx.fillStyle = PALETTE.haft;
  ctx.fillRect(10, -2, 26, 4);
  // Axe head.
  ctx.fillStyle = PALETTE.axeHead;
  ctx.fillRect(32, -9, 12, 18);
  ctx.fillStyle = PALETTE.axeEdge;
  ctx.fillRect(42, -9, 3, 18);
  ctx.restore();
}

function drawFlyLog(ctx, s) {
  const f = s.flyLog;
  if (!f) return;
  ctx.save();
  ctx.translate(f.x, f.y || 410);
  ctx.rotate(f.rot);
  ctx.fillStyle = PALETTE.barkDk;
  ctx.fillRect(-26, -16, 52, 32);
  ctx.fillStyle = PALETTE.bark;
  ctx.fillRect(-24, -14, 48, 28);
  ctx.fillStyle = PALETTE.ring;
  ctx.fillRect(-10, -10, 20, 20);
  ctx.restore();
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 30);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 11px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText(t(lang, 'best') + ' ' + best, 8, 15);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score'), VW - 8, 15);
  // Stamina bar - drains from both ends like the arcade original.
  const bw = 220, bx = (VW - bw) / 2, by = 44;
  ctx.fillStyle = PALETTE.staminaBg;
  ctx.fillRect(bx - 3, by - 3, bw + 6, 18);
  const fillW = Math.max(0, Math.min(1, s.stamina)) * bw;
  ctx.fillStyle = s.stamina < 0.3 ? PALETTE.staminaLo : PALETTE.stamina;
  ctx.fillRect(bx + (bw - fillW) / 2, by, fillW, 12);
  ctx.fillStyle = 'rgba(255,255,255,0.25)';
  ctx.fillRect(bx + (bw - fillW) / 2, by, fillW, 3);
  // Big score.
  ctx.fillStyle = PALETTE.hud;
  ctx.font = 'bold 34px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillText(String(s.score), VW / 2, 96);
}
