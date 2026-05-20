// Pixel-art rendering for Pixel Tank. 360x480 world units.

const PALETTE = {
  bg:        '#0d1228',
  field:     '#1c2240',
  fieldEdge: '#262d54',
  brick:     '#a36835',
  brickEdge: '#5a3819',
  brickMortar:'#7d4f29',
  steel:     '#9aa6cc',
  steelEdge: '#5a6188',
  steelHi:   '#dde6ff',
  eagle:     '#f7e69a',
  eagleDark: '#a8853a',
  eagleDead: '#5a3819',
  player:    '#5fc06e',
  playerDark:'#2b6f3a',
  enemy:     '#e8554f',
  enemyDark: '#a83a37',
  bullet:    '#f8f5e8',
  hud:       '#0d1228',
  hudText:   '#f8f5e8',
  hudDim:    '#9aa6cc',
  ok:        '#54c47c',
};

function drawScene(ctx, s, lang) {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, 360, 480);
  // Field backdrop.
  ctx.fillStyle = PALETTE.fieldEdge;
  ctx.fillRect(PLAY_X - 2, PLAY_Y - 2, PLAY_W + 4, PLAY_H + 4);
  ctx.fillStyle = PALETTE.field;
  ctx.fillRect(PLAY_X, PLAY_Y, PLAY_W, PLAY_H);
  drawWalls(ctx, s);
  drawTanks(ctx, s);
  drawBullets(ctx, s);
  drawHud(ctx, s, lang);
}

function drawWalls(ctx, s) {
  for (let y = 0; y < GRID_H; y++) for (let x = 0; x < GRID_W; x++) {
    const t = s.grid[y * GRID_W + x];
    if (t === EMPTY) continue;
    const px = PLAY_X + x * CELL;
    const py = PLAY_Y + y * CELL;
    if (t === BRICK) {
      ctx.fillStyle = PALETTE.brickEdge;
      ctx.fillRect(px, py, CELL, CELL);
      ctx.fillStyle = PALETTE.brick;
      ctx.fillRect(px + 1, py + 1, CELL - 2, CELL - 2);
      ctx.fillStyle = PALETTE.brickMortar;
      ctx.fillRect(px, py + CELL / 2, CELL, 1);
      ctx.fillRect(px + CELL / 2, py,     1, CELL / 2);
      ctx.fillRect(px, py + CELL - 1, CELL, 1);
    } else if (t === STEEL) {
      ctx.fillStyle = PALETTE.steelEdge;
      ctx.fillRect(px, py, CELL, CELL);
      ctx.fillStyle = PALETTE.steel;
      ctx.fillRect(px + 1, py + 1, CELL - 2, CELL - 2);
      ctx.fillStyle = PALETTE.steelHi;
      ctx.fillRect(px + 2, py + 2, 4, 1);
    } else if (t === EAGLE) {
      ctx.fillStyle = PALETTE.eagleDark;
      ctx.fillRect(px, py, CELL, CELL);
      ctx.fillStyle = PALETTE.eagle;
      ctx.fillRect(px + 2, py + 2, CELL - 4, CELL - 4);
      ctx.fillStyle = PALETTE.eagleDark;
      ctx.fillRect(px + 5, py + 5, 2, 2);
      ctx.fillRect(px + 9, py + 5, 2, 2);
      ctx.fillRect(px + 4, py + 9, 8, 2);
    } else if (t === EAGLE_DEAD) {
      ctx.fillStyle = PALETTE.eagleDead;
      ctx.fillRect(px, py, CELL, CELL);
      ctx.fillStyle = PALETTE.brickEdge;
      ctx.fillRect(px + 5, py + 8, 6, 2);
    }
  }
}

function drawTanks(ctx, s) {
  if (s.player.alive) drawTank(ctx, s.player, true);
  for (const e of s.enemies) if (e.alive) drawTank(ctx, e, false);
}

function drawTank(ctx, t, isPlayer) {
  const px = PLAY_X + t.x, py = PLAY_Y + t.y;
  const base  = isPlayer ? PALETTE.player    : PALETTE.enemy;
  const dark  = isPlayer ? PALETTE.playerDark : PALETTE.enemyDark;
  // Body.
  ctx.fillStyle = dark;
  ctx.fillRect(px, py, TANK_SIZE, TANK_SIZE);
  ctx.fillStyle = base;
  ctx.fillRect(px + 1, py + 1, TANK_SIZE - 2, TANK_SIZE - 2);
  // Treads (left + right strips along motion axis).
  ctx.fillStyle = dark;
  if (t.dir === 0 || t.dir === 2) {
    ctx.fillRect(px,             py + 2, 2, TANK_SIZE - 4);
    ctx.fillRect(px + TANK_SIZE - 2, py + 2, 2, TANK_SIZE - 4);
  } else {
    ctx.fillRect(px + 2, py,             TANK_SIZE - 4, 2);
    ctx.fillRect(px + 2, py + TANK_SIZE - 2, TANK_SIZE - 4, 2);
  }
  // Turret + barrel.
  const cx = px + TANK_SIZE / 2, cy = py + TANK_SIZE / 2;
  ctx.fillStyle = dark;
  ctx.fillRect(cx - 2, cy - 2, 4, 4);
  const [dx, dy] = DIR_VEC[t.dir];
  ctx.fillRect(cx - 1 + dx * 2, cy - 1 + dy * 2, 2, 2);
  ctx.fillRect(cx - 1 + dx * 4, cy - 1 + dy * 4, 2, 2);
}

function drawBullets(ctx, s) {
  const all = [];
  if (s.player.bullet) all.push(s.player.bullet);
  for (const e of s.enemies) if (e.bullet) all.push(e.bullet);
  ctx.fillStyle = PALETTE.bullet;
  for (const b of all) {
    ctx.fillRect((PLAY_X + b.x) | 0, (PLAY_Y + b.y) | 0, BULLET_SIZE, BULLET_SIZE);
  }
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
  ctx.fillText(t(lang, 'lives') + ' ' + s.lives, 180, 16);
  ctx.textAlign = 'right';
  ctx.fillText(t(lang, 'left') + ' ' + s.enemiesLeft, 352, 16);
}

// Bottom D-pad + fire button.
function drawControls(ctx, lang, dpadHits, pressed) {
  const top = 320;
  ctx.fillStyle = 'rgba(13, 18, 40, 0.85)';
  ctx.fillRect(0, top, 360, 140);
  // D-pad on the left.
  const cx = 70, cy = 390, sz = 36;
  dpadHits.length = 0;
  function btn(x, y, w, h, key, label) {
    ctx.fillStyle = pressed.has(key) ? '#54c47c' : '#28315c';
    ctx.fillRect(x, y, w, h);
    ctx.fillStyle = pressed.has(key) ? '#86df9d' : '#3c4576';
    ctx.fillRect(x, y, w, 2);
    ctx.fillStyle = PALETTE.hudText;
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(label, x + w / 2, y + h / 2 + 1);
    dpadHits.push({ kind: key, x, y, w, h });
  }
  btn(cx - sz / 2, cy - sz * 1.5, sz, sz, 'up',    '▲');
  btn(cx - sz / 2, cy + sz / 2,  sz, sz, 'down',  '▼');
  btn(cx - sz * 1.5, cy - sz / 2, sz, sz, 'left',  '◀');
  btn(cx + sz / 2,  cy - sz / 2, sz, sz, 'right', '▶');
  // Fire button on the right.
  btn(255, 360, 80, 60, 'fire', 'FIRE');
}
