// Pixel Word Hunt - backdrop and grid rendering.

const WH_HUES = ['#e8554f', '#4a9be8', '#5fc06e', '#9a6cd8',
                 '#4fd6d6', '#ff7db0', '#a8d84a', '#ef9b3e'];

function drawBackground(ctx) {
  const g = ctx.createLinearGradient(0, 0, 0, VH);
  g.addColorStop(0, '#2c2616');
  g.addColorStop(1, '#0e0b07');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, VW, VH);
}

// One letter tile. bg tints by state: found-word colour, live selection, plain.
function drawLetter(ctx, x, y, s, letter, bg, fg) {
  ctx.fillStyle = bg;
  ctx.fillRect(x + 1, y + 1, s - 2, s - 2);
  ctx.fillStyle = fg;
  ctx.font = '900 ' + Math.round(s * 0.52) + 'px ui-monospace, monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(letter, x + s / 2, y + s / 2 + 1);
}
