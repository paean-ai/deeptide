// Pixel Galaga - a tribute to Namco's Galaga: enemies enter the screen
// in arcing formations, settle into a slowly drifting grid, then
// individual enemies break formation to dive-bomb the player. The
// player sits near the bottom and shoots straight up.
//
// Each grid slot is fixed in world space; an enemy in 'enter' state
// follows a sine-curve path from off-screen to its slot; in 'formation'
// it tracks the grid origin's drift; in 'dive' it parabolically swoops
// at the last-known player position and returns to formation.

const VW = 360, VH = 480;
const PLAYER_Y = 440;
const PLAYER_W = 24, PLAYER_H = 18;
const BULLET_SPEED = 360;
const BULLET_R = 3;
const ENEMY_R = 10;
const ENEMY_BULLET_SPEED = 180;
const PLAYER_FIRE_CD = 0.25;

// Formation grid: 4 rows x 8 cols centred under HUD.
const ROWS = 4;
const COLS = 8;
const SLOT_W = 36;
const SLOT_H = 30;
const SLOT_OX = ((VW - COLS * SLOT_W) / 2) | 0;     // 36
const SLOT_OY = 56;                                  // top of formation

function slotXY(col, row) {
  return { x: SLOT_OX + col * SLOT_W + SLOT_W / 2, y: SLOT_OY + row * SLOT_H };
}

// ---- waves -------------------------------------------------------------
const LEVELS = [
  // diveCd shrinks across waves: faster dives + denser fire later.
  { name: ['Recon',     '侦察'], cols: 5, rows: 3, diveCd: 2.6, swarm: 0.9, fireChance: 0.35 },
  { name: ['Sortie',    '出击'], cols: 6, rows: 3, diveCd: 2.2, swarm: 1.0, fireChance: 0.40 },
  { name: ['Squadron',  '中队'], cols: 7, rows: 4, diveCd: 1.9, swarm: 1.1, fireChance: 0.45 },
  { name: ['Vanguard',  '前锋'], cols: 8, rows: 4, diveCd: 1.6, swarm: 1.2, fireChance: 0.50 },
  { name: ['Onslaught', '猛攻'], cols: 8, rows: 4, diveCd: 1.3, swarm: 1.4, fireChance: 0.55 },
  { name: ['Galaxy',    '银河'], cols: 8, rows: 4, diveCd: 1.0, swarm: 1.6, fireChance: 0.60 },
];
const LEVEL_COUNT = LEVELS.length;

// ---- helpers -----------------------------------------------------------
function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(levelIndex * 41 + 7);
  const enemies = [];
  const palette = ['#ff7a7a', '#ffd34a', '#5fc06e', '#bda6ff'];
  for (let r = 0; r < cfg.rows; r++) for (let c = 0; c < cfg.cols; c++) {
    const slot = slotXY(c, r);
    const fromLeft = c < cfg.cols / 2;
    enemies.push({
      slotC: c, slotR: r,
      x: fromLeft ? -20 : VW + 20,            // enter from off-screen
      y: -20,
      vx: 0, vy: 0,
      state: 'enter',
      enterT: 0,
      enterDur: 1.0 + (c + r) * 0.05,         // staggered entry
      enterFromLeft: fromLeft,
      color: palette[r % palette.length],
      hp: 1,
      alive: true,
      diveT: 0,
      diveTargetX: 180,
      diveFireT: 0,
    });
  }
  return {
    levelIndex, cfg, rng,
    enemies,
    formationOx: 0,                  // current horizontal drift of the formation
    formationDir: 1,
    player: { x: VW / 2, alive: true, fireCd: 0, hitFlash: 0, respawn: 0 },
    bullets: [],
    enemyBullets: [],
    diveCdT: 1.0 + rng() * 1.5,
    score: 0,
    lives: 2,
    inputX: null,
    elapsed: 0,
    over: false, won: false,
    flash: 0,
  };
}

