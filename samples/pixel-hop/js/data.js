// Pixel Hop - vertical endless-climber. Auto-bounce on every platform; drag
// left/right to steer. Reach the level's altitude target before you fall.
//
// Platform types add skill variety:
//   static  - the default - a fixed plank that gives a regular bounce.
//   mover   - slides left/right within a band; timing the landing matters.
//   spring  - launches you ~1.6x higher, useful for crossing wide gaps.
//   cloud   - vanishes after one bounce; commit to the next platform fast.
// Gems sprinkled along the climb give bonus score on pickup.

const VW = 360, VH = 480;
const PLAYER_R = 12;
const GRAVITY = 1300;
const JUMP_V = -520;
const SPRING_V = -820;
const MOVE_DAMP = 0.78;       // velocity decay per second
const MAX_VX = 360;
const TILT_ACCEL = 1400;
const PLATFORM_W = 56, PLATFORM_H = 10;

const LEVELS = [
  { name: ['Foothills', '丘陵'],  seed: 13,  target: 900,  density: 74,
    mix: { spring: 0, mover: 0,   cloud: 0   } },
  { name: ['Cliffs', '峭壁'],     seed: 41,  target: 1600, density: 86,
    mix: { spring: 8, mover: 6,   cloud: 0   } },
  { name: ['Spires', '尖塔'],     seed: 96,  target: 2400, density: 96,
    mix: { spring: 10, mover: 14, cloud: 6   } },
  { name: ['Skyway', '云道'],     seed: 162, target: 3400, density: 106,
    mix: { spring: 12, mover: 16, cloud: 14  } },
  { name: ['Stratos', '平流层'],  seed: 247, target: 4600, density: 114,
    mix: { spring: 12, mover: 22, cloud: 18  } },
  { name: ['Apex', '苍穹之巅'],   seed: 358, target: 6200, density: 122,
    mix: { spring: 14, mover: 26, cloud: 22  } },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  const rng = seededRandom(cfg.seed);
  const s = {
    levelIndex, cfg, rng,
    // World coords: y grows downward; the player climbs toward smaller y.
    player: { x: VW / 2, y: 0, vx: 0, vy: 0, alive: true },
    cameraY: -VH * 0.6,         // top of viewport in world coords
    platforms: [],
    gems: [],
    gemsCollected: 0,
    nextSpawnY: 40,
    nextGemY: -180,
    altitude: 0,                 // peak altitude in metres
    score: 0,                    // altitude + 50 * gems on win
    over: false, won: false, started: false,
  };
  // Anchor floor so the first flap always lands on something.
  s.platforms.push({
    x: VW / 2 - PLATFORM_W / 2, y: 40, w: PLATFORM_W,
    type: 'static', dir: 0, alive: true,
  });
  while (s.nextSpawnY > -cfg.target - 220) spawnNext(s);
  return s;
}

function spawnNext(s) {
  const cfg = s.cfg;
  s.nextSpawnY -= cfg.density;
  const x = 6 + s.rng() * (VW - PLATFORM_W - 12);
  // Pick a type from the configured mix; the rest are 'static'.
  const roll = s.rng() * 100;
  let acc = 0, type = 'static';
  const mix = cfg.mix;
  if ((acc += (mix.spring || 0)) > roll)      type = 'spring';
  else if ((acc += (mix.mover || 0)) > roll)  type = 'mover';
  else if ((acc += (mix.cloud || 0)) > roll)  type = 'cloud';
  const dir = type === 'mover' ? (s.rng() < 0.5 ? -1 : 1) : 0;
  s.platforms.push({ x, y: s.nextSpawnY, w: PLATFORM_W, type, dir, alive: true });
  // Drop a gem every ~3 platforms.
  if (s.nextSpawnY < s.nextGemY) {
    s.gems.push({
      x: 16 + s.rng() * (VW - 32),
      y: s.nextSpawnY + cfg.density * 0.5,
      alive: true,
    });
    s.nextGemY = s.nextSpawnY - 180 - s.rng() * 120;
  }
}

function flap(s) {                // kicks off the first jump (also called by tap)
  if (s.over) return;
  s.started = true;
  if (s.player.alive && s.player.vy >= 0) s.player.vy = JUMP_V;
}

// ---- tick ---------------------------------------------------------------
function tick(s, dt, tiltX) {
  if (s.over) return;
  if (!s.started) return;
  const p = s.player;
  if (!p.alive) {
    // Slide off-screen before the result banner.
    p.y += p.vy * dt;
    p.vy += GRAVITY * dt;
    if (p.y > s.cameraY + VH + 60) { s.over = true; s.won = false; }
    return;
  }
  // Horizontal control: tiltX in [-1..1] (drag relative to player).
  if (tiltX !== undefined && tiltX !== null) {
    p.vx += tiltX * TILT_ACCEL * dt;
    if (p.vx > MAX_VX) p.vx = MAX_VX;
    if (p.vx < -MAX_VX) p.vx = -MAX_VX;
  }
  p.vx *= 1 - MOVE_DAMP * dt;
  // Physics.
  p.vy += GRAVITY * dt;
  p.x += p.vx * dt;
  p.y += p.vy * dt;
  // Wrap horizontally Doodle-Jump style.
  if (p.x < -PLAYER_R) p.x += VW;
  else if (p.x > VW + PLAYER_R) p.x -= VW;
  // Move 'mover' platforms within a band.
  for (const pl of s.platforms) {
    if (pl.type !== 'mover' || !pl.alive) continue;
    pl.x += pl.dir * 60 * dt;
    if (pl.x < 4)              { pl.x = 4;              pl.dir =  1; }
    if (pl.x > VW - pl.w - 4)  { pl.x = VW - pl.w - 4;  pl.dir = -1; }
  }
  // Land on a platform if falling and overlapping.
  if (p.vy > 0) {
    for (const pl of s.platforms) {
      if (!pl.alive) continue;
      if (p.y < pl.y - 4 || p.y > pl.y + 14) continue;
      if (p.x < pl.x - PLAYER_R || p.x > pl.x + pl.w + PLAYER_R) continue;
      p.y = pl.y - 1;
      p.vy = pl.type === 'spring' ? SPRING_V : JUMP_V;
      if (pl.type === 'cloud') pl.alive = false;   // single-use platform
      break;
    }
  }
  // Gem pickup.
  for (const g of s.gems) {
    if (!g.alive) continue;
    if (Math.abs(g.x - p.x) < PLAYER_R + 8 && Math.abs(g.y - p.y) < PLAYER_R + 8) {
      g.alive = false;
      s.gemsCollected++;
    }
  }
  // Camera follows the player upward only.
  if (p.y - 200 < s.cameraY) s.cameraY = p.y - 200;
  // Altitude = how far above the start line.
  const climb = Math.max(0, -p.y);
  if (climb > s.altitude) s.altitude = climb;
  // Spawn more platforms ahead as the camera rises.
  while (s.nextSpawnY > s.cameraY - 200) spawnNext(s);
  // Drop platforms / gems that have scrolled well below the camera.
  s.platforms = s.platforms.filter(pl => pl.y < s.cameraY + VH + 80);
  s.gems = s.gems.filter(g => g.y < s.cameraY + VH + 80);
  // Win.
  if (s.altitude >= s.cfg.target && !s.over) {
    s.over = true; s.won = true;
    s.score = Math.round(s.altitude) + s.gemsCollected * 50;
  }
  // Lose - player below the visible field.
  if (p.y > s.cameraY + VH - 8) p.alive = false;
}
