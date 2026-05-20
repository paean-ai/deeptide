// Pixel Climb - Donkey-Kong-style climb-the-beams platformer. Stacked
// horizontal beams with ladders between them; barrels roll across the
// beams from the top and fall off the ends. Climb past every barrel
// to reach the goal at the top.

const VW = 360, VH = 480;

// Beam geometry: 6 beams stacked vertically. Beam thickness = 6 px.
// Each beam alternates horizontal-flow direction so barrels naturally
// snake down (like the original).
const BEAMS = 6;
const BEAM_Y = [];      // top-y of each beam (0 = topmost)
const BEAM_DIR = [];    // +1 (right) or -1 (left) for barrel flow

function setupBeams() {
  // Fill from bottom up so the player starts at (low x, high y).
  const startY = 70;          // top beam y
  const step = 56;            // vertical spacing
  for (let i = 0; i < BEAMS; i++) {
    BEAM_Y.push(startY + i * step);
    BEAM_DIR.push(i % 2 === 0 ? 1 : -1);   // top beam flows right
  }
}
setupBeams();

const BEAM_THICK = 6;
const PLAYER_W = 12, PLAYER_H = 18;
const BARREL_R = 8;
const GRAVITY = 900;
const JUMP_VY = -260;
const MOVE_SPEED = 90;
const CLIMB_SPEED = 64;

// Ladder spec: per gap between beam i (above) and beam i+1 (below), one
// or two ladder columns are placed at specific x positions per level.
// Hand-designed for clean visual rhythm; the player can climb through
// either column.

const LEVELS = [
  // 1. Climb 1 — one ladder per gap, generous spacing, slow barrels.
  {
    name: ['Tutorial', '入门'],
    ladders: [
      [40, 270], [310, 270], [40, 320], [310, 320], [40, 320],   // dummy fill
    ],
    barrelCd: 2.8, barrelSpeed: 70,
  },
  // 2. Climb 2 — two ladders per gap; barrels a bit faster.
  {
    name: ['Site',     '工地'],
    ladders: [[40, 280], [120, 280], [40, 280], [120, 280], [40, 280]],
    barrelCd: 2.4, barrelSpeed: 82,
  },
  // 3. Tighten — barrels faster.
  {
    name: ['Brace',    '支撑'],
    ladders: [[60, 280], [200, 280], [80, 260], [220, 240], [60, 260]],
    barrelCd: 2.1, barrelSpeed: 92,
  },
  // 4. Stagger — ladders far apart, forcing a long beam-walk.
  {
    name: ['Stagger',  '错位'],
    ladders: [[80, 280], [120, 240], [200, 280], [60, 240], [240, 280]],
    barrelCd: 1.9, barrelSpeed: 100,
  },
  // 5. Frenzy — fast barrels.
  {
    name: ['Frenzy',   '混乱'],
    ladders: [[60, 220], [200, 280], [100, 220], [220, 280], [80, 220]],
    barrelCd: 1.6, barrelSpeed: 112,
  },
  // 6. Pinnacle — finale.
  {
    name: ['Pinnacle', '顶峰'],
    ladders: [[50, 200], [220, 280], [60, 180], [240, 260], [80, 200]],
    barrelCd: 1.3, barrelSpeed: 124,
  },
];
const LEVEL_COUNT = LEVELS.length;

// Per-level ladders are 5 entries — one ladder per gap (beams i / i+1 for
// i in 0..BEAMS-2 = 0..4). Each entry is [x1, x2] giving two ladder
// columns. We keep the level data as a flat array of `[x1, x2]` pairs.
// Builder normalises into a list of { x, top, bottom } ladder rects.

function buildLadders(level) {
  const ladders = [];
  for (let i = 0; i < BEAMS - 1; i++) {
    const cols = level.ladders[i] || [40, 320];
    for (const x of cols) {
      ladders.push({ x, top: BEAM_Y[i] + BEAM_THICK, bottom: BEAM_Y[i + 1] });
    }
  }
  return ladders;
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  return {
    levelIndex, cfg,
    ladders: buildLadders(cfg),
    player: {
      x: 30, y: BEAM_Y[BEAMS - 1] - PLAYER_H, vx: 0, vy: 0,
      state: 'beam',      // 'beam' | 'jump' | 'climb' | 'dead'
      face: 1,
      hitFlash: 0,
      respawn: 0,
    },
    barrels: [],          // { x, y, vx, vy, state: 'roll' | 'fall', beam: int }
    barrelCd: cfg.barrelCd * 0.6,
    over: false, won: false,
    lives: 2,
    score: 0,
    flash: 0,
    input: { left: false, right: false, up: false, down: false },
    jumpQueued: false,
  };
}

