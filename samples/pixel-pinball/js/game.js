// Pixel Pinball - launch, flippers, ball physics, bumpers, targets, scoring.

const BEST_KEY = 'pixel-pinball-best';
const FLIP_REST = 0.55;        // ball restitution off flippers
const SCORE_BUMP = 150, SCORE_SLING = 75, SCORE_TARGET = 400;

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = VW;
canvas.height = VH;
ctx.imageSmoothingEnabled = false;

let game = null;
let best = +(localStorage.getItem(BEST_KEY) || 0);
let lastT = performance.now();

// ---- setup -------------------------------------------------------------
function newGame() {
  game = {
    ball: { x: TEE.x, y: TEE.y, vx: 0, vy: 0 },
    ballState: 'ready', ballsLeft: START_BALLS, score: 0,
    charge: 0, charging: false,
    flippers: FLIPPERS.map(f => ({ ...f, angle: f.rest, omega: 0, held: false })),
    bumpers: BUMPERS.map(b => ({ ...b, flash: 0 })),
    slings: SLINGS.map(s => ({ ...s, flash: 0 })),
    targets: TARGETS.map(r => ({ rect: r, up: true })),
    sparks: [], banner: null, t: 0, over: false,
  };
  updateHud();
}

function spawnBall() {
  if (game.ballsLeft <= 0) { gameOver(); return; }
  game.ball = { x: TEE.x, y: TEE.y, vx: 0, vy: 0 };
  game.ballState = 'ready';
  game.charge = 0; game.charging = false;
}

function launch() {
  if (game.ballState !== 'ready') return;
  const c = game.charge;
  game.ball.vx = -(38 + c * 44);
  game.ball.vy = -(LAUNCH_MIN + c * LAUNCH_RANGE);
  game.ballState = 'live';
  game.charging = false;
}

function loseBall() {
  game.ballsLeft--;
  if (game.ballsLeft <= 0) { gameOver(); return; }
  setBanner(game.ballsLeft === 1 ? t('lastBall') : t('ballLost'),
    game.ballsLeft === 1 ? '#ff7d6d' : '#9fb6e0');
  spawnBall();
  updateHud();
}

function gameOver() {
  game.over = true;
  game.ballState = 'dead';
  const isBest = game.score > best;
  if (isBest) { best = game.score; try { localStorage.setItem(BEST_KEY, best); } catch (e) {} }
  document.getElementById('final-score').textContent = t('finalScore', game.score);
  document.getElementById('final-best').textContent = isBest ? t('newBest') : `${t('best')}: ${best}`;
  showScreen('screen-over');
  updateHud();
}

function setBanner(text, color) { game.banner = { text, color, life: 1.4 }; }

function addSparks(x, y, color) {
  for (let i = 0; i < 7; i++) {
    const a = Math.random() * Math.PI * 2;
    game.sparks.push({ x, y, vx: Math.cos(a) * 90, vy: Math.sin(a) * 90, life: 1, color });
  }
}

// ---- collision helpers -------------------------------------------------
function collideSeg(b, s) {
  const ex = s[2] - s[0], ey = s[3] - s[1];
  const len2 = ex * ex + ey * ey || 1;
  let tt = ((b.x - s[0]) * ex + (b.y - s[1]) * ey) / len2;
  tt = Math.max(0, Math.min(1, tt));
  const cx = s[0] + ex * tt, cy = s[1] + ey * tt;
  const dx = b.x - cx, dy = b.y - cy;
  const d = Math.hypot(dx, dy);
  if (d >= BALL_R || d < 0.0001) return;
  const nx = dx / d, ny = dy / d;
  b.x = cx + nx * BALL_R; b.y = cy + ny * BALL_R;
  const dot = b.vx * nx + b.vy * ny;
  if (dot < 0) { b.vx -= (1 + WALL_REST) * dot * nx; b.vy -= (1 + WALL_REST) * dot * ny; }
}

function collideCircle(b, c, kick) {
  const dx = b.x - c.x, dy = b.y - c.y;
  let d = Math.hypot(dx, dy);
  if (d >= BALL_R + c.r) return false;
  if (d < 0.001) d = 0.001;
  const nx = dx / d, ny = dy / d;
  b.x = c.x + nx * (BALL_R + c.r); b.y = c.y + ny * (BALL_R + c.r);
  const dot = b.vx * nx + b.vy * ny;
  if (dot < 0) { b.vx -= 1.5 * dot * nx; b.vy -= 1.5 * dot * ny; }
  b.vx += nx * kick; b.vy += ny * kick;
  return true;
}

