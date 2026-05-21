// Pixel Duelist - parry / dodge timing-duel boss rush. Pure game logic.
//
// A foe winds up an attack along a timing track. A blue slash must be
// PARRIED, an amber thrust must be DODGED - and the input only counts in the
// react window near the end of the windup. Land it in the last sliver for a
// PERFECT (double posture). Fill the foe's posture to stagger them, then
// EXECUTE for a point of health. Read the blade, do not just mash.

const VW = 360, VH = 480;
const PLAYER_HP = 6;
const PERFECT_WIN = 0.19;   // pressing within this of the strike = a PERFECT
const STAGGER_WIN = 1.6;    // seconds the EXECUTE prompt stays open

// Each foe: hp = executes needed, windup = base telegraph length (shorter is
// faster), react = how wide the success window is, thrustRate = share of
// dodge attacks, postureMax = posture to stagger, dmg = health lost per hit.
const BOSSES = [
  { name: ['Recruit',  '新兵'],   seed: 211, hp: 3, windup: 1.34, react: 0.46, thrustRate: 0.18, postureMax: 4, dmg: 1 },
  { name: ['Sentinel', '哨卫'],   seed: 367, hp: 4, windup: 1.17, react: 0.41, thrustRate: 0.27, postureMax: 5, dmg: 1 },
  { name: ['Duelist',  '决斗者'], seed: 521, hp: 4, windup: 1.03, react: 0.37, thrustRate: 0.35, postureMax: 6, dmg: 1 },
  { name: ['Warden',   '典狱长'], seed: 677, hp: 5, windup: 0.93, react: 0.33, thrustRate: 0.42, postureMax: 6, dmg: 2 },
  { name: ['Champion', '冠军'],   seed: 829, hp: 5, windup: 0.84, react: 0.30, thrustRate: 0.48, postureMax: 7, dmg: 2 },
  { name: ['Revenant', '亡魂'],   seed: 983, hp: 6, windup: 0.74, react: 0.27, thrustRate: 0.54, postureMax: 8, dmg: 2 },
];
const BOSS_COUNT = BOSSES.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function newDuel(bossIndex) {
  const cfg = BOSSES[bossIndex];
  return {
    bossIndex, cfg,
    rng: seededRandom(cfg.seed),
    playerHP: PLAYER_HP,
    bossHP: cfg.hp,
    posture: 0,
    phase: 'gap',          // gap | attack | stagger
    gapT: 1.0,             // opening pause before the first attack
    staggerT: 0,
    atk: null,             // { kind:'slash'|'thrust', windup, t, done }
    flash: null,           // { kind, t, dur } - transient feedback for art
    combo: 0, bestCombo: 0,
    hitsTaken: 0, parries: 0, dodges: 0, perfects: 0,
    over: false, won: false,
  };
}

function nextGap(s) {
  s.phase = 'gap';
  s.gapT = 0.40 + s.rng() * 0.42;
  s.atk = null;
}

function spawnAttack(s) {
  const cfg = s.cfg;
  const kind = s.rng() < cfg.thrustRate ? 'thrust' : 'slash';
  const windup = cfg.windup * (0.82 + s.rng() * 0.36);
  s.atk = { kind, windup, t: 0, done: false };
  s.phase = 'attack';
}

function hitPlayer(s) {
  s.playerHP -= s.cfg.dmg;
  s.hitsTaken++;
  s.combo = 0;
  s.flash = { kind: 'hurt', t: 0.40, dur: 0.40 };
  if (s.playerHP <= 0) { s.playerHP = 0; s.over = true; s.won = false; }
}

function doExecute(s) {
  s.bossHP -= 1;
  s.posture = 0;
  s.flash = { kind: 'execute', t: 0.55, dur: 0.55 };
  if (s.bossHP <= 0) {
    s.bossHP = 0; s.over = true; s.won = true;
    return { result: 'execute', killed: true };
  }
  nextGap(s);
  return { result: 'execute' };
}

function tick(s, dt) {
  if (s.over) return;
  if (s.flash) { s.flash.t -= dt; if (s.flash.t <= 0) s.flash = null; }
  if (s.phase === 'gap') {
    s.gapT -= dt;
    if (s.gapT <= 0) spawnAttack(s);
  } else if (s.phase === 'attack') {
    s.atk.t += dt;
    if (s.atk.t >= s.atk.windup) {     // windup finished with no parry -> hit lands
      hitPlayer(s);
      if (!s.over) nextGap(s);
    }
  } else if (s.phase === 'stagger') {
    s.staggerT -= dt;
    if (s.staggerT <= 0) {             // missed the execute window -> foe recovers
      s.posture = 0;
      nextGap(s);
    }
  }
}

// type: 'parry' | 'dodge' | 'execute'. Returns a small result object or null.
function input(s, type) {
  if (s.over) return null;
  if (s.phase === 'stagger') {
    return type === 'execute' ? doExecute(s) : null;
  }
  if (s.phase !== 'attack' || !s.atk || s.atk.done) return null;
  const atk = s.atk;
  const correct = (type === 'parry' && atk.kind === 'slash') ||
                  (type === 'dodge' && atk.kind === 'thrust');
  const inWindow = atk.t >= atk.windup - s.cfg.react;
  if (correct && inWindow) {
    atk.done = true;
    const perfect = atk.t >= atk.windup - PERFECT_WIN;
    s.combo++;
    if (s.combo > s.bestCombo) s.bestCombo = s.combo;
    if (type === 'parry') s.parries++; else s.dodges++;
    if (perfect) s.perfects++;
    s.posture += perfect ? 2 : 1;
    s.flash = { kind: perfect ? 'perfect' : type, t: 0.34, dur: 0.34 };
    s.atk = null;
    if (s.posture >= s.cfg.postureMax) {
      s.phase = 'stagger';
      s.staggerT = STAGGER_WIN;
      return { result: perfect ? 'perfect' : 'good', stagger: true };
    }
    nextGap(s);
    return { result: perfect ? 'perfect' : 'good' };
  }
  // a misread or a too-early press: the input is spent and the rhythm breaks;
  // the strike still lands when the windup completes (resolved in tick).
  atk.done = true;
  s.combo = 0;
  s.flash = { kind: 'whiff', t: 0.26, dur: 0.26 };
  return { result: correct ? 'early' : 'wrong' };
}

function stars(hitsTaken) {
  if (hitsTaken === 0) return 3;
  if (hitsTaken <= 2) return 2;
  return 1;
}
