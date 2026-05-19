// Pixel Solitaire - felt backdrop and card rendering.

// 7x7 pixel bitmaps for the four suit pips.
const SUIT_PIPS = {
  spade: ['0001000', '0011100', '0111110', '1111111', '1111111', '0010100', '0011100'],
  heart: ['0110110', '1111111', '1111111', '1111111', '0111110', '0011100', '0001000'],
  diamond: ['0001000', '0011100', '0111110', '1111111', '0111110', '0011100', '0001000'],
  club: ['0011100', '0011100', '1101011', '1111111', '1111111', '0001000', '0011100'],
};

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#1c3a2a');
  g.addColorStop(1, '#0c1c14');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

function drawPip(ctx, suitId, cx, cy, px) {
  const bmp = SUIT_PIPS[suitId];
  const ox = Math.round(cx - 3.5 * px), oy = Math.round(cy - 3.5 * px);
  for (let r = 0; r < 7; r++) {
    for (let c = 0; c < 7; c++) {
      if (bmp[r][c] === '1') ctx.fillRect(ox + c * px, oy + r * px, px, px);
    }
  }
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

// An empty pile slot — a faint outline, optionally watermarked with a suit.
function drawSlot(ctx, x, y, suitId) {
  roundRect(ctx, x, y, CARD_W, CARD_H, 5);
  ctx.fillStyle = 'rgba(255, 255, 255, 0.05)';
  ctx.fill();
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.22)';
  ctx.lineWidth = 1;
  ctx.stroke();
  if (suitId) {
    ctx.fillStyle = 'rgba(255, 255, 255, 0.14)';
    drawPip(ctx, suitId, x + CARD_W / 2, y + CARD_H / 2, 3);
  }
}

function drawCard(ctx, card, x, y) {
  roundRect(ctx, x, y, CARD_W, CARD_H, 5);
  if (!card.faceUp) {
    ctx.fillStyle = '#2f4f8a';
    ctx.fill();
    ctx.strokeStyle = '#1a2c52';
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.fillStyle = '#4664a8';
    for (let r = y + 6; r < y + CARD_H - 4; r += 6) {
      for (let c = x + 6; c < x + CARD_W - 4; c += 6) ctx.fillRect(c, r, 3, 3);
    }
    return;
  }
  ctx.fillStyle = '#f5f1e4';
  ctx.fill();
  ctx.strokeStyle = '#9a9482';
  ctx.lineWidth = 1;
  ctx.stroke();
  const suit = SUITS[card.suit];
  const ink = suit.red ? '#d23b3b' : '#202028';
  ctx.fillStyle = ink;
  ctx.font = '900 14px ui-monospace, monospace';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  ctx.fillText(RANK_LABELS[card.rank], x + 4, y + 4);
  drawPip(ctx, suit.id, x + 9, y + 26, 2);
  drawPip(ctx, suit.id, x + CARD_W / 2, y + CARD_H - 18, 3);
}