function collideTarget(b, tg) {
  const r = tg.rect;
  const cx = Math.max(r[0], Math.min(b.x, r[0] + r[2]));
  const cy = Math.max(r[1], Math.min(b.y, r[1] + r[3]));
  const dx = b.x - cx, dy = b.y - cy;
  const d = Math.hypot(dx, dy);
  if (d >= BALL_R) return false;
  const nx = d > 0.001 ? dx / d : 0, ny = d > 0.001 ? dy / d : -1;
  b.x = cx + nx * BALL_R; b.y = cy + ny * BALL_R;
  const dot = b.vx * nx + b.vy * ny;
  if (dot < 0) { b.vx -= 1.4 * dot * nx; b.vy -= 1.4 * dot * ny; }
  return true;
}

function collideFlipper(b, fl) {
  const ex = Math.cos(fl.angle) * fl.len, ey = Math.sin(fl.angle) * fl.len;
  const len2 = ex * ex + ey * ey || 1;
  let tt = ((b.x - fl.px) * ex + (b.y - fl.py) * ey) / len2;
  tt = Math.max(0, Math.min(1, tt));
  const cx = fl.px + ex * tt, cy = fl.py + ey * tt;
  let dx = b.x - cx, dy = b.y - cy;
  let d = Math.hypot(dx, dy);
  const minD = BALL_R + FLIP_THICK;
  if (d >= minD) return;
  if (d < 0.001) { dx = 0; dy = -1; d = 1; }
  const nx = dx / d, ny = dy / d;
  b.x = cx + nx * minD; b.y = cy + ny * minD;
  const r = tt * fl.len;
  const vsx = -fl.omega * r * Math.sin(fl.angle);
  const vsy = fl.omega * r * Math.cos(fl.angle);
  let rvx = b.vx - vsx, rvy = b.vy - vsy;
  const dot = rvx * nx + rvy * ny;
  if (dot < 0) { rvx -= (1 + FLIP_REST) * dot * nx; rvy -= (1 + FLIP_REST) * dot * ny; }
  b.vx = rvx + vsx; b.vy = rvy + vsy;
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const g = game;
  g.t += dt;
  // flippers ease toward rest/active
  for (const fl of g.flippers) {
    const target = fl.held ? fl.active : fl.rest;
    const prev = fl.angle;
    fl.angle += (target - fl.angle) * Math.min(1, 26 * dt);
    fl.omega = (fl.angle - prev) / dt;
  }
  // visuals
  for (const bm of g.bumpers) bm.flash = Math.max(0, bm.flash - dt * 4);
  for (const s of g.slings) s.flash = Math.max(0, s.flash - dt * 4);
  for (const sp of g.sparks) {
    sp.x += sp.vx * dt; sp.y += sp.vy * dt; sp.vy += 300 * dt; sp.life -= dt * 1.8;
  }
  g.sparks = g.sparks.filter(s => s.life > 0);
  if (g.banner) { g.banner.life -= dt; if (g.banner.life <= 0) g.banner = null; }

  if (g.charging && g.ballState === 'ready') g.charge = Math.min(1, g.charge + dt * 1.15);
  if (g.ballState !== 'live') return;

  const b = g.ball;
  const speed = Math.hypot(b.vx, b.vy);
  const steps = Math.max(1, Math.min(14, Math.ceil(speed * dt / 3.5)));
  const sdt = dt / steps;
  for (let s = 0; s < steps; s++) {
    b.vy += GRAVITY * sdt;
    b.x += b.vx * sdt; b.y += b.vy * sdt;
    for (const w of WALLS) collideSeg(b, w);
    for (const bm of g.bumpers) {
      if (collideCircle(b, bm, BUMP_KICK)) {
        bm.flash = 1; g.score += SCORE_BUMP; addSparks(bm.x, bm.y, '#ffd089'); updateHud();
      }
    }
    for (const sl of g.slings) {
      if (collideCircle(b, sl, SLING_KICK)) {
        sl.flash = 1; g.score += SCORE_SLING; addSparks(sl.x, sl.y, '#5be0a0'); updateHud();
      }
    }
    for (const tg of g.targets) {
      if (collideTarget(b, tg) && tg.up) {
        tg.up = false; g.score += SCORE_TARGET;
        addSparks(b.x, b.y, '#fff6c4'); updateHud();
        if (g.targets.every(x => !x.up)) {
          g.score += TARGET_BONUS;
          setBanner(t('targetBonus'), '#f2d24a');
          for (const x of g.targets) x.up = true;
          updateHud();
        }
      }
    }
    for (const fl of g.flippers) collideFlipper(b, fl);
    // speed cap
    const sp = Math.hypot(b.vx, b.vy);
    if (sp > MAX_SPEED) { b.vx *= MAX_SPEED / sp; b.vy *= MAX_SPEED / sp; }
    // drain
    if (b.y > DRAIN_Y || b.x < -24 || b.x > VW + 24 || b.y > VH + 60) {
      loseBall();
      return;
    }
  }
}

