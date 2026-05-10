function svgGlyph(kind, color) {
  const stroke = '#0a0d13';
  const hi = '#f8fbff';
  const fill = color || '#8fa0b5';
  const parts = {
    dagger: `<rect x="14" y="4" width="4" height="18" fill="${hi}"/><rect x="12" y="20" width="8" height="4" fill="${stroke}"/><rect x="15" y="24" width="2" height="6" fill="${fill}"/>`,
    bow: `<path d="M10 4 C22 10 22 22 10 28" fill="none" stroke="${stroke}" stroke-width="4"/><path d="M10 4 C19 11 19 21 10 28" fill="none" stroke="${fill}" stroke-width="3"/><rect x="18" y="6" width="2" height="22" fill="${hi}"/>`,
    wand: `<rect x="14" y="5" width="4" height="23" fill="${stroke}"/><rect x="15" y="5" width="2" height="23" fill="${fill}"/><rect x="11" y="3" width="10" height="6" fill="${hi}"/>`,
    cannon: `<rect x="7" y="11" width="18" height="10" fill="${stroke}"/><rect x="9" y="12" width="15" height="7" fill="${fill}"/><rect x="5" y="20" width="7" height="5" fill="${stroke}"/><rect x="20" y="20" width="7" height="5" fill="${stroke}"/>`,
    cleaver: `<rect x="14" y="6" width="5" height="22" fill="${stroke}"/><rect x="6" y="5" width="14" height="12" fill="${fill}"/><rect x="8" y="7" width="8" height="4" fill="${hi}"/>`,
    shield: `<path d="M6 6 H26 V16 C26 23 20 28 16 30 C12 28 6 23 6 16 Z" fill="${stroke}"/><path d="M9 8 H23 V16 C23 21 19 25 16 27 C13 25 9 21 9 16 Z" fill="${fill}"/>`,
    armor: `<rect x="8" y="7" width="16" height="21" fill="${stroke}"/><rect x="11" y="8" width="10" height="18" fill="${fill}"/><rect x="12" y="12" width="8" height="3" fill="${hi}"/>`,
    herb: `<rect x="15" y="14" width="3" height="13" fill="${stroke}"/><rect x="8" y="7" width="10" height="9" fill="${fill}"/><rect x="17" y="8" width="9" height="8" fill="${fill}"/>`,
    coin: `<rect x="7" y="8" width="18" height="18" fill="${stroke}"/><rect x="9" y="10" width="14" height="14" fill="${fill}"/><rect x="14" y="12" width="4" height="10" fill="${hi}"/>`,
    magnet: `<path d="M8 7 H13 V20 H19 V7 H24 V22 C24 27 8 27 8 22 Z" fill="${stroke}"/><path d="M10 9 H13 V20 H19 V9 H22 V21 C22 24 10 24 10 21 Z" fill="${fill}"/>`,
    contract: `<rect x="8" y="5" width="17" height="23" fill="${stroke}"/><rect x="10" y="7" width="13" height="19" fill="${fill}"/><rect x="12" y="11" width="8" height="2" fill="${hi}"/><rect x="12" y="17" width="7" height="2" fill="${hi}"/>`,
    battery: `<rect x="8" y="8" width="16" height="17" fill="${stroke}"/><rect x="12" y="5" width="8" height="3" fill="${stroke}"/><rect x="10" y="10" width="12" height="13" fill="${fill}"/><rect x="15" y="11" width="3" height="10" fill="${hi}"/>`,
    gear: `<rect x="13" y="4" width="6" height="24" fill="${stroke}"/><rect x="4" y="13" width="24" height="6" fill="${stroke}"/><rect x="9" y="9" width="14" height="14" fill="${fill}"/><rect x="13" y="13" width="6" height="6" fill="${stroke}"/>`,
    frost: `<rect x="14" y="4" width="4" height="24" fill="${stroke}"/><rect x="4" y="14" width="24" height="4" fill="${stroke}"/><rect x="10" y="10" width="12" height="12" fill="${fill}"/><rect x="14" y="14" width="4" height="4" fill="${hi}"/>`,
    ember: `<rect x="10" y="14" width="12" height="13" fill="${stroke}"/><rect x="12" y="8" width="8" height="16" fill="${fill}"/><rect x="15" y="5" width="4" height="8" fill="${hi}"/>`,
    poison: `<rect x="11" y="6" width="10" height="5" fill="${stroke}"/><rect x="9" y="11" width="14" height="17" fill="${stroke}"/><rect x="11" y="13" width="10" height="12" fill="${fill}"/><rect x="13" y="15" width="5" height="3" fill="${hi}"/>`,
    thunderbow: `<path d="M8 3 C25 10 25 22 8 29" fill="none" stroke="${stroke}" stroke-width="4"/><path d="M8 3 C20 10 20 22 8 29" fill="none" stroke="${fill}" stroke-width="3"/><rect x="17" y="5" width="2" height="22" fill="${hi}"/><rect x="22" y="11" width="5" height="4" fill="${hi}"/>`,
    frostcannon: `<rect x="6" y="10" width="20" height="11" fill="${stroke}"/><rect x="8" y="12" width="16" height="7" fill="${fill}"/><rect x="12" y="5" width="8" height="5" fill="${hi}"/><rect x="9" y="22" width="14" height="4" fill="${stroke}"/>`,
    bloodcleaver: `<rect x="14" y="6" width="5" height="22" fill="${stroke}"/><rect x="5" y="4" width="16" height="14" fill="${fill}"/><rect x="8" y="7" width="8" height="4" fill="${hi}"/><rect x="20" y="19" width="4" height="5" fill="${fill}"/>`,
    engine: `<rect x="7" y="7" width="18" height="18" fill="${stroke}"/><rect x="10" y="10" width="12" height="12" fill="${fill}"/><rect x="13" y="3" width="6" height="7" fill="${hi}"/><rect x="13" y="22" width="6" height="7" fill="${hi}"/>`,
  };
  return `<svg class="item-glyph" viewBox="0 0 32 32" aria-hidden="true">${parts[kind] || parts.dagger}</svg>`;
}