// ---- input -------------------------------------------------------------
function setInput(s, key, on) {
  if (!s.input || s.over) return;
  if (key === 'left' || key === 'right' || key === 'up' || key === 'down') {
    s.input[key] = on;
  } else if (key === 'jump' && on) {
    s.jumpQueued = true;
  }
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  const p = s.player;
  if (p.hitFlash > 0) p.hitFlash = Math.max(0, p.hitFlash - dt);
  // Respawn beat.
  if (p.state === 'dead') {
    p.respawn -= dt;
    if (p.respawn <= 0) {
      p.x = 30; p.y = BEAM_Y[BEAMS - 1] - PLAYER_H;
      p.vx = 0; p.vy = 0;
      p.state = 'beam'; p.face = 1; p.hitFlash = 0.6;
    }
    advanceBarrels(s, dt);
    return;
  }
  // Player horizontal input
  if (p.state !== 'climb') {
    if (s.input.left)       { p.vx = -MOVE_SPEED; p.face = -1; }
    else if (s.input.right) { p.vx =  MOVE_SPEED; p.face =  1; }
    else                    { p.vx = 0; }
  } else {
    p.vx = 0;
  }
  // Ladder grab — only if standing on a beam AND aligned with a ladder.
  if (p.state === 'beam') {
    const ladder = ladderAt(s.ladders, p.x + PLAYER_W / 2, p.y + PLAYER_H);
    if (ladder && s.input.up) {
      p.state = 'climb';
      p.x = ladder.x - PLAYER_W / 2;
      p.y -= 1;
      p.vx = 0; p.vy = 0;
    }
    // Step down through a beam onto the ladder below — only if the
    // current standing beam has a ladder going further down.
    if (s.input.down) {
      const beamI = currentBeam(p.y + PLAYER_H);
      if (beamI < BEAMS - 1) {
        // A ladder must straddle the beam below (i.e. its bottom is the
        // next beam's top edge).
        const downLadder = downLadderAt(s.ladders, p.x + PLAYER_W / 2, beamI);
        if (downLadder) {
          p.state = 'climb';
          p.x = downLadder.x - PLAYER_W / 2;
          p.y = BEAM_Y[beamI] + BEAM_THICK + 1;
          p.vx = 0; p.vy = 0;
        }
      }
    }
  }
  if (p.state === 'climb') {
    p.vy = s.input.up ? -CLIMB_SPEED : s.input.down ? CLIMB_SPEED : 0;
    p.y += p.vy * dt;
    // Top of ladder: snap onto the beam above.
    const ladder = ladderColumn(s.ladders, p.x + PLAYER_W / 2);
    if (ladder) {
      if (p.y + PLAYER_H <= ladder.top + 1) {
        p.y = ladder.top - PLAYER_H;
        p.state = 'beam';
        p.vy = 0;
      } else if (p.y + PLAYER_H >= ladder.bottom + BEAM_THICK + 1) {
        p.y = ladder.bottom - PLAYER_H;
        p.state = 'beam';
        p.vy = 0;
      }
    }
  } else {
    // beam or jump
    if (s.jumpQueued && p.state === 'beam') {
      p.vy = JUMP_VY;
      p.state = 'jump';
      s.score += 5;  // a small bonus for staying brave
    }
    s.jumpQueued = false;
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    p.vy += GRAVITY * dt;
    p.x = Math.max(0, Math.min(VW - PLAYER_W, p.x));
    // Land on a beam.
    const beamI = currentBeam(p.y + PLAYER_H);
    if (p.vy >= 0 && beamI >= 0) {
      const beamTop = BEAM_Y[beamI];
      if (p.y + PLAYER_H >= beamTop && p.y + PLAYER_H <= beamTop + BEAM_THICK + 6) {
        p.y = beamTop - PLAYER_H;
        p.vy = 0;
        p.state = 'beam';
      }
    }
    // Off the bottom = die.
    if (p.y > VH) { die(s); return; }
  }
  // Reached the goal beam (top) — win.
  if (p.state === 'beam' && p.y + PLAYER_H <= BEAM_Y[0] + BEAM_THICK + 2) {
    s.over = true; s.won = true;
    s.score += 500;
    s.flash = 0.5;
    return;
  }
  // Spawn + advance barrels.
  s.barrelCd -= dt;
  if (s.barrelCd <= 0) {
    spawnBarrel(s);
    s.barrelCd = s.cfg.barrelCd;
  }
  advanceBarrels(s, dt);
  // Collision with barrels.
  for (const b of s.barrels) {
    const px = p.x + PLAYER_W / 2, py = p.y + PLAYER_H / 2;
    if (Math.abs(b.x - px) < BARREL_R + 5 && Math.abs(b.y - py) < BARREL_R + 6) {
      die(s); return;
    }
  }
}

