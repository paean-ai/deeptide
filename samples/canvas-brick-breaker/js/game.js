const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const W = 960;
const H = 540;
const BEST_KEY = 'canvas-brick-breaker-best';

const POWERUPS = {
  wide: { label: 'W', color: '#68da86', name: 'Wide Paddle' },
  multi: { label: 'M', color: '#f4c85a', name: 'Multi Ball' },
  laser: { label: 'L', color: '#ff7d7d', name: 'Laser Paddle' },
  slow: { label: 'S', color: '#b7a7ff', name: 'Slow Time' },
};

const state = {
  score: 0,
  stage: 1,
  lives: 3,
  combo: 0,
  running: true,
  launched: false,
  shake: 0,
  slowTimer: 0,
  laserTimer: 0,
  msg: { k: 'breakAll' },
  best: +(localStorage.getItem(BEST_KEY) || 0),
  paddle: { x: W / 2, y: H - 58, w: 132, h: 16, target: W / 2, cooldown: 0 },
  balls: [],
  bricks: [],
  powerups: [],
  particles: [],
  lasers: [],
};

function restart() {
  Object.assign(state, {
    score: 0,
    stage: 1,
    lives: 3,
    combo: 0,
    running: true,
    launched: false,
    shake: 0,
    slowTimer: 0,
    laserTimer: 0,
    msg: { k: 'breakAll' },
    powerups: [],
    particles: [],
    lasers: [],
  });
  state.paddle.w = 132;
  makeStage();
  updateHud();
}

function resetBall() {
  state.launched = false;
  state.combo = 0;
  state.balls = [{ x: state.paddle.x, y: state.paddle.y - 18, vx: 4.4, vy: -5.4, r: 8, trail: [] }];
}

function makeStage() {
  state.bricks = [];
  const rows = Math.min(9, 4 + state.stage);
  const cols = 12;
  const gap = 6;
  const bw = 68;
  const bh = 24;
  const ox = (W - cols * bw - (cols - 1) * gap) / 2;
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const edge = x === 0 || x === cols - 1 || y === 0;
      const armored = state.stage >= 3 && (x + y + state.stage) % 7 === 0;
      const hp = armored ? 4 : 1 + Math.floor((y + state.stage - 1) / 3);
      const type = armored ? 'armor' : edge && state.stage >= 2 ? 'edge' : 'core';
      state.bricks.push({ x: ox + x * (bw + gap), y: 76 + y * (bh + gap), w: bw, h: bh, hp, maxHp: hp, type, hit: 0 });
    }
  }
  state.msg = { k: 'stageMsg', a: [state.stage, state.bricks.length] };
  resetBall();
}

function launch() {
  if (!state.running) return;
  state.launched = true;
}

function spawnPowerup(x, y) {
  const keys = Object.keys(POWERUPS);
  const type = keys[Math.floor(Math.random() * keys.length)];
  state.powerups.push({ x, y, type, vy: 2.15, spin: 0 });
}

function burst(x, y, color, count = 10, power = 1) {
  for (let i = 0; i < count; i++) {
    state.particles.push({
      x,
      y,
      vx: (Math.random() - 0.5) * 5 * power,
      vy: (Math.random() - 0.8) * 5 * power,
      life: 24 + Math.random() * 18,
      color,
      size: 2 + Math.random() * 3,
    });
  }
}

function activatePowerup(type) {
  state.msg = { k: 'pu_' + type };
  if (type === 'wide') state.paddle.w = Math.min(230, state.paddle.w + 34);
  if (type === 'multi') {
    const source = state.balls[0] || { x: state.paddle.x, y: state.paddle.y - 18 };
    state.balls.push({ x: source.x, y: source.y, vx: -5.2, vy: -5.6, r: 8, trail: [] });
    state.balls.push({ x: source.x, y: source.y, vx: 5.2, vy: -5.6, r: 8, trail: [] });
  }
  if (type === 'laser') state.laserTimer = 520;
  if (type === 'slow') state.slowTimer = 420;
}