function drawPixelEnemy(ctx, enemy, frame) {
  const x = Math.round(enemy.x);
  const y = Math.round(enemy.y);
  const s = enemy.size;
  const bob = Math.round(Math.sin(frame * 0.12 + enemy.seed) * 2);
  ctx.save();
  ctx.imageSmoothingEnabled = false;
  ctx.globalAlpha = enemy.hit > 0 ? 0.65 : 1;
  ctx.fillStyle = 'rgba(0,0,0,0.38)';
  ctx.fillRect(x - s, y + s - 2, s * 2, 5);
  ctx.fillStyle = enemy.hit > 0 ? '#ffffff' : enemy.color;
  ctx.fillRect(x - s, y - s + bob, s * 2, s * 2);
  ctx.fillStyle = '#0a0d13';
  ctx.fillRect(x - s, y - s + bob, s * 2, 3);
  ctx.fillRect(x - s, y + s - 3 + bob, s * 2, 3);
  ctx.fillRect(x - s, y - s + bob, 3, s * 2);
  ctx.fillRect(x + s - 3, y - s + bob, 3, s * 2);
  ctx.fillStyle = '#f8fbff';
  ctx.fillRect(x - 5, y - 4 + bob, 3, 3);
  ctx.fillRect(x + 4, y - 4 + bob, 3, 3);
  if (enemy.kind === 'boss') {
    ctx.fillStyle = '#f2c14e';
    ctx.fillRect(x - 16, y - s - 8 + bob, 10, 5);
    ctx.fillRect(x + 6, y - s - 8 + bob, 10, 5);
  }
  const bw = Math.max(24, s * 2);
  ctx.fillStyle = '#0a0d13';
  ctx.fillRect(x - bw / 2, y - s - 12 + bob, bw, 4);
  ctx.fillStyle = enemy.burn > 0 ? '#f06d3a' : enemy.poison > 0 ? '#75c94a' : '#e05243';
  ctx.fillRect(x - bw / 2, y - s - 12 + bob, bw * Math.max(0, enemy.hp / enemy.maxHp), 4);
  ctx.restore();
}

function drawGate(ctx, x, y, life, maxLife) {
  ctx.fillStyle = '#0a0d13';
  ctx.fillRect(x - 18, y - 52, 36, 74);
  ctx.fillStyle = '#45546a';
  ctx.fillRect(x - 14, y - 48, 28, 68);
  ctx.fillStyle = '#1b2330';
  ctx.fillRect(x - 8, y - 26, 16, 46);
  ctx.fillStyle = '#f2c14e';
  ctx.fillRect(x - 12, y - 52, 24, 6);
  ctx.fillStyle = '#0a0d13';
  ctx.fillRect(x - 18, y + 26, 36, 5);
  ctx.fillStyle = '#43d17a';
  ctx.fillRect(x - 18, y + 36, 36 * Math.max(0, life / maxLife), 4);
}
