// Pixel Barkeep - a Tapper-style serving arcade.
//
// Patrons stride down four counters toward you. Slide a mug down a counter
// to shove the nearest patron back; shove one off the far end and they're
// served. Let a patron reach the bar and they grab you. Catch the empty
// mugs that come sliding back for a bonus. Endless escalating rounds.

const VW = 360, VH = 480;

const LANES = 4;
const LANE_TOP = 64, LANE_H = 92;
const BAR_X = 322;            // the barkeep's counter end
const LEFT_EDGE = 18;         // patrons spawn here; shoved past it = served
const PATRON_W = 26, MUG_W = 16;

const MUG_SPEED = 244;
const PUSH = 58;              // how far one mug shoves a patron back
const POUR_CD = 0.2;
const START_LIVES = 3;

// Patron kinds: walk speed (px/s, before the round multiplier) and score.
const KINDS = {
  regular:  { speed: 25, score: 50,  color: '#5f9bd0' },
  sluggard: { speed: 16, score: 40,  color: '#7a9a5a' },
  rowdy:    { speed: 41, score: 75,  color: '#d06a4a' },
};
const KIND_IDS = ['regular', 'sluggard', 'rowdy'];

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

function laneCenterY(lane) { return LANE_TOP + lane * LANE_H + LANE_H / 2; }

function buildGame(seed) {
  const s = {
    rng: seededRandom(seed || ((Date.now() & 0x7fffffff) || 1)),
    barkeepLane: 0,
    patrons: [], mugs: [],
    lives: START_LIVES,
    score: 0,
    round: 1,
    toSpawn: roundCount(1),
    spawnTimer: 0.8,
    pourCD: 0,
    roundBanner: 1.4,
    flash: 0,
    served: 0,
    over: false,
  };
  return s;
}

function roundCount(r) { return 5 + r * 2; }
function roundSpeedMul(r) { return 1 + (r - 1) * 0.08; }
function spawnGap(r) { return Math.max(0.55, 1.5 - r * 0.09); }

// ---- input -------------------------------------------------------------
function moveTo(s, lane) {
  if (s.over) return;
  s.barkeepLane = Math.max(0, Math.min(LANES - 1, lane));
}

function pour(s) {
  if (s.over || s.pourCD > 0) return false;
  s.mugs.push({ lane: s.barkeepLane, x: BAR_X - MUG_W, dir: -1 });
  s.pourCD = POUR_CD;
  return true;
}

// Tapping a counter: step to it and pour in one motion.
function serveLane(s, lane) {
  moveTo(s, lane);
  return pour(s);
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.flash > 0) s.flash = Math.max(0, s.flash - dt);
  if (s.over) return;
  if (s.roundBanner > 0) s.roundBanner = Math.max(0, s.roundBanner - dt);
  if (s.pourCD > 0) s.pourCD = Math.max(0, s.pourCD - dt);

  // Spawn patrons for this round.
  if (s.toSpawn > 0) {
    s.spawnTimer -= dt;
    if (s.spawnTimer <= 0) {
      spawnPatron(s);
      s.toSpawn--;
      s.spawnTimer = spawnGap(s.round) * (0.6 + s.rng() * 0.8);
    }
  }

  // Patrons advance toward the bar.
  const mul = roundSpeedMul(s.round);
  for (const p of s.patrons) {
    if (!p.alive) continue;
    p.x += KINDS[p.kind].speed * mul * dt;
    if (p.x + PATRON_W >= BAR_X) {        // reached the barkeep
      p.alive = false;
      loseLife(s);
    }
  }

  // Mugs slide; a poured mug shoves the first patron it overtakes.
  for (const m of s.mugs) {
    if (m.done) continue;
    m.x += m.dir * MUG_SPEED * dt;
    if (m.dir < 0) {
      // poured mug travelling left
      let hit = null;
      for (const p of s.patrons) {
        if (!p.alive || p.lane !== m.lane) continue;
        if (m.x <= p.x + PATRON_W && m.x + MUG_W >= p.x) {
          if (!hit || p.x > hit.x) hit = p;     // rightmost overtaken patron
        }
      }
      if (hit) {
        m.done = true;
        hit.x -= PUSH;
        if (hit.x + PATRON_W < LEFT_EDGE) {     // shoved clean off - served
          hit.alive = false;
          s.score += KINDS[hit.kind].score;
          s.served++;
          s.mugs.push({ lane: hit.lane, x: LEFT_EDGE, dir: 1 });   // empty mug returns
        }
      } else if (m.x <= LEFT_EDGE) {
        m.done = true;                          // poured into an empty lane - wasted
      }
    } else {
      // empty mug returning right - caught for a bonus if the barkeep is there
      if (m.x + MUG_W >= BAR_X) {
        m.done = true;
        if (s.barkeepLane === m.lane) s.score += 15;
      }
    }
  }
  s.patrons = s.patrons.filter(p => p.alive);
  s.mugs = s.mugs.filter(m => !m.done);

  // Round clear.
  if (s.toSpawn === 0 && s.patrons.length === 0 &&
      !s.mugs.some(m => m.dir < 0)) {
    s.score += 40 * s.round;
    s.round++;
    s.toSpawn = roundCount(s.round);
    s.spawnTimer = 0.7;
    s.roundBanner = 1.4;
  }
}

function spawnPatron(s) {
  const r = s.rng();
  let kind = 'regular';
  if (s.round >= 2 && r < 0.22)      kind = 'rowdy';
  else if (s.round >= 2 && r < 0.46) kind = 'sluggard';
  s.patrons.push({ lane: (s.rng() * LANES) | 0, x: LEFT_EDGE, kind, alive: true });
}

function loseLife(s) {
  s.lives--;
  s.flash = 0.5;
  if (s.lives <= 0) { s.lives = 0; s.over = true; }
}

function finalScore(s) { return s.score; }
