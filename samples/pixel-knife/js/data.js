// Pixel Knife - rotating-target knife-throw arcade.
//
// The target is a wooden disk that spins continuously. Tap to launch a knife
// from the bottom of the screen; when its tip reaches the disk it sticks at
// the disk's current top-most angle. If that angle is within TOLERANCE of any
// already-stuck knife the run fails. Land every knife in the level's quota
// to clear the round.

const VW = 360, VH = 480;

const DISK_CX     = 180;
const DISK_CY     = 170;
const DISK_R      = 64;            // wood radius
const KNIFE_LEN   = 56;
const KNIFE_BLADE = 36;
const KNIFE_W     = 8;
const KNIFE_SPEED = 760;           // px/s for flying knives
const SPAWN_Y     = VH - 80;       // bottom of canvas for queued knives
const TOLERANCE   = 0.21;          // ~12° angular safety zone (radians)
const APPLE_TOL   = 0.18;          // pickup window for an apple

// Each level: knife quota, rotation profile (radians/s, can change over time).
// `pattern` is one of 'steady' | 'reverse' | 'pulse' (slow/fast alternation).
// `apples` = number of bonus apples placed equidistant on the disk; landing a
// flying knife on an apple's slice scores +50 instead of sticking a knife.
const LEVELS = [
  { name: ['Sapling', '嫩芽'],   knives: 5,  speed: 1.05, pattern: 'steady',  apples: 0 },
  { name: ['Pine',    '松树'],   knives: 6,  speed: 1.55, pattern: 'steady',  apples: 0 },
  { name: ['Oak',     '橡树'],   knives: 7,  speed: 1.95, pattern: 'reverse', apples: 1 },
  { name: ['Maple',   '枫树'],   knives: 8,  speed: 2.35, pattern: 'pulse',   apples: 1 },
  { name: ['Bamboo',  '竹林'],   knives: 9,  speed: 2.70, pattern: 'reverse', apples: 2 },
  { name: ['Ironwood','铁木'],   knives: 10, speed: 3.20, pattern: 'pulse',   apples: 2 },
];
const LEVEL_COUNT = LEVELS.length;

function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  // Pre-place pinned knives on the disk so the player isn't starting from
  // empty — the rotation immediately matters. Spread them evenly at angles
  // 0, 2π/3, 4π/3 for the first level; just one for the rest so the early
  // rounds feel approachable.
  const startKnives = levelIndex === 0 ? 1 : levelIndex < 3 ? 1 : 2;
  const stuck = [];
  for (let i = 0; i < startKnives; i++) stuck.push({ relAngle: (i * 2 * Math.PI) / startKnives });
  // Apples sit on a slightly inset radius so a flying knife crossing the rim
  // can hit them. Equidistant placements.
  const apples = [];
  for (let i = 0; i < cfg.apples; i++) {
    apples.push({
      relAngle: ((i + 0.5) * 2 * Math.PI) / Math.max(1, cfg.apples),
      alive: true,
    });
  }
  return {
    levelIndex, cfg,
    disk: { angle: 0, baseSpeed: cfg.speed, dir: 1, elapsed: 0 },
    stuck, apples,
    flying: null,                   // { y: tip-y in world coords }
    queue: cfg.knives,              // knives left to launch
    thrown: 0,                      // knives successfully stuck
    score: 0,
    over: false, won: false, started: false,
  };
}

function currentSpeed(s) {
  const c = s.cfg;
  if (c.pattern === 'reverse') {
    // Direction flips every 1.6 s.
    return c.speed * (Math.floor(s.disk.elapsed / 1.6) % 2 === 0 ? 1 : -1);
  }
  if (c.pattern === 'pulse') {
    // Doubles for short bursts of 0.5 s every 1.5 s.
    const phase = s.disk.elapsed % 1.5;
    return c.speed * (phase < 0.5 ? 1.9 : 1);
  }
  return c.speed;
}

function tick(s, dt) {
  if (s.over) return;
  if (!s.started) return;
  s.disk.elapsed += dt;
  s.disk.angle = (s.disk.angle + currentSpeed(s) * dt) % (2 * Math.PI);
  if (s.disk.angle < 0) s.disk.angle += 2 * Math.PI;
  if (s.flying) {
    s.flying.y -= KNIFE_SPEED * dt;
    if (s.flying.y <= DISK_CY + DISK_R) {
      // Landing angle in disk-local coords. The knife arrives at the
      // 12-o'clock world position; the disk-local angle THERE is -disk.angle
      // (because as the disk turns by Δ, a point that was at -Δ in local
      // coords now sits at the top).
      let rel = (-s.disk.angle) % (2 * Math.PI);
      if (rel < 0) rel += 2 * Math.PI;
      // Apple pickup first - the knife passes through an apple without sticking.
      let appleHit = false;
      for (const a of s.apples) {
        if (a.alive && angularDist(a.relAngle, rel) < APPLE_TOL) {
          a.alive = false;
          s.score += 50;
          appleHit = true;
          break;
        }
      }
      if (!appleHit) {
        for (const k of s.stuck) {
          if (angularDist(k.relAngle, rel) < TOLERANCE) {
            s.over = true; s.won = false;
            return;
          }
        }
        s.stuck.push({ relAngle: rel });
        s.thrown++;
        s.score += 10;
        if (s.thrown >= s.cfg.knives) {
          s.over = true; s.won = true;
          s.score += s.queue * 5;       // small bonus for leftover knives (always 0 here)
          // Apple bonus already added above.
        }
      } else {
        // Apple absorbed the knife - that throw is still consumed but no penalty.
        // (queue already decremented when launched.)
      }
      s.flying = null;
    }
  }
}

function throwKnife(s) {
  if (s.over) return false;
  s.started = true;
  if (s.flying) return false;
  if (s.queue <= 0) return false;
  s.queue--;
  s.flying = { y: SPAWN_Y };
  return true;
}

function angularDist(a, b) {
  let d = Math.abs(a - b) % (2 * Math.PI);
  return Math.min(d, 2 * Math.PI - d);
}
