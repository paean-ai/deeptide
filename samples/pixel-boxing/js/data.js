// Pixel Boxing - a Punch-Out-style timing duel. The opponent telegraphs
// each punch (a wind-up with a left/right tell); dodge the correct way
// to make them whiff and stagger, opening a counter window where your
// punches land for triple damage. Mistime it and you eat the hit.
//
// Opponent state machine:
//   idle    -> windup (after a short rest; picks a side, shows the tell)
//   windup  -> strike (the punch lands NOW unless dodged)
//   strike  -> stagger (if the player dodged correctly) | recover (if it hit)
//   stagger -> recover (counter window closes)
//   recover -> idle

const VW = 360, VH = 480;

// 6 opponents on a rising curve: faster wind-up = less reaction time,
// more HP, harder hits.
const LEVELS = [
  { name: ['Glass Joe',  '玻璃乔'], hp: 60,  windup: 1.10, strike: 0.34, hit: 12, color: '#9ad1ff' },
  { name: ['Bald Bull',  '光头牛'], hp: 80,  windup: 0.95, strike: 0.30, hit: 15, color: '#ffb24a' },
  { name: ['Iron Mike',  '铁麦克'], hp: 100, windup: 0.82, strike: 0.27, hit: 18, color: '#ff7a7a' },
  { name: ['Don Flame',  '烈焰唐'], hp: 120, windup: 0.70, strike: 0.24, hit: 21, color: '#ff5fae' },
  { name: ['Tiger Sanda','虎三打'], hp: 145, windup: 0.60, strike: 0.21, hit: 24, color: '#bda6ff' },
  { name: ['Mr. Pixel',  '像素先生'], hp: 175, windup: 0.50, strike: 0.18, hit: 28, color: '#ffd34a' },
];
const LEVEL_COUNT = LEVELS.length;

const PLAYER_HP = 100;
const JAB_DMG = 6;             // a clean jab on an idle/recovering foe
const COUNTER_DMG = 26;        // a counter punch during the stagger window
const STAGGER_TIME = 0.9;      // how long the counter window stays open
const RECOVER_TIME = 0.5;
const REST_MIN = 0.5, REST_RANGE = 0.8;
const PLAYER_DODGE_TIME = 0.45; // how long a dodge pose lasts
const PLAYER_PUNCH_TIME = 0.28;
const PLAYER_BLOCK_DMG = 0.35;  // fraction of damage taken while blocking

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () { s = (s * 16807) % 2147483647; return (s - 1) / 2147483646; };
}

// ---- runtime state -----------------------------------------------------
function buildGame(levelIndex) {
  const cfg = LEVELS[levelIndex];
  return {
    levelIndex, cfg,
    rng: seededRandom(levelIndex * 53 + 17),
    foe: { hp: cfg.hp, maxhp: cfg.hp, state: 'idle', t: REST_MIN, tell: null, hitFlash: 0 },
    player: {
      hp: PLAYER_HP, maxhp: PLAYER_HP,
      pose: 'idle',        // 'idle' | 'dodgeL' | 'dodgeR' | 'punchL' | 'punchR' | 'block'
      poseT: 0,
      hitFlash: 0,
    },
    combo: 0,
    score: 0,
    over: false, won: false,
    flash: 0,
    msg: '',
    msgT: 0,
  };
}

// ---- input -------------------------------------------------------------
// action: 'dodgeL' | 'dodgeR' | 'punchL' | 'punchR' | 'block'
function input(s, action) {
  if (s.over) return false;
  const p = s.player;
  // Can't act mid-pose (except block can be released; here block is a tap).
  if (p.pose !== 'idle') return false;
  if (action === 'dodgeL' || action === 'dodgeR') {
    p.pose = action; p.poseT = PLAYER_DODGE_TIME;
    resolveDodge(s, action);
    return true;
  }
  if (action === 'block') {
    p.pose = 'block'; p.poseT = PLAYER_DODGE_TIME;
    return true;
  }
  if (action === 'punchL' || action === 'punchR') {
    p.pose = action; p.poseT = PLAYER_PUNCH_TIME;
    resolvePunch(s);
    return true;
  }
  return false;
}