function spawnBarrel(s) {
  const topY = BEAM_Y[0];
  s.barrels.push({
    x: 40, y: topY - BARREL_R, vx: s.cfg.barrelSpeed * BEAM_DIR[0], vy: 0,
    state: 'roll', beam: 0, age: 0,
  });
}

function advanceBarrels(s, dt) {
  for (const b of s.barrels) {
    b.age += dt;
    if (b.state === 'roll') {
      b.x += b.vx * dt;
      // Roll off the end of the beam.
      if (b.x < -BARREL_R || b.x > VW + BARREL_R) {
        b.state = 'fall';
        b.vy = 60;
        // Step to next beam: stay at current x but clamp into bounds.
        b.x = Math.max(BARREL_R, Math.min(VW - BARREL_R, b.x));
      }
    } else {
      // falling
      b.y += b.vy * dt;
      b.vy += GRAVITY * dt;
      const beamI = currentBeam(b.y + BARREL_R);
      if (b.vy > 0 && beamI > b.beam && beamI >= 0) {
        // Land on the next beam.
        b.beam = beamI;
        b.y = BEAM_Y[beamI] - BARREL_R;
        b.vy = 0;
        if (beamI >= BEAMS - 1) {
          // Last beam — keep rolling off then despawn.
          b.state = 'roll';
          b.vx = s.cfg.barrelSpeed * BEAM_DIR[beamI];
        } else {
          b.state = 'roll';
          b.vx = s.cfg.barrelSpeed * BEAM_DIR[beamI];
        }
      }
      if (b.y > VH + 40) b.dead = true;
    }
  }
  s.barrels = s.barrels.filter(b => !b.dead);
}

function die(s) {
  const p = s.player;
  if (p.state === 'dead') return;
  p.state = 'dead';
  p.hitFlash = 0.6;
  s.flash = 0.4;
  s.lives--;
  if (s.lives < 0) { s.over = true; s.won = false; return; }
  p.respawn = 0.7;
}

// Which beam is at vertical y (player's foot or barrel centre)?
// Returns the beam index whose top sits <= y, or -1 if none.
function currentBeam(y) {
  let best = -1;
  for (let i = 0; i < BEAMS; i++) {
    if (BEAM_Y[i] <= y + 4 && BEAM_Y[i] >= y - 30) {
      best = i;
    }
  }
  return best;
}

// A ladder whose top sits below y (player can step DOWN onto it from
// beam `beamI`)? Returns the ladder if so.
function downLadderAt(ladders, x, beamI) {
  for (const l of ladders) {
    if (Math.abs(l.x - x) < 14 && l.top === BEAM_Y[beamI] + BEAM_THICK) return l;
  }
  return null;
}

// Returns the ladder whose top is just below this y position (player
// standing on a beam can grab a ladder going UP from this beam).
function ladderAt(ladders, x, y) {
  for (const l of ladders) {
    if (Math.abs(l.x - x) < 14 && Math.abs(l.bottom - y) < 8) return l;
  }
  return null;
}
function ladderColumn(ladders, x) {
  for (const l of ladders) if (Math.abs(l.x - x) < 14) return l;
  return null;
}

function finalScore(s) {
  return s.score + Math.max(0, s.lives) * 100;
}