// ---- input -------------------------------------------------------------
function setPlayerX(s, x) {
  if (s.over) return;
  s.inputX = Math.max(PLAYER_W / 2, Math.min(VW - PLAYER_W / 2, x));
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.elapsed += dt;
  s.flash = Math.max(0, s.flash - dt);
  // Respawn beat.
  if (!s.player.alive) {
    s.player.respawn -= dt;
    if (s.player.respawn <= 0) resetPlayer(s);
    advanceEnemies(s, dt);
    return;
  }
  // Player slides toward inputX if set.
  if (s.inputX !== null) {
    const dx = s.inputX - s.player.x;
    s.player.x += Math.sign(dx) * Math.min(Math.abs(dx), 280 * dt);
  }
  // Auto-fire.
  s.player.fireCd -= dt;
  if (s.player.fireCd <= 0) {
    s.bullets.push({ x: s.player.x, y: PLAYER_Y - 12, vy: -BULLET_SPEED, dead: false });
    s.player.fireCd = PLAYER_FIRE_CD;
  }
  // Bullets travel up; remove off-screen.
  for (const b of s.bullets) {
    b.y += b.vy * dt;
    if (b.y < -10) b.dead = true;
  }
  // Formation drift.
  s.formationOx += s.formationDir * s.cfg.swarm * 16 * dt;
  if (s.formationOx > 22) { s.formationOx = 22; s.formationDir = -1; }
  if (s.formationOx < -22) { s.formationOx = -22; s.formationDir = 1; }
  advanceEnemies(s, dt);
  // Spawn a dive periodically.
  s.diveCdT -= dt;
  if (s.diveCdT <= 0) {
    pickDiver(s);
    s.diveCdT = s.cfg.diveCd * (0.8 + s.rng() * 0.5);
  }
  // Enemy bullets travel down.
  for (const b of s.enemyBullets) {
    b.y += b.vy * dt;
    if (b.y > VH + 10) b.dead = true;
  }
  // Bullet vs enemy.
  for (const b of s.bullets) {
    if (b.dead) continue;
    for (const e of s.enemies) {
      if (!e.alive) continue;
      if (Math.abs(b.x - e.x) < ENEMY_R + BULLET_R && Math.abs(b.y - e.y) < ENEMY_R + BULLET_R) {
        e.alive = false;
        b.dead = true;
        s.score += e.state === 'dive' ? 200 : 50;
        s.flash = 0.18;
        break;
      }
    }
  }
  s.bullets = s.bullets.filter(b => !b.dead);
  // Enemy bullet / dive vs player.
  for (const b of s.enemyBullets) {
    if (b.dead) continue;
    if (Math.abs(b.x - s.player.x) < PLAYER_W / 2 && Math.abs(b.y - PLAYER_Y) < PLAYER_H / 2) {
      b.dead = true; die(s); return;
    }
  }
  s.enemyBullets = s.enemyBullets.filter(b => !b.dead);
  for (const e of s.enemies) {
    if (!e.alive) continue;
    if (e.state === 'dive' &&
        Math.abs(e.x - s.player.x) < PLAYER_W / 2 + ENEMY_R - 2 &&
        Math.abs(e.y - PLAYER_Y) < PLAYER_H / 2 + ENEMY_R - 2) {
      die(s); return;
    }
  }
  // Wave clear when every enemy is down.
  if (s.enemies.every(e => !e.alive)) {
    s.over = true; s.won = true; s.score += 300; s.flash = 0.5;
  }
}

function advanceEnemies(s, dt) {
  for (const e of s.enemies) {
    if (!e.alive) continue;
    const slot = slotXY(e.slotC, e.slotR);
    const targetX = slot.x + s.formationOx;
    const targetY = slot.y;
    if (e.state === 'enter') {
      e.enterT += dt;
      const u = Math.min(1, e.enterT / e.enterDur);
      // S-curve from off-screen to slot.
      const startX = e.enterFromLeft ? -20 : VW + 20;
      const startY = -20;
      const arcX = startX + (targetX - startX) * u;
      const arcY = startY + (targetY - startY) * u + Math.sin(u * Math.PI) * 80;
      e.x = arcX; e.y = arcY;
      if (u >= 1) { e.state = 'formation'; e.x = targetX; e.y = targetY; }
    } else if (e.state === 'formation') {
      e.x = targetX; e.y = targetY;
    } else if (e.state === 'dive') {
      e.diveT += dt;
      // Parabolic dive: down to mid-screen, then loop back to slot.
      const u = e.diveT / 2.2;
      if (u >= 1) {
        e.state = 'formation';
      } else {
        // Phase 1: dive down + horizontal toward diveTargetX.
        if (u < 0.55) {
          const v = u / 0.55;
          e.x = e.x + (e.diveTargetX - e.x) * Math.min(1, v * 1.2) * dt * 12;
          e.y += 220 * dt;
          // Fire periodically during dive.
          e.diveFireT -= dt;
          if (e.diveFireT <= 0 && s.rng() < s.cfg.fireChance) {
            s.enemyBullets.push({ x: e.x, y: e.y + 6, vy: ENEMY_BULLET_SPEED, dead: false });
            e.diveFireT = 0.7;
          }
          if (e.y > VH - 40) {
            // Loop back up: jump to start of return.
            e.diveT = 0.55 * 2.2;
          }
        } else {
          // Return phase: glide back to slot.
          const v = (u - 0.55) / 0.45;
          e.x = e.x + (targetX - e.x) * v * dt * 18;
          e.y = e.y + (targetY - e.y) * v * dt * 18;
        }
      }
    }
  }
}

function pickDiver(s) {
  const candidates = s.enemies.filter(e => e.alive && e.state === 'formation');
  if (!candidates.length) return;
  const e = candidates[Math.floor(s.rng() * candidates.length)];
  e.state = 'dive';
  e.diveT = 0;
  e.diveTargetX = s.player.x;
  e.diveFireT = 0.3;
}

function die(s) {
  const p = s.player;
  if (!p.alive) return;
  p.alive = false;
  p.hitFlash = 0.6;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  p.respawn = 0.7;
}

function resetPlayer(s) {
  s.player.x = VW / 2;
  s.player.alive = true;
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 100;
}
