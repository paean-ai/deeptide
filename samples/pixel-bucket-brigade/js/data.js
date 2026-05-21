// Pixel Bucket Brigade - a Kaboom!-style catch arcade.
//
// A bomber paces the sky lobbing bombs. Slide your stack of buckets to
// catch every one before it reaches the rim. Miss a bomb and a bucket is
// lost - and the panic clears the sky. Catch power-ups for an extra
// bucket, a slow-motion window, or a wider magnet rim. Endless waves.

const VW = 360, VH = 480;

const RIM_Y       = 432;          // y of the top bucket's rim - the catch line
const CATCH_HALF  = 27;           // half-width of the catch zone
const MAGNET_HALF = 46;           // ...while a magnet is active
const BOMBER_Y    = 50;
const START_BUCKETS = 3, MAX_BUCKETS = 6;

// Bomb kinds.
const B_NORMAL = 'normal', B_FAST = 'fast', B_CLUSTER = 'cluster', B_GOLD = 'gold';
const CLUSTER_SPLIT_Y = 220;
// Power-up kinds.
const P_BUCKET = 'bucket', P_SLOW = 'slow', P_MAGNET = 'magnet';

const SCORE = { normal: 10, fast: 16, cluster: 14, gold: 60 };

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

function buildGame(seed) {
  const s = {
    rng: seededRandom(seed || ((Date.now() & 0x7fffffff) || 1)),
    stackX: VW / 2,
    buckets: START_BUCKETS,
    bombs: [], powerups: [],
    bomber: { x: VW / 2, dir: 1 },
    wave: 1,
    toDrop: waveCount(1),
    dropTimer: 1.0,
    score: 0,
    slow: 0, magnet: 0,
    waveBanner: 1.4,
    flash: 0,
    over: false,
  };
  return s;
}

function waveCount(w) { return 7 + w * 3; }
function bombSpeed(w)  { return 116 + w * 11; }
function bomberSpeed(w){ return 54 + w * 13; }
function dropGap(w)    { return Math.max(0.42, 1.25 - w * 0.07); }

// ---- input -------------------------------------------------------------
function setStackX(s, x) {
  if (s.over) return;
  s.stackX = Math.max(CATCH_HALF, Math.min(VW - CATCH_HALF, x));
}

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.flash > 0) s.flash = Math.max(0, s.flash - dt);
  if (s.over) return;
  if (s.waveBanner > 0) s.waveBanner = Math.max(0, s.waveBanner - dt);
  if (s.slow > 0)   s.slow   = Math.max(0, s.slow - dt);
  if (s.magnet > 0) s.magnet = Math.max(0, s.magnet - dt);
  const ts = s.slow > 0 ? 0.5 : 1;          // slow-motion scales the world

  // Bomber paces and lobs bombs.
  const b = s.bomber;
  b.x += b.dir * bomberSpeed(s.wave) * dt * ts;
  if (b.x < 40)        { b.x = 40;        b.dir = 1; }
  if (b.x > VW - 40)   { b.x = VW - 40;   b.dir = -1; }
  if (s.toDrop > 0) {
    s.dropTimer -= dt * ts;
    if (s.dropTimer <= 0) {
      dropBomb(s);
      s.toDrop--;
      s.dropTimer = dropGap(s.wave) * (0.7 + s.rng() * 0.6);
    }
  }

  const catchW = s.magnet > 0 ? MAGNET_HALF : CATCH_HALF;

  // Bombs fall; resolve each as it crosses the rim.
  const split = [];
  for (const bomb of s.bombs) {
    const v = bomb.kind === B_FAST ? bombSpeed(s.wave) * 1.7 : bombSpeed(s.wave);
    bomb.y += v * dt * ts;
    if (bomb.kind === B_CLUSTER && !bomb.didSplit && bomb.y >= CLUSTER_SPLIT_Y) {
      bomb.didSplit = true;
      bomb.dead = true;
      split.push({ x: bomb.x - 20, y: bomb.y, kind: B_NORMAL },
                  { x: bomb.x + 20, y: bomb.y, kind: B_NORMAL });
    }
  }
  for (const c of split) s.bombs.push({ x: c.x, y: c.y, kind: c.kind, didSplit: true });
  for (const bomb of s.bombs) {
    if (bomb.dead) continue;
    if (bomb.y >= RIM_Y) {
      bomb.dead = true;
      if (Math.abs(bomb.x - s.stackX) <= catchW) {
        s.score += SCORE[bomb.kind] || 10;
        bomb.caught = true;
      } else {
        missBomb(s);
        return;                              // miss wipes the sky; stop here
      }
    }
  }
  s.bombs = s.bombs.filter(x => !x.dead);

  // Power-ups fall; catching one applies its effect, missing one is free.
  for (const p of s.powerups) {
    p.y += 96 * dt * ts;
    if (p.y >= RIM_Y) {
      p.dead = true;
      if (Math.abs(p.x - s.stackX) <= catchW) applyPowerup(s, p.kind);
    }
  }
  s.powerups = s.powerups.filter(x => !x.dead);

  // Wave clear: every bomb dropped and the sky empty.
  if (s.toDrop === 0 && s.bombs.length === 0 && s.powerups.length === 0) {
    s.score += 25 * s.wave;                  // wave bonus
    s.wave++;
    s.toDrop = waveCount(s.wave);
    s.dropTimer = 0.8;
    s.waveBanner = 1.4;
  }
}

function dropBomb(s) {
  const r = s.rng();
  let kind = B_NORMAL;
  if (s.wave >= 2 && r < 0.06)      kind = B_GOLD;
  else if (s.wave >= 3 && r < 0.26) kind = B_CLUSTER;
  else if (s.wave >= 2 && r < 0.50) kind = B_FAST;
  s.bombs.push({ x: s.bomber.x, y: BOMBER_Y + 14, kind });
  // Sprinkle the occasional power-up.
  if (s.rng() < 0.12) {
    const pr = s.rng();
    const kind2 = pr < 0.4 ? P_BUCKET : (pr < 0.7 ? P_SLOW : P_MAGNET);
    s.powerups.push({ x: 30 + s.rng() * (VW - 60), y: BOMBER_Y, kind: kind2 });
  }
}

function missBomb(s) {
  s.buckets--;
  s.flash = 0.5;
  s.bombs = [];                              // the blast clears every falling bomb
  s.powerups = [];
  if (s.buckets <= 0) { s.buckets = 0; s.over = true; return; }
  // Resume the current wave with whatever bombs were left undropped.
  s.dropTimer = 1.0;
}

function applyPowerup(s, kind) {
  if (kind === P_BUCKET)      s.buckets = Math.min(MAX_BUCKETS, s.buckets + 1);
  else if (kind === P_SLOW)   s.slow = 5;
  else if (kind === P_MAGNET) s.magnet = 6;
  s.score += 8;
}

function finalScore(s) { return s.score; }
