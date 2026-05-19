// Pixel Sky Raiders - pixel art for ships, enemies, the boss, bullets, pickups.

function srShade(hex, amt) {
  const n = parseInt(hex.slice(1), 16);
  let r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  r = Math.max(0, Math.min(255, r + amt));
  g = Math.max(0, Math.min(255, g + amt));
  b = Math.max(0, Math.min(255, b + amt));
  return '#' + ((r << 16) | (g << 8) | b).toString(16).padStart(6, '0');
}
function R(ctx, x, y, w, h, c) { ctx.fillStyle = c; ctx.fillRect(x | 0, y | 0, w, h); }

// Player raider, pointing up. cx,cy = centre.
function drawPlayer(ctx, cx, cy, t, invuln, shield) {
  if (invuln > 0 && Math.floor(t * 18) % 2) return; // blink while invulnerable
  const x = cx - 11, y = cy - 13;
  // thruster flame
  const f = 4 + Math.abs(Math.sin(t * 30)) * 5;
  R(ctx, cx - 4, y + 24, 3, f, '#ffd24d');
  R(ctx, cx + 1, y + 24, 3, f, '#ff8a3c');
  // wings
  R(ctx, x, y + 12, 6, 9, '#3f6ea8');
  R(ctx, x + 16, y + 12, 6, 9, '#3f6ea8');
  R(ctx, x, y + 12, 6, 2, '#5e96d4');
  R(ctx, x + 16, y + 12, 6, 2, '#5e96d4');
  // body
  R(ctx, x + 8, y, 6, 26, '#dfe7f2');
  R(ctx, x + 6, y + 6, 10, 18, '#b9c6d8');
  R(ctx, x + 8, y, 6, 6, '#eef3fa');
  // cockpit
  R(ctx, x + 9, y + 8, 4, 5, '#7ad0ff');
  R(ctx, x + 9, y + 8, 4, 2, '#d6f4ff');
  // side cannons
  R(ctx, x + 3, y + 8, 3, 7, '#9aa6b8');
  R(ctx, x + 16, y + 8, 3, 7, '#9aa6b8');
  if (shield > 0) {
    ctx.globalAlpha = 0.35 + Math.abs(Math.sin(t * 6)) * 0.3;
    ctx.strokeStyle = '#7ad0ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(cx, cy, 22, 0, Math.PI * 2);
    ctx.stroke();
    ctx.globalAlpha = 1;
  }
}

function drawEnemy(ctx, cx, cy, type, t, flash) {
  const c = ENEMIES[type].color;
  const lit = flash > 0 ? '#ffffff' : c;
  const dk = srShade(c, -58);
  if (type === 'drone') {
    R(ctx, cx - 8, cy - 6, 16, 10, lit);
    R(ctx, cx - 8, cy + 2, 16, 3, dk);
    R(ctx, cx - 11, cy - 2, 3, 6, dk);
    R(ctx, cx + 8, cy - 2, 3, 6, dk);
    R(ctx, cx - 3, cy - 3, 6, 4, '#1a1626');
  } else if (type === 'weaver') {
    for (let i = -3; i <= 3; i++) {
      const w = 10 - Math.abs(i) * 2;
      R(ctx, cx - w, cy + i * 2 - 1, w * 2, 2, i === 0 ? lit : srShade(c, i < 0 ? 30 : -34));
    }
    R(ctx, cx - 2, cy - 2, 4, 4, '#1a1626');
  } else if (type === 'turret') {
    R(ctx, cx - 11, cy - 7, 22, 12, lit);
    R(ctx, cx - 11, cy + 1, 22, 4, dk);
    R(ctx, cx - 3, cy + 4, 6, 7, '#9aa6b8'); // barrel
    R(ctx, cx - 6, cy - 4, 12, 4, srShade(c, 36));
    R(ctx, cx - 3, cy - 4, 6, 4, '#1a1626');
  } else if (type === 'tank') {
    R(ctx, cx - 16, cy - 10, 32, 18, lit);
    R(ctx, cx - 16, cy + 3, 32, 5, dk);
    R(ctx, cx - 16, cy - 10, 32, 4, srShade(c, 34));
    R(ctx, cx - 9, cy + 7, 4, 7, '#9aa6b8');
    R(ctx, cx + 5, cy + 7, 4, 7, '#9aa6b8');
    R(ctx, cx - 6, cy - 4, 12, 6, '#1a1626');
    R(ctx, cx - 4, cy - 2, 3, 3, c);
  }
}

function drawBoss(ctx, cx, cy, t, flash) {
  const base = flash > 0 ? '#ffffff' : '#c04a7a';
  R(ctx, cx - 44, cy - 24, 88, 36, base);
  R(ctx, cx - 44, cy + 6, 88, 8, '#6e2547');
  R(ctx, cx - 44, cy - 24, 88, 6, '#e07ba2');
  // wing pods
  R(ctx, cx - 56, cy - 10, 14, 28, '#8c3560');
  R(ctx, cx + 42, cy - 10, 14, 28, '#8c3560');
  // core eye
  const pulse = 6 + Math.sin(t * 4) * 3;
  R(ctx, cx - pulse, cy - pulse, pulse * 2, pulse * 2, '#ffd24d');
  R(ctx, cx - pulse / 2, cy - pulse / 2, pulse, pulse, '#ff8a3c');
  // cannons
  R(ctx, cx - 26, cy + 12, 7, 12, '#9aa6b8');
  R(ctx, cx + 19, cy + 12, 7, 12, '#9aa6b8');
  R(ctx, cx - 4, cy + 12, 8, 14, '#9aa6b8');
}

function drawBullet(ctx, b) {
  R(ctx, b.x - 2, b.y - 7, 4, 11, '#fff2a8');
  R(ctx, b.x - 1, b.y - 9, 2, 5, '#ffffff');
}
function drawEnemyBullet(ctx, b, t) {
  ctx.fillStyle = '#ff7ad0';
  ctx.beginPath();
  ctx.arc(b.x, b.y, 4, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = '#ffd6f0';
  ctx.fillRect(b.x - 1, b.y - 1, 2, 2);
}

function drawPowerup(ctx, x, y, kind, t) {
  const p = POWERUPS[kind];
  const bob = Math.sin(t * 4) * 2;
  R(ctx, x - 9, y - 9 + bob, 18, 18, '#10131c');
  R(ctx, x - 7, y - 7 + bob, 14, 14, p.color);
  R(ctx, x - 7, y - 7 + bob, 14, 3, srShade(p.color, 50));
  ctx.fillStyle = '#10131c';
  ctx.font = 'bold 12px ui-monospace, monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(p.glyph, x, y + 1 + bob);
  ctx.textBaseline = 'alphabetic';
  ctx.textAlign = 'left';
}