// A dodge fired while the foe is winding up / striking: if it goes the
// OPPOSITE way to the foe's punch, the foe whiffs and staggers.
function resolveDodge(s, action) {
  const f = s.foe;
  if ((f.state === 'windup' || f.state === 'strike') && f.tell) {
    // tell 'L' means the foe punches toward the player's left; dodging
    // RIGHT evades it (and vice-versa).
    const evaded = (f.tell === 'L' && action === 'dodgeR') ||
                   (f.tell === 'R' && action === 'dodgeL');
    if (evaded) {
      f.state = 'stagger';
      f.t = STAGGER_TIME;
      f.tell = null;
      setMsg(s, 'DODGE!');
    }
  }
}

function resolvePunch(s) {
  const f = s.foe;
  if (f.state === 'stagger') {
    // Counter punch — triple-ish damage + combo.
    s.combo++;
    const dmg = COUNTER_DMG + s.combo * 2;
    hurtFoe(s, dmg);
    setMsg(s, 'COUNTER x' + s.combo);
    s.score += 100;
  } else if (f.state === 'idle' || f.state === 'recover') {
    // A clean jab.
    hurtFoe(s, JAB_DMG);
    s.score += 20;
  } else {
    // Punching into a wind-up / strike — you trade and eat the hit.
    s.combo = 0;
    setMsg(s, 'BLOCKED');
  }
}

function hurtFoe(s, dmg) {
  const f = s.foe;
  f.hp = Math.max(0, f.hp - dmg);
  f.hitFlash = 0.25;
  s.flash = 0.2;
  if (f.hp <= 0) {
    s.over = true; s.won = true;
    s.score += 300 + s.player.hp * 3;
  }
}

function hurtPlayer(s, dmg) {
  const p = s.player;
  const real = p.pose === 'block' ? Math.ceil(dmg * PLAYER_BLOCK_DMG) : dmg;
  p.hp = Math.max(0, p.hp - real);
  p.hitFlash = 0.4;
  s.flash = 0.35;
  s.combo = 0;
  if (p.hp <= 0) { s.over = true; s.won = false; }
}

function setMsg(s, m) { s.msg = m; s.msgT = 0.8; }

// ---- tick --------------------------------------------------------------
function tick(s, dt) {
  if (s.over) return;
  s.flash = Math.max(0, s.flash - dt);
  if (s.msgT > 0) s.msgT = Math.max(0, s.msgT - dt);
  const p = s.player, f = s.foe;
  if (p.hitFlash > 0) p.hitFlash = Math.max(0, p.hitFlash - dt);
  if (f.hitFlash > 0) f.hitFlash = Math.max(0, f.hitFlash - dt);
  // Player pose timer.
  if (p.pose !== 'idle') {
    p.poseT -= dt;
    if (p.poseT <= 0) { p.pose = 'idle'; p.poseT = 0; }
  }
  // Foe state machine.
  f.t -= dt;
  if (f.t <= 0) {
    if (f.state === 'idle') {
      f.state = 'windup';
      f.tell = s.rng() < 0.5 ? 'L' : 'R';
      f.t = s.cfg.windup;
    } else if (f.state === 'windup') {
      f.state = 'strike';
      f.t = s.cfg.strike;
    } else if (f.state === 'strike') {
      // The punch lands unless the player evaded (which already flipped
      // the foe to 'stagger'). A correct dodge pose also saves the player.
      const dodged = (f.tell === 'L' && p.pose === 'dodgeR') ||
                     (f.tell === 'R' && p.pose === 'dodgeL');
      if (!dodged) {
        hurtPlayer(s, s.cfg.hit);
        setMsg(s, 'HIT!');
      }
      f.state = 'recover';
      f.tell = null;
      f.t = RECOVER_TIME;
    } else if (f.state === 'stagger') {
      f.state = 'recover';
      f.t = RECOVER_TIME;
    } else { // recover
      f.state = 'idle';
      f.t = REST_MIN + s.rng() * REST_RANGE;
    }
  }
}

function finalScore(s) {
  return s.score + (s.won ? Math.max(0, s.player.hp) * 2 : 0);
}
