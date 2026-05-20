// Pixel Burrow - a whack-a-mole arcade. Critters pop from a 3x3 of burrows;
// bonk the good ones, leave the bombs alone, beat the score target in time.

const VW = 360, VH = 480;
const BURROWS = 9;   // 3x3

// type: gopher (common), golden (rare bonus), bomb (never tap), owl
// (rare; +5 seconds of timer on a clean bonk so a fading run can recover).
const LEVELS = [
  { name: ['Sprout Field', '新芽田'], seed: 11, target: 800,  duration: 40, spawn: 1.00, up: 1.5,  bomb: 0.06, gold: 0.10, owl: 0.04 },
  { name: ['Clover Lawn', '苜蓿坪'],  seed: 27, target: 950,  duration: 40, spawn: 0.90, up: 1.4,  bomb: 0.08, gold: 0.10, owl: 0.05 },
  { name: ['Turnip Patch', '萝卜地'], seed: 44, target: 1150,  duration: 42, spawn: 0.80, up: 1.3,  bomb: 0.10, gold: 0.11, owl: 0.05 },
  { name: ['Pumpkin Plot', '南瓜园'], seed: 63, target: 1350,  duration: 42, spawn: 0.74, up: 1.2,  bomb: 0.12, gold: 0.12, owl: 0.06 },
  { name: ['Berry Thicket', '莓丛'],  seed: 88, target: 1300,  duration: 44, spawn: 0.66, up: 1.1,  bomb: 0.14, gold: 0.12, owl: 0.06 },
  { name: ['Orchard Row', '果园畦'],  seed: 115, target: 1600, duration: 44, spawn: 0.60, up: 1.05, bomb: 0.16, gold: 0.12, owl: 0.07 },
  { name: ['Meadow Maze', '草甸阵'],  seed: 147, target: 1900, duration: 45, spawn: 0.55, up: 0.95, bomb: 0.18, gold: 0.13, owl: 0.07 },
  { name: ['Harvest Moon', '丰收月'], seed: 182, target: 2200, duration: 48, spawn: 0.50, up: 0.90, bomb: 0.20, gold: 0.13, owl: 0.08 },
];
const LEVEL_COUNT = LEVELS.length;

const SCORE = { gopher: 10, golden: 50, owl: 15 };
const OWL_TIME_BONUS = 5;
const COMBO_MAX = 5;
const START_LIVES = 3;

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
  return {
    index: levelIndex, cfg,
    burrows: new Array(BURROWS).fill(null),
    score: 0, combo: 1, lives: START_LIVES,
    timeLeft: cfg.duration, spawnTimer: 0.6,
    over: false, won: false,
    hits: 0, bonked: 0, popped: 0,
    rng: seededRandom(cfg.seed),
    flash: 0,
  };
}

function spawnCritter(s) {
  const free = [];
  for (let i = 0; i < BURROWS; i++) if (!s.burrows[i]) free.push(i);
  if (!free.length) return;
  const i = free[(s.rng() * free.length) | 0];
  const roll = s.rng();
  const owlChance = s.cfg.owl || 0;
  let type;
  if (roll < s.cfg.bomb) type = 'bomb';
  else if (roll < s.cfg.bomb + s.cfg.gold) type = 'golden';
  else if (roll < s.cfg.bomb + s.cfg.gold + owlChance) type = 'owl';
  else type = 'gopher';
  // Owls stay up a touch longer than gophers; goldens are quick; bombs linger.
  const life = type === 'golden' ? s.cfg.up * 0.72 :
               type === 'bomb'   ? s.cfg.up * 1.15 :
               type === 'owl'    ? s.cfg.up * 1.0  : s.cfg.up;
  s.burrows[i] = { type, age: 0, life, maxlife: life };
  s.popped++;
}

// advance the game by dt seconds
function tick(s, dt) {
  if (s.over) return;
  if (s.flash > 0) s.flash = Math.max(0, s.flash - dt);
  s.timeLeft -= dt;
  if (s.timeLeft <= 0) {
    s.timeLeft = 0;
    finish(s);
    return;
  }
  s.spawnTimer -= dt;
  if (s.spawnTimer <= 0) {
    spawnCritter(s);
    s.spawnTimer = s.cfg.spawn * (0.8 + s.rng() * 0.45);
  }
  for (let i = 0; i < BURROWS; i++) {
    const c = s.burrows[i];
    if (!c) continue;
    c.age += dt;
    if (c.age >= c.life) {
      // a good critter that retracted unbonked breaks the combo
      if (c.type !== 'bomb') s.combo = 1;
      s.burrows[i] = null;
    }
  }
}

// bonk burrow i; returns the gained score (0 if empty / bomb)
function bonk(s, i) {
  if (s.over) return 0;
  const c = s.burrows[i];
  if (!c) return 0;
  s.burrows[i] = null;
  if (c.type === 'bomb') {
    s.lives--;
    s.combo = 1;
    s.flash = 0.35;
    if (s.lives <= 0) finish(s);
    return 0;
  }
  const gain = SCORE[c.type] * s.combo;
  s.score += gain;
  s.combo = Math.min(COMBO_MAX, s.combo + 1);
  s.hits++;
  s.bonked += SCORE[c.type];
  // Owls additionally extend the timer.
  if (c.type === 'owl') s.timeLeft += OWL_TIME_BONUS;
  return gain;
}

function finish(s) {
  s.over = true;
  s.won = s.lives > 0 && s.score >= s.cfg.target;
}
