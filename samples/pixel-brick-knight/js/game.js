// Pixel Brick Knight - breakout physics, floors, roguelite upgrades, save.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-brick-knight-best';
let best = parseInt(localStorage.getItem(SAVE_KEY) || '0', 10) || 0;

const PADDLE_Y = VH - 58, PADDLE_H = 12, BALL_R = 6;
let run = null;
let state = null;
let dragging = false;

// ---- run / floor setup ---------------------------------------------------
function newRun() {
  run = {
    floor: 1, gold: 0, lives: 3, dmg: 1, paddleW: 66,
    ballCount: 1, ballSpeed: 250, pierce: false, goldMult: 1,
  };
  startFloor();
}

function startFloor() {
  state = {
    bricks: genFloor(run.floor),
    balls: [],
    paddle: { x: VW / 2, w: run.paddleW },
    particles: [],
    status: 'launch',
  };
  updateHud();
}

function launchBalls() {
  state.status = 'play';
  state.balls = [];
  const minVx = run.ballSpeed * 0.2;
  for (let i = 0; i < run.ballCount; i++) {
    const ang = -Math.PI / 2 + (i - (run.ballCount - 1) / 2) * 0.34
      + (Math.random() - 0.5) * 0.3;
    let vx = Math.cos(ang) * run.ballSpeed;
    let vy = Math.sin(ang) * run.ballSpeed;
    // guarantee a horizontal component so a ball can never get stuck vertical
    if (Math.abs(vx) < minVx) {
      vx = (Math.random() < 0.5 ? -1 : 1) * minVx;
      vy = -Math.sqrt(Math.max(1, run.ballSpeed * run.ballSpeed - vx * vx));
    }
    state.balls.push({
      x: state.paddle.x, y: PADDLE_Y - BALL_R - 1, r: BALL_R,
      vx, vy, hitSet: new Set(),
    });
  }
}

// ---- physics -------------------------------------------------------------
function reflectBall(ball, b) {
  const oxL = (ball.x + ball.r) - b.x;
  const oxR = (b.x + b.w) - (ball.x - ball.r);
  const oyT = (ball.y + ball.r) - b.y;
  const oyB = (b.y + b.h) - (ball.y - ball.r);
  const minX = Math.min(oxL, oxR), minY = Math.min(oyT, oyB);
  if (minX < minY) {
    ball.vx = -ball.vx;
    ball.x += oxL < oxR ? -minX : minX;
  } else {
    ball.vy = -ball.vy;
    ball.y += oyT < oyB ? -minY : minY;
  }
}
function circleHitsBrick(ball, b) {
  const cx = Math.max(b.x, Math.min(ball.x, b.x + b.w));
  const cy = Math.max(b.y, Math.min(ball.y, b.y + b.h));
  return (ball.x - cx) ** 2 + (ball.y - cy) ** 2 < ball.r * ball.r;
}

function burst(x, y, color, n) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, s = 40 + Math.random() * 150;
    state.particles.push({
      x, y, vx: Math.cos(a) * s, vy: Math.sin(a) * s - 40,
      life: 0.3 + Math.random() * 0.4, color,
    });
  }
}

function stepBall(ball, h) {
  const p = state.paddle, pw = p.w;
  ball.x += ball.vx * h;
  ball.y += ball.vy * h;
  if (ball.x < ball.r) { ball.x = ball.r; ball.vx = Math.abs(ball.vx); }
  if (ball.x > VW - ball.r) { ball.x = VW - ball.r; ball.vx = -Math.abs(ball.vx); }
  if (ball.y < ball.r + 40) { ball.y = ball.r + 40; ball.vy = Math.abs(ball.vy); }
  // paddle
  if (ball.vy > 0 && ball.y + ball.r >= PADDLE_Y && ball.y + ball.r <= PADDLE_Y + PADDLE_H + 8 &&
      ball.x >= p.x - pw / 2 - ball.r && ball.x <= p.x + pw / 2 + ball.r) {
    ball.y = PADDLE_Y - ball.r;
    const off = Math.max(-1, Math.min(1, (ball.x - p.x) / (pw / 2)));
    const ang = -Math.PI / 2 + off * 1.05;
    ball.vx = Math.cos(ang) * run.ballSpeed;
    ball.vy = Math.sin(ang) * run.ballSpeed;
    // never let the ball settle into a stuck vertical bounce
    const minVx = run.ballSpeed * 0.2;
    if (Math.abs(ball.vx) < minVx) {
      ball.vx = (ball.vx >= 0 ? 1 : -1) * minVx;
      ball.vy = -Math.sqrt(Math.max(1, run.ballSpeed * run.ballSpeed - ball.vx * ball.vx));
    }
  }
  // bricks
  const overlapping = state.bricks.filter(b => circleHitsBrick(ball, b));
  for (const b of overlapping) {
    if (!ball.hitSet.has(b)) {
      b.hp -= run.dmg;
      if (b.hp <= 0) b.dead = true;
    }
  }
  if (!run.pierce && overlapping.length) reflectBall(ball, overlapping[0]);
  ball.hitSet = new Set(overlapping);
}