function update() {
  if (!state.running) return;
  const speedMul = state.slowTimer > 0 ? 0.62 : 1;
  if (state.slowTimer > 0) state.slowTimer--;
  if (state.laserTimer > 0) state.laserTimer--;
  if (state.shake > 0) state.shake *= 0.86;

  const p = state.paddle;
  p.x += (p.target - p.x) * 0.22;
  p.x = Math.max(p.w / 2 + 12, Math.min(W - p.w / 2 - 12, p.x));
  p.cooldown--;

  if (state.laserTimer > 0 && state.launched && p.cooldown <= 0) {
    p.cooldown = 18;
    state.lasers.push({ x: p.x - 38, y: p.y, life: 38 });
    state.lasers.push({ x: p.x + 38, y: p.y, life: 38 });
  }

  if (!state.launched) {
    for (const b of state.balls) {
      b.x = p.x;
      b.y = p.y - 18;
    }
  }

  for (const b of state.balls) {
    if (!state.launched) continue;
    b.trail.push({ x: b.x, y: b.y });
    if (b.trail.length > 9) b.trail.shift();
    b.x += b.vx * speedMul;
    b.y += b.vy * speedMul;
    if (b.x < b.r) { b.x = b.r; b.vx *= -1; }
    if (b.x > W - b.r) { b.x = W - b.r; b.vx *= -1; }
    if (b.y < b.r + 42) { b.y = b.r + 42; b.vy *= -1; }
    if (b.y > H + 42) b.dead = true;

    if (b.x > p.x - p.w / 2 && b.x < p.x + p.w / 2 && b.y + b.r > p.y && b.y - b.r < p.y + p.h && b.vy > 0) {
      const t = (b.x - p.x) / (p.w / 2);
      b.vx = t * 7.2;
      b.vy = -Math.max(5.6, Math.abs(b.vy) + 0.18);
      burst(b.x, b.y, '#7fe8ff', 6);
    }

    for (const brick of state.bricks) {
      if (brick.dead || b.x + b.r < brick.x || b.x - b.r > brick.x + brick.w || b.y + b.r < brick.y || b.y - b.r > brick.y + brick.h) continue;
      hitBrick(brick, b.x, b.y, 1);
      const fromSide = b.x < brick.x + 4 || b.x > brick.x + brick.w - 4;
      if (fromSide) b.vx *= -1;
      else b.vy *= -1;
      break;
    }
  }

  for (const laser of state.lasers) {
    laser.y -= 14;
    laser.life--;
    for (const brick of state.bricks) {
      if (brick.dead || laser.x < brick.x || laser.x > brick.x + brick.w || laser.y < brick.y || laser.y > brick.y + brick.h) continue;
      laser.life = 0;
      hitBrick(brick, laser.x, laser.y, 1);
      break;
    }
  }
  state.lasers = state.lasers.filter(l => l.life > 0 && l.y > 42);

  state.balls = state.balls.filter(b => !b.dead);
  if (!state.balls.length) {
    state.lives--;
    state.paddle.w = Math.max(132, state.paddle.w - 24);
    if (state.lives <= 0) {
      state.running = false;
      if (state.score > state.best) {
        state.best = state.score;
        try { localStorage.setItem(BEST_KEY, state.best); } catch (e) { /* storage off */ }
      }
    } else resetBall();
  }

  state.bricks = state.bricks.filter(b => !b.dead);
  if (!state.bricks.length) {
    state.stage++;
    state.lives++;
    state.score += 500 * state.stage;
    makeStage();
  }

  for (const item of state.powerups) {
    item.y += item.vy;
    item.spin += 0.12;
    if (item.x > p.x - p.w / 2 && item.x < p.x + p.w / 2 && item.y > p.y - 12 && item.y < p.y + p.h + 22) {
      item.dead = true;
      activatePowerup(item.type);
      burst(item.x, item.y, POWERUPS[item.type].color, 14);
    }
  }
  state.powerups = state.powerups.filter(item => !item.dead && item.y < H + 40);

  for (const fx of state.particles) {
    fx.x += fx.vx;
    fx.y += fx.vy;
    fx.vy += 0.08;
    fx.life--;
  }
  state.particles = state.particles.filter(fx => fx.life > 0);
  updateHud();
}

