// Pixel-art rendering for Pixel Climb. 360x480 world units.

const PALETTE = {
  bg:       '#1a1224',
  bgHi:     '#2a1d3a',
  beam:     '#e85a3a',           // classic red I-beam
  beamHi:   '#ff8a6a',
  beamLo:   '#7a1e0c',
  ladder:   '#f4d27b',
  ladderHi: '#fff0c8',
  ladderLo: '#9a8048',
  barrel:   '#a06030',
  barrelHi: '#d8a560',
  barrelLo: '#5a2e10',
  barrelBand:'#3a1a08',
  player:   '#5fc0ff',
  playerHi: '#a8e0ff',
  playerLo: '#205a8a',
  player2:  '#e85a3a',           // hat / shirt accent
  goal:     '#ff7fb8',
  goalHi:   '#ffd0e0',
  dk:       '#7a3a14',
  dkHi:     '#a0552a',
  border:   '#070315',
  hud:      '#070315',
  hudText:  '#f8f5e8',
  hudDim:   '#a0a8b8',
  heart:    '#ff4a5a',
  win:      '#5fc06e',
  bad:      '#ff5a3a',
  ctrl:     '#28315c',
  ctrlHi:   '#3c4576',
  ctrlText: '#f8f5e8',
};

function drawBackdrop(ctx) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  ctx.fillStyle = PALETTE.bgHi;
  for (let i = 0; i < 28; i++) {
    const sx = (i * 53 + 7) % VW;
    const sy = (i * 71 + 17) % VH;
    ctx.fillRect(sx, sy, 2, 2);
  }
}

function drawTower(ctx, s) {
  // Beams (drawn back-to-front so ladders sit between).
  for (let i = 0; i < BEAMS; i++) drawBeam(ctx, BEAM_Y[i], i);
  // Ladders.
  for (const l of s.ladders) drawLadder(ctx, l);
  // Goal — a pink heart marker on the top beam left side.
  drawGoal(ctx, 30, BEAM_Y[0] - 22);
  // DK silhouette at the top right of the top beam.
  drawDK(ctx, VW - 50, BEAM_Y[0] - 24);
}

function drawBeam(ctx, y, i) {
  ctx.fillStyle = PALETTE.beamLo;
  ctx.fillRect(0, y - 1, VW, BEAM_THICK + 2);
  ctx.fillStyle = PALETTE.beam;
  ctx.fillRect(0, y, VW, BEAM_THICK);
  ctx.fillStyle = PALETTE.beamHi;
  ctx.fillRect(0, y, VW, 1);
  // Rivets so the beam reads as an I-beam.
  ctx.fillStyle = PALETTE.beamLo;
  for (let x = 8; x < VW; x += 18) {
    ctx.fillRect(x, y + 2, 2, 2);
  }
}

function drawLadder(ctx, l) {
  const W = 4;
  ctx.fillStyle = PALETTE.ladderLo;
  ctx.fillRect(l.x - 9, l.top, 2, l.bottom - l.top + BEAM_THICK);
  ctx.fillRect(l.x + 7, l.top, 2, l.bottom - l.top + BEAM_THICK);
  ctx.fillStyle = PALETTE.ladder;
  ctx.fillRect(l.x - 8, l.top, 2, l.bottom - l.top + BEAM_THICK);
  ctx.fillRect(l.x + 6, l.top, 2, l.bottom - l.top + BEAM_THICK);
  // Rungs every 8 px.
  for (let y = l.top + 4; y < l.bottom; y += 8) {
    ctx.fillStyle = PALETTE.ladderHi;
    ctx.fillRect(l.x - 6, y, 12, 2);
    ctx.fillStyle = PALETTE.ladderLo;
    ctx.fillRect(l.x - 6, y + 2, 12, 1);
  }
}

function drawGoal(ctx, x, y) {
  ctx.fillStyle = PALETTE.goal;
  ctx.fillRect(x - 6, y + 2, 4, 4);
  ctx.fillRect(x + 2, y + 2, 4, 4);
  ctx.fillRect(x - 6, y + 6, 12, 4);
  ctx.fillRect(x - 5, y + 10, 10, 3);
  ctx.fillRect(x - 3, y + 13, 6, 2);
  ctx.fillRect(x - 1, y + 15, 2, 1);
  ctx.fillStyle = PALETTE.goalHi;
  ctx.fillRect(x - 5, y + 3, 3, 2);
}

function drawDK(ctx, x, y) {
  ctx.fillStyle = PALETTE.dk;
  ctx.fillRect(x - 10, y, 20, 18);
  ctx.fillStyle = PALETTE.dkHi;
  ctx.fillRect(x - 10, y, 20, 3);
  ctx.fillStyle = '#1a1010';
  ctx.fillRect(x - 6, y + 4, 4, 4);
  ctx.fillRect(x + 2, y + 4, 4, 4);
  ctx.fillStyle = '#fff';
  ctx.fillRect(x - 5, y + 5, 2, 2);
  ctx.fillRect(x + 3, y + 5, 2, 2);
}