function update(dt) {
  if (!state) return;
  for (const pa of state.particles) {
    pa.life -= dt; pa.x += pa.vx * dt; pa.y += pa.vy * dt; pa.vy += 300 * dt;
  }
  state.particles = state.particles.filter(pa => pa.life > 0);

  if (state.status === 'launch') {
    return;
  }
  if (state.status !== 'play') return;

  const STEPS = 3, h = dt / STEPS;
  for (let s = 0; s < STEPS; s++) {
    for (const ball of state.balls) stepBall(ball, h);
  }
  // resolve destroyed bricks
  let cleared = false;
  for (let i = state.bricks.length - 1; i >= 0; i--) {
    const b = state.bricks[i];
    if (b.dead) {
      run.gold += Math.round(b.gold * run.goldMult);
      burst(b.x + b.w / 2, b.y + b.h / 2,
        b.kind === 'boss' ? '#e8554f' : '#f2cf3f', b.kind === 'boss' ? 22 : 8);
      state.bricks.splice(i, 1);
      cleared = true;
    }
  }
  if (cleared) updateHud();
  // lost balls
  state.balls = state.balls.filter(b => b.y - b.r < VH);
  if (state.balls.length === 0) {
    run.lives--;
    updateHud();
    if (run.lives <= 0) { gameOver(); return; }
    state.status = 'launch';
  }
  if (state.bricks.length === 0) floorCleared();
}

// ---- floor flow ----------------------------------------------------------
function floorCleared() {
  state.status = 'upgrade';
  const pool = UPGRADES.filter(u => !(u.id === 'pierce' && run.pierce));
  for (let i = pool.length - 1; i > 0; i--) {
    const j = (Math.random() * (i + 1)) | 0;
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  const picks = pool.slice(0, 3);
  const cards = document.getElementById('upgrade-cards');
  cards.innerHTML = '';
  const li = currentLang === 'zh' ? 1 : 0;
  for (const up of picks) {
    const card = document.createElement('div');
    card.className = 'up-card';
    card.innerHTML = '<b>' + up.name[li] + '</b><span>' + up.desc[li] + '</span>';
    card.onclick = () => {
      up.apply(run);
      run.floor++;
      document.getElementById('overlay-upgrade').classList.add('hidden');
      startFloor();
    };
    cards.appendChild(card);
  }
  document.getElementById('overlay-upgrade').classList.remove('hidden');
}

function gameOver() {
  state.status = 'over';
  const newBest = run.floor > best;
  if (newBest) { best = run.floor; try { localStorage.setItem(SAVE_KEY, String(best)); } catch (e) { /* ignore */ } }
  setTimeout(() => {
    document.getElementById('final-line').textContent = t('finalLine', run.floor, run.gold);
    document.getElementById('final-best').textContent =
      (newBest ? t('newBest') + '  ' : '') + t('bestLine', best);
    document.getElementById('overlay-over').classList.remove('hidden');
  }, 700);
}

// ---- render --------------------------------------------------------------
function render() {
  drawBackground(ctx);
  if (!state) return;
  for (const b of state.bricks) drawBrick(ctx, b);
  drawPaddle(ctx, state.paddle.x, PADDLE_Y, state.paddle.w, PADDLE_H);
  if (state.status === 'launch') {
    drawBall(ctx, { x: state.paddle.x, y: PADDLE_Y - BALL_R - 1, r: BALL_R });
  }
  for (const ball of state.balls) drawBall(ctx, ball);
  for (const pa of state.particles) {
    ctx.globalAlpha = Math.min(1, pa.life * 2.6);
    ctx.fillStyle = pa.color;
    ctx.fillRect(pa.x | 0, pa.y | 0, 3, 3);
  }
  ctx.globalAlpha = 1;
  if (state.status === 'launch') {
    ctx.fillStyle = '#ece8f4';
    ctx.font = '900 13px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(currentLang === 'zh' ? '点击发球' : 'TAP TO LAUNCH', VW / 2, VH - 86);
    ctx.textAlign = 'left';
  }
}

// ---- HUD -----------------------------------------------------------------
function updateHud() {
  if (!run) return;
  document.getElementById('hud-floor').textContent = 'FLOOR ' + run.floor;
  document.getElementById('hud-gold').textContent = '◎ ' + run.gold;
  document.getElementById('hud-lives').textContent = '♥'.repeat(Math.max(0, run.lives)) || '—';
}

// ---- input ---------------------------------------------------------------
function pointerX(e) {
  const rect = canvas.getBoundingClientRect();
  return (e.clientX - rect.left) * VW / rect.width;
}
function movePaddle(px) {
  if (!state) return;
  const half = state.paddle.w / 2;
  state.paddle.x = Math.max(half, Math.min(VW - half, px));
}
canvas.addEventListener('pointerdown', e => {
  if (!state || document.getElementById('screen-game').classList.contains('hidden')) return;
  dragging = true;
  movePaddle(pointerX(e));
  if (state.status === 'launch') launchBalls();
});
canvas.addEventListener('pointermove', e => { if (dragging) movePaddle(pointerX(e)); });
canvas.addEventListener('pointerup', () => { dragging = false; });
canvas.addEventListener('pointercancel', () => { dragging = false; });

// ---- screens -------------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden'));
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function refreshTitle() {
  document.getElementById('title-best').textContent = best > 0 ? t('bestLine', best) : '';
}
document.getElementById('btn-play').onclick = () => { newRun(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => {
  document.getElementById('overlay-over').classList.add('hidden');
  newRun();
  showScreen('screen-game');
};
document.getElementById('btn-menu').onclick = () => { refreshTitle(); showScreen('screen-title'); };

setupLanguageToggle(() => { refreshTitle(); updateHud(); });

// ---- main loop -----------------------------------------------------------
let last = performance.now();
function loop(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;
  if (!document.getElementById('screen-game').classList.contains('hidden')) {
    update(dt);
    render();
  } else {
    drawBackground(ctx);
  }
  requestAnimationFrame(loop);
}
refreshTitle();
showScreen('screen-title');
requestAnimationFrame(loop);