function hitBrick(brick, x, y, damage) {
  brick.hp -= damage;
  brick.hit = 8;
  state.combo++;
  state.score += (12 + state.combo * 2) * state.stage;
  burst(x, y, brickColor(brick), 9);
  if (brick.hp <= 0) {
    brick.dead = true;
    state.score += 30 * state.stage + state.combo * 5;
    state.shake = Math.max(state.shake, brick.type === 'armor' ? 4 : 2);
    if (Math.random() < (brick.type === 'armor' ? 0.32 : 0.16)) spawnPowerup(brick.x + brick.w / 2, brick.y + brick.h / 2);
  }
}

function brickColor(brick) {
  if (brick.type === 'armor') return '#c8d2e0';
  return ['#7fe8ff', '#68da86', '#f4c85a', '#ff8f66', '#aa7dff'][Math.min(4, brick.maxHp - 1)];
}

function draw() {
  const sx = state.shake ? (Math.random() - 0.5) * state.shake : 0;
  const sy = state.shake ? (Math.random() - 0.5) * state.shake : 0;
  ctx.save();
  ctx.translate(sx, sy);
  drawArena();
  drawBricks();
  drawPowerups();
  drawPaddle();
  drawBalls();
  drawLasers();
  drawParticles();
  ctx.restore();
  drawOverlay();
}

function drawArena() {
  ctx.fillStyle = '#08101a';
  ctx.fillRect(0, 0, W, H);
  const g = ctx.createLinearGradient(0, 42, 0, H);
  g.addColorStop(0, '#111b2b');
  g.addColorStop(1, '#070b12');
  ctx.fillStyle = g;
  ctx.fillRect(0, 42, W, H - 42);
  for (let x = 0; x < W; x += 32) {
    ctx.fillStyle = x % 64 === 0 ? '#121f31' : '#0e1726';
    ctx.fillRect(x, 42, 16, H);
  }
  ctx.fillStyle = 'rgba(127,232,255,0.16)';
  ctx.fillRect(0, 42, W, 3);
}

function drawBricks() {
  for (const brick of state.bricks) {
    if (brick.hit > 0) brick.hit--;
    const inset = brick.hit > 0 ? 2 : 0;
    ctx.fillStyle = '#05080d';
    ctx.fillRect(brick.x + 3, brick.y + 4, brick.w, brick.h);
    ctx.fillStyle = brickColor(brick);
    ctx.fillRect(brick.x + inset, brick.y + inset, brick.w - inset * 2, brick.h - inset * 2);
    ctx.fillStyle = 'rgba(255,255,255,0.30)';
    ctx.fillRect(brick.x + 5, brick.y + 4, brick.w - 10, 4);
    if (brick.type === 'armor') {
      ctx.fillStyle = '#5b6573';
      for (let x = brick.x + 8; x < brick.x + brick.w - 4; x += 14) ctx.fillRect(x, brick.y + 14, 8, 4);
    }
    if (brick.hp > 1) {
      ctx.fillStyle = '#071018';
      ctx.font = '12px monospace';
      ctx.textAlign = 'center';
      ctx.fillText(brick.hp, brick.x + brick.w / 2, brick.y + 17);
    }
  }
}

function drawPowerups() {
  for (const item of state.powerups) {
    const def = POWERUPS[item.type];
    ctx.fillStyle = '#05080d';
    ctx.fillRect(item.x - 13, item.y - 12, 28, 28);
    ctx.fillStyle = def.color;
    ctx.fillRect(item.x - 12, item.y - 13, 26, 26);
    ctx.fillStyle = '#071018';
    ctx.font = '13px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(def.label, item.x + 1, item.y + 5);
  }
}

