function drawPixelHero(ctx, x, y, flip, pulse) {
  ctx.save();
  ctx.translate(Math.round(x), Math.round(y));
  ctx.scale(flip ? -1 : 1, 1);
  ctx.fillStyle = '#101722';
  ctx.fillRect(-10, -24, 20, 30);
  ctx.fillStyle = '#6be8ff';
  ctx.fillRect(-6, -28, 12, 8);
  ctx.fillStyle = '#edf4ff';
  ctx.fillRect(-4, -26, 3, 2);
  ctx.fillRect(2, -26, 3, 2);
  ctx.fillStyle = '#34506b';
  ctx.fillRect(-9, -16, 18, 14);
  ctx.fillStyle = '#52dc88';
  ctx.fillRect(-6, -12, 12, 3);
  ctx.fillStyle = '#1d2837';
  ctx.fillRect(-8, 6, 6, 11);
  ctx.fillRect(2, 6, 6, 11);
  ctx.fillStyle = '#61e5ff';
  ctx.fillRect(10, -13, 14 + pulse, 4);
  ctx.fillRect(20 + pulse, -17, 4, 12);
  ctx.restore();
}

function drawEnemy(ctx, enemy, camX) {
  const x = Math.round(enemy.x - camX);
  const y = Math.round(enemy.y);
  ctx.fillStyle = '#05070b';
  ctx.fillRect(x - enemy.w / 2 + 2, y - enemy.h + 4, enemy.w, enemy.h);
  ctx.fillStyle = enemy.kind === 'drone' ? '#ad7dff' : '#ec5b56';
  ctx.fillRect(x - enemy.w / 2, y - enemy.h, enemy.w, enemy.h - 4);
  ctx.fillStyle = '#ffd36a';
  ctx.fillRect(x - 5, y - enemy.h + 7, 4, 3);
  ctx.fillRect(x + 2, y - enemy.h + 7, 4, 3);
  ctx.fillStyle = '#202a37';
  ctx.fillRect(x - enemy.w / 2 + 2, y - 8, enemy.w - 4, 5);
}

function drawSkillIcon(ctx, color, glyph) {
  ctx.clearRect(0, 0, 54, 54);
  ctx.fillStyle = '#080b11';
  ctx.fillRect(0, 0, 54, 54);
  ctx.fillStyle = color;
  const data = glyph || [
    '...11...',
    '..1111..',
    '.11..11.',
    '11111111',
    '.11..11.',
    '..1111..',
    '...11...',
  ];
  const s = 5;
  const ox = 7;
  const oy = 9;
  for (let y = 0; y < data.length; y++) {
    for (let x = 0; x < data[y].length; x++) {
      if (data[y][x] === '1') ctx.fillRect(ox + x * s, oy + y * s, s, s);
    }
  }
  ctx.fillStyle = 'rgba(255,255,255,0.28)';
  ctx.fillRect(4, 4, 46, 2);
}
