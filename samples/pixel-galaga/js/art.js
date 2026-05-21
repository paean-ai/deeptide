// Pixel-art rendering for Pixel Galaga. 360x480 world units.

const PALETTE = {
  bg:       '#06061a',
  bgHi:     '#0e0e2a',
  star:     '#f8f5e8',
  starDim:  '#5060a0',
  player:   '#5fc0ff',
  playerHi: '#a8e0ff',
  playerLo: '#205a8a',
  bullet:   '#ffd34a',
  bulletHi: '#fff0c8',
  enemyBul: '#ff7a7a',
  hud:      '#06061a',
  hudText:  '#f8f5e8',
  hudDim:   '#a0a8b8',
  heart:    '#ff4a5a',
  win:      '#5fc06e',
};

// A small starfield with three parallax layers for depth.
function buildStarfield() {
  const stars = [];
  for (let i = 0; i < 60; i++) {
    stars.push({ x: (i * 53 + 11) % VW, y: (i * 79 + 17) % VH, speed: 12 + (i % 3) * 12 });
  }
  return stars;
}
let stars = null;

function drawBackdrop(ctx, t) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, VW, VH);
  if (!stars) stars = buildStarfield();
  for (const s of stars) {
    const y = (s.y + s.speed * t) % VH;
    ctx.fillStyle = s.speed > 20 ? PALETTE.star : PALETTE.starDim;
    ctx.fillRect(s.x | 0, y | 0, s.speed > 20 ? 2 : 1, s.speed > 20 ? 2 : 1);
  }
}

function drawPlayer(ctx, p) {
  if (!p.alive && p.respawn > 0) return;
  if (p.hitFlash > 0 && Math.floor(p.hitFlash * 12) % 2 === 0) return;
  const x = p.x, y = PLAYER_Y;
  // Wings.
  ctx.fillStyle = PALETTE.playerLo;
  ctx.fillRect(x - 12, y - 2, 24, 8);
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(x - 11, y - 1, 22, 6);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(x - 11, y - 1, 22, 2);
  // Hull
  ctx.fillStyle = PALETTE.player;
  ctx.fillRect(x - 3, y - 9, 6, 12);
  ctx.fillStyle = PALETTE.playerHi;
  ctx.fillRect(x - 2, y - 9, 4, 4);
  // Nose
  ctx.fillStyle = PALETTE.bulletHi;
  ctx.fillRect(x - 1, y - 11, 2, 3);
}

function drawEnemies(ctx, enemies, t) {
  for (const e of enemies) {
    if (!e.alive) continue;
    const wob = Math.sin(t * 5 + e.slotC * 0.4 + e.slotR * 0.6) * 1;
    drawEnemy(ctx, e.x | 0, (e.y + wob) | 0, e.color, e.state === 'dive');
  }
}

function drawEnemy(ctx, x, y, color, diving) {
  // Bug-shaped enemy: round body + side wings + antennae.
  ctx.fillStyle = '#1a0a0a';
  fillDisk(ctx, x, y, ENEMY_R);
  ctx.fillStyle = color;
  fillDisk(ctx, x, y - 1, ENEMY_R - 1);
  ctx.fillStyle = '#fff7ed';
  fillDisk(ctx, x - 2, y - 3, ENEMY_R - 5);
  // Wings on the sides.
  ctx.fillStyle = color;
  ctx.fillRect(x - ENEMY_R - 3, y - 2, 3, 4);
  ctx.fillRect(x + ENEMY_R,     y - 2, 3, 4);
  // Antennae (only when diving, to telegraph aggression).
  if (diving) {
    ctx.fillStyle = PALETTE.bulletHi;
    ctx.fillRect(x - 3, y - ENEMY_R - 3, 1, 3);
    ctx.fillRect(x + 2, y - ENEMY_R - 3, 1, 3);
  }
}

function fillDisk(ctx, cx, cy, r) {
  if (r <= 0) return;
  const r2 = r * r;
  for (let dy = -r; dy <= r; dy++) {
    const w = Math.floor(Math.sqrt(r2 - dy * dy));
    ctx.fillRect((cx - w) | 0, (cy + dy) | 0, w * 2 + 1, 1);
  }
}

function drawBullet(ctx, b) {
  ctx.fillStyle = PALETTE.bullet;
  ctx.fillRect((b.x - 1) | 0, (b.y - 5) | 0, 2, 9);
  ctx.fillStyle = PALETTE.bulletHi;
  ctx.fillRect((b.x - 1) | 0, (b.y - 5) | 0, 2, 2);
}
function drawEnemyBullet(ctx, b) {
  ctx.fillStyle = PALETTE.enemyBul;
  ctx.fillRect((b.x - 2) | 0, (b.y - 3) | 0, 4, 6);
  ctx.fillStyle = PALETTE.bulletHi;
  ctx.fillRect((b.x - 1) | 0, (b.y - 3) | 0, 2, 2);
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

function drawFlash(ctx, s) {
  if (s.flash <= 0) return;
  const a = Math.min(1, s.flash / 0.5);
  ctx.fillStyle = s.won ? `rgba(95,192,110,${0.35 * a})` :
                  !s.player.alive ? `rgba(255,80,80,${0.5 * a})` :
                                    `rgba(255,255,255,${0.18 * a})`;
  ctx.fillRect(0, 32, VW, VH - 32);
}
