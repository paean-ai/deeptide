// Pixel-art rendering for Pixel Plinko. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  board:     '#1c2240',
  boardEdge: '#262d54',
  peg:       '#dde6ff',
  pegDark:   '#9aa6cc',
  ball:      '#e8554f',
  ballDark:  '#a8373a',
  ballHi:    '#fff',
  slotBg:    '#262d54',
  slotEdge:  '#0c1230',
  slotLo:    '#9aa6cc',
  slotMid:   '#f7e69a',
  slotHi:    '#54c47c',
  slotTop:   '#e8554f',
  trail:     'rgba(247, 230, 154, 0.45)',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function drawScene(ctx, s, lang) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, 360, 480);
  drawBoard(ctx);
  drawPegs(ctx, s);
  drawSlots(ctx, s);
  drawLanded(ctx, s);
  drawBall(ctx, s);
  drawHud(ctx, s, lang);
}

function drawBoard(ctx) {
  ctx.fillStyle = PALETTE.boardEdge;
  ctx.fillRect(BOARD_X0 - 4, BOARD_Y0 - 4, (BOARD_X1 - BOARD_X0) + 8, (SLOT_Y - BOARD_Y0) + 56);
  ctx.fillStyle = PALETTE.board;
  ctx.fillRect(BOARD_X0, BOARD_Y0, BOARD_X1 - BOARD_X0, SLOT_Y - BOARD_Y0);
}

function drawPegs(ctx, s) {
  for (const p of s.pegs) {
    ctx.fillStyle = PALETTE.pegDark;
    ctx.beginPath(); ctx.arc(p.x, p.y, 5, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = PALETTE.peg;
    ctx.beginPath(); ctx.arc(p.x, p.y, 4, 0, Math.PI * 2); ctx.fill();
  }
}

function drawSlots(ctx, s) {
  const nSlots = s.cfg.slotValues.length;
  const slotW = (BOARD_X1 - BOARD_X0) / nSlots;
  // Background.
  ctx.fillStyle = PALETTE.slotBg;
  ctx.fillRect(BOARD_X0, SLOT_Y, BOARD_X1 - BOARD_X0, 50);
  // Walls.
  ctx.fillStyle = PALETTE.slotEdge;
  for (let i = 0; i <= nSlots; i++) {
    const x = BOARD_X0 + i * slotW;
    ctx.fillRect(x - 1, SLOT_Y, 2, 50);
  }
  // Slot values colour-coded by magnitude.
  const max = Math.max(...s.cfg.slotValues);
  ctx.font = 'bold 11px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (let i = 0; i < nSlots; i++) {
    const v = s.cfg.slotValues[i];
    let col = PALETTE.slotLo;
    if (v === max && v > 0)        col = PALETTE.slotTop;
    else if (v >= max * 0.5)       col = PALETTE.slotHi;
    else if (v >= max * 0.2)       col = PALETTE.slotMid;
    ctx.fillStyle = col;
    ctx.fillText(String(v), BOARD_X0 + (i + 0.5) * slotW, SLOT_Y + 28);
  }
}

function drawLanded(ctx, s) {
  // Show ghost trails of where past balls landed for visual feedback.
  for (const l of s.landed) {
    ctx.fillStyle = PALETTE.trail;
    ctx.beginPath(); ctx.arc(l.x, SLOT_Y + 38, 3, 0, Math.PI * 2); ctx.fill();
  }
}

function drawBall(ctx, s) {
  if (!s.ball) return;
  const b = s.ball;
  ctx.fillStyle = PALETTE.ballDark;
  ctx.beginPath(); ctx.arc(b.x, b.y, 7, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.ball;
  ctx.beginPath(); ctx.arc(b.x, b.y, 6, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = PALETTE.ballHi;
  ctx.fillRect((b.x - 2) | 0, (b.y - 3) | 0, 1, 1);
}

function drawHud(ctx, s, lang) {
  ctx.fillStyle = PALETTE.hud;
  ctx.fillRect(0, 0, 360, 32);
  ctx.fillStyle = PALETTE.hudText;
  ctx.font = 'bold 12px monospace';
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'left';
  ctx.fillText('L' + (s.levelIndex + 1) + ' ' + s.cfg.name[0], 8, 16);
  ctx.textAlign = 'center';
  ctx.fillText(t(lang, 'score') + ' ' + s.score + ' / ' + s.cfg.target, 180, 16);
  ctx.textAlign = 'right';
  // Ball pips.
  for (let i = 0; i < BALLS_PER_ROUND; i++) {
    ctx.fillStyle = (i < (BALLS_PER_ROUND - s.ballsLeft - (s.ball ? 1 : 0))) ? PALETTE.ok : (i < BALLS_PER_ROUND - s.ballsLeft) ? PALETTE.ball : '#3a4274';
    ctx.fillRect(258 + i * 9, 10, 6, 12);
  }
}