function drawPaddle() {
  const p = state.paddle;
  ctx.fillStyle = '#05080d';
  ctx.fillRect(p.x - p.w / 2 + 4, p.y + 5, p.w, p.h);
  ctx.fillStyle = state.laserTimer > 0 ? '#ff7d7d' : '#edf4ff';
  ctx.fillRect(p.x - p.w / 2, p.y, p.w, p.h);
  ctx.fillStyle = state.slowTimer > 0 ? '#b7a7ff' : '#7fe8ff';
  ctx.fillRect(p.x - p.w / 2 + 9, p.y + 4, p.w - 18, 5);
  if (state.laserTimer > 0) {
    ctx.fillStyle = '#ffced5';
    ctx.fillRect(p.x - 42, p.y - 8, 8, 10);
    ctx.fillRect(p.x + 34, p.y - 8, 8, 10);
  }
}

function drawBalls() {
  for (const b of state.balls) {
    b.trail.forEach((p, i) => {
      ctx.globalAlpha = i / b.trail.length * 0.35;
      ctx.fillStyle = '#f4c85a';
      ctx.beginPath();
      ctx.arc(p.x, p.y, b.r * (i / b.trail.length), 0, Math.PI * 2);
      ctx.fill();
    });
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#fff2a6';
    ctx.beginPath();
    ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#f4c85a';
    ctx.fillRect(b.x - 3, b.y - 3, 6, 6);
  }
}

function drawLasers() {
  for (const l of state.lasers) {
    ctx.globalAlpha = Math.max(0, l.life / 38);
    ctx.fillStyle = '#ff7d7d';
    ctx.fillRect(l.x - 2, l.y, 4, 42);
    ctx.globalAlpha = 1;
  }
}

function drawParticles() {
  for (const fx of state.particles) {
    ctx.globalAlpha = Math.max(0, fx.life / 34);
    ctx.fillStyle = fx.color;
    ctx.fillRect(fx.x, fx.y, fx.size, fx.size);
    ctx.globalAlpha = 1;
  }
}

function drawOverlay() {
  ctx.fillStyle = 'rgba(7,10,16,0.74)';
  ctx.fillRect(14, H - 42, 330, 28);
  ctx.fillStyle = '#a7b5c8';
  ctx.font = '12px monospace';
  ctx.textAlign = 'left';
  ctx.fillText(`${mt(state.msg)}  ${t('combo')} x${state.combo}`, 24, H - 24);
  if (!state.launched && state.running) {
    ctx.fillStyle = '#a7b5c8';
    ctx.font = '18px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('launchHint'), W / 2, H / 2 + 112);
  }
  if (!state.running) {
    ctx.fillStyle = 'rgba(0,0,0,0.65)';
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = '#f4c85a';
    ctx.font = '42px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('gameOver'), W / 2, H / 2 - 12);
    ctx.fillStyle = '#a7b5c8';
    ctx.font = '16px monospace';
    ctx.fillText(t('finalScore', state.score), W / 2, H / 2 + 24);
  }
}

function updateHud() {
  document.getElementById('score').textContent = state.score;
  document.getElementById('stage').textContent = state.stage;
  document.getElementById('lives').textContent = state.lives;
  document.getElementById('best').textContent = Math.max(state.best, state.score);
}

function pointerX(e) {
  const rect = canvas.getBoundingClientRect();
  return ((e.clientX - rect.left) / rect.width) * W;
}

canvas.addEventListener('pointermove', e => {
  state.paddle.target = pointerX(e);
});
canvas.addEventListener('pointerdown', e => {
  state.paddle.target = pointerX(e);
  launch();
});
document.addEventListener('keydown', e => {
  if (e.key === ' ') {
    e.preventDefault();
    launch();
  }
  if (e.key === 'ArrowLeft') state.paddle.target -= 64;
  if (e.key === 'ArrowRight') state.paddle.target += 64;
});
document.getElementById('restart').onclick = restart;
setupLanguageToggle(updateHud);

function loop() {
  update();
  draw();
  requestAnimationFrame(loop);
}

restart();
loop();