function drawBarrel(ctx, b, time) {
  const x = b.x, y = b.y;
  ctx.fillStyle = PALETTE.barrelLo;
  ctx.fillRect(x - BARREL_R, y - BARREL_R, BARREL_R * 2, BARREL_R * 2);
  ctx.fillStyle = PALETTE.barrel;
  ctx.fillRect(x - BARREL_R + 1, y - BARREL_R + 1, BARREL_R * 2 - 2, BARREL_R * 2 - 2);
  ctx.fillStyle = PALETTE.barrelHi;
  ctx.fillRect(x - BARREL_R + 1, y - BARREL_R + 1, BARREL_R * 2 - 2, 2);
  // Bands rotate with motion to suggest a roll.
  const spin = Math.floor((b.age * (b.state === 'roll' ? 14 : 4))) % 4;
  ctx.fillStyle = PALETTE.barrelBand;
  for (let i = 0; i < 4; i++) {
    if ((i + spin) % 2 === 0) continue;
    ctx.fillRect(x - BARREL_R + 1, y - BARREL_R + 2 + i * 3, BARREL_R * 2 - 2, 1);
  }
}

function drawPlayer(ctx, p) {
  if (p.state === 'dead' && p.respawn > 0) return;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return;
  const x = p.x, y = p.y;
  // body
  ctx.fillStyle = PALETTE.playerLo;
  ctx.fillRect(x, y + 1, PLAYER_W, PLAYER_H);
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(x + 1, y + 1, PLAYER_W - 2, PLAYER_H - 2);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(x + 1, y + 1, PLAYER_W - 2, 3);
  // shirt accent
  ctx.fillStyle = PALETTE.player2;
  ctx.fillRect(x + 2, y + 8, PLAYER_W - 4, 4);
  // hat
  ctx.fillStyle = PALETTE.player2;
  ctx.fillRect(x + 1, y - 2, PLAYER_W - 2, 3);
  // eye dot facing direction
  ctx.fillStyle = '#0a0a18';
  ctx.fillRect(x + (p.face > 0 ? PLAYER_W - 4 : 2), y + 4, 2, 2);
}

function drawHud(ctx, lang, s, best) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, VW, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 6, 16);
  for (let i = 0; i < Math.max(0, s.lives + 1); i++) drawHeart(ctx, 130 + i * 12, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'score') + ' ' + s.score, VW - 6, 16);
}

function drawHeart(ctx, cx, cy) {
  ctx.fillStyle = PALETTE.heart;
  ctx.fillRect(cx - 4, cy - 3, 3, 3);
  ctx.fillRect(cx + 1, cy - 3, 3, 3);
  ctx.fillRect(cx - 4, cy, 8, 2);
  ctx.fillRect(cx - 3, cy + 2, 6, 1);
  ctx.fillRect(cx - 1, cy + 3, 2, 1);
}

// On-screen controls — five touch zones in a strip at the bottom (L / R /
// UP / DOWN / JUMP). Each is also exposed as a hit-test rect for game.js.
function controlRects() {
  const y = VH - 56, w = 64, h = 48, gap = 6;
  const total = w * 5 + gap * 4;
  const x0 = ((VW - total) / 2) | 0;
  return {
    left:  { x: x0,                       y, w, h, label: '←' },
    down:  { x: x0 + (w + gap),           y, w, h, label: '↓' },
    up:    { x: x0 + (w + gap) * 2,       y, w, h, label: '↑' },
    right: { x: x0 + (w + gap) * 3,       y, w, h, label: '→' },
    jump:  { x: x0 + (w + gap) * 4,       y, w, h, label: '✦' },
  };
}

function drawControls(ctx, input) {
  const rs = controlRects();
  for (const key of Object.keys(rs)) {
    const r = rs[key];
    const hot = (key !== 'jump' && input && input[key]);
    ctx.fillStyle = hot ? PALETTE.ctrlHi : PALETTE.ctrl;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    ctx.fillStyle = '#3c4576';
    ctx.fillRect(r.x, r.y, r.w, 2);
    ctx.fillStyle = PALETTE.ctrlText;
    ctx.font = 'bold 20px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(r.label, r.x + r.w / 2, r.y + r.h / 2);
    ctx.textBaseline = 'alphabetic';
  }
}

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(255,221,90,${0.35 * a})` :
                  s.player.state === 'dead' ? `rgba(255,80,80,${0.5 * a})` :
                                              `rgba(255,255,255,${0.18 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