// ---- render ------------------------------------------------------------
function render() {
  const g = game;
  drawTable(ctx, g.t);
  for (const tg of g.targets) drawTarget(ctx, tg.rect, tg.up);
  drawWalls(ctx, WALLS);
  for (const bm of g.bumpers) drawBumper(ctx, bm, bm.flash);
  for (const sl of g.slings) drawSling(ctx, sl, sl.flash);
  for (const fl of g.flippers) drawFlipper(ctx, fl, fl.angle);
  drawSparks(ctx, g.sparks);
  if (g.ballState !== 'dead') drawBall(ctx, g.ball.x, g.ball.y);

  if (g.ballState === 'ready') {
    // launch charge meter beside the tee
    const h = 90, mx = 304, my = TEE.y - h / 2;
    ctx.fillStyle = '#0c1226';
    ctx.fillRect(mx - 3, my, 6, h);
    ctx.fillStyle = g.charge > 0.85 ? '#ff5d5d' : '#5be0a0';
    ctx.fillRect(mx - 3, my + h * (1 - g.charge), 6, h * g.charge);
  }
  if (g.banner) {
    ctx.globalAlpha = Math.min(1, g.banner.life);
    ctx.fillStyle = g.banner.color;
    ctx.font = '900 22px ui-monospace, Menlo, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(g.banner.text, VW / 2, 230);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
}

function updateHud() {
  if (!game) return;
  document.getElementById('hud-score').textContent = `${t('score')} ${game.score}`;
  document.getElementById('hud-best').textContent = `${t('best')} ${Math.max(best, game.score)}`;
  document.getElementById('hud-balls').textContent = `${t('balls')} ${'●'.repeat(Math.max(0, game.ballsLeft))}`;
}

// ---- screens -----------------------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}

// ---- input -------------------------------------------------------------
const pointerRole = new Map();
function setFlipper(side, held) {
  if (!game) return;
  for (const fl of game.flippers) if (fl.side === side) fl.held = held;
}
canvas.addEventListener('pointerdown', e => {
  if (!game || game.over) return;
  e.preventDefault();
  if (game.ballState === 'ready') {
    game.charging = true;
    pointerRole.set(e.pointerId, 'charge');
  } else {
    const r = canvas.getBoundingClientRect();
    const side = (e.clientX - r.left) < r.width / 2 ? 'L' : 'R';
    pointerRole.set(e.pointerId, side);
    setFlipper(side, true);
  }
});
function releasePointer(e) {
  const role = pointerRole.get(e.pointerId);
  if (role === undefined) return;
  pointerRole.delete(e.pointerId);
  if (role === 'charge') { if (game) launch(); }
  else setFlipper(role, false);
}
canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);
canvas.addEventListener('contextmenu', e => e.preventDefault());

window.addEventListener('keydown', e => {
  if (!game || game.over) return;
  if (e.code === 'ArrowLeft' || e.code === 'KeyA') setFlipper('L', true);
  else if (e.code === 'ArrowRight' || e.code === 'KeyD') setFlipper('R', true);
  else if (e.code === 'Space') { if (game.ballState === 'ready') game.charging = true; }
});
window.addEventListener('keyup', e => {
  if (!game) return;
  if (e.code === 'ArrowLeft' || e.code === 'KeyA') setFlipper('L', false);
  else if (e.code === 'ArrowRight' || e.code === 'KeyD') setFlipper('R', false);
  else if (e.code === 'Space') { if (game.charging) launch(); }
});

document.getElementById('btn-play').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-again').onclick = () => { newGame(); showScreen('screen-game'); };
document.getElementById('btn-menu').onclick = () => {
  updateHud();
  document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
  showScreen('screen-title');
};
setupLanguageToggle(() => { if (game) updateHud(); });

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.028, (now - lastT) / 1000);
  lastT = now;
  if (game && !document.getElementById('screen-game').classList.contains('hidden') && !game.over) {
    update(dt);
  }
  if (game) render();
  requestAnimationFrame(loop);
}

document.getElementById('title-best').textContent = best ? `${t('best')}: ${best}` : '';
newGame();
showScreen('screen-title');
requestAnimationFrame(loop);
