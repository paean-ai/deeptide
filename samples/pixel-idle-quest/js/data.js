// Pixel Idle Quest - an idle monster-slaying RPG.
//
// A lone hero grinds an endless dungeon. Tap the monster to strike it;
// your squires deal damage on their own even while you rest. Gold buys a
// sharper blade and more squires. Stuck? Ascend - reset the run for relics
// that permanently multiply all your damage.

const VW = 360, VH = 480;

const KILLS_PER_STAGE = 10;
const MAX_OFFLINE = 8 * 3600;          // cap idle catch-up at 8 hours

// ---- economy curves ----------------------------------------------------
// Monster HP and gold both grow at the same exponential rate; upgrades
// grow damage a touch slower than their cost grows, so progress gradually
// slows and ascending becomes worthwhile.
const HP_GROWTH = 1.38, GOLD_GROWTH = 1.38;
const DMG_MULT = 1.17, COST_MULT = 1.18;   // per upgrade level

function monsterMaxHp(stage) {
  let hp = 14 * Math.pow(HP_GROWTH, stage - 1);
  if (stage % 5 === 0) hp *= 2.5;            // elite stage
  return hp;
}
function killGold(stage) {
  let g = 6 * Math.pow(GOLD_GROWTH, stage - 1);
  if (stage % 5 === 0) g *= 2.5;
  return g;
}
function bladeCost(s)  { return 10 * Math.pow(COST_MULT, s.blade); }
function squireCost(s) { return 14 * Math.pow(COST_MULT, s.squire); }

function prestigeMult(s) { return 1 + 0.5 * s.relics; }
function tapDamage(s)    { return 4 * Math.pow(DMG_MULT, s.blade)  * prestigeMult(s); }
function autoDps(s)      { return 2 * Math.pow(DMG_MULT, s.squire) * prestigeMult(s); }

// Relics earned by ascending from a given best stage.
function relicsFor(bestStage) { return Math.max(0, Math.floor((bestStage - 6) / 4)); }
function canAscend(s) { return relicsFor(s.bestStage) > 0; }

function buildGame(saved) {
  const s = {
    gold: 0, stage: 1, kills: 0,
    blade: 0, squire: 0,
    relics: 0, bestStage: 1, lifetimeBest: 1,
    monsterHp: 0, monsterMax: 0,
    flash: 0,
  };
  if (saved) {
    for (const k of ['gold','stage','kills','blade','squire','relics','bestStage','lifetimeBest']) {
      if (typeof saved[k] === 'number' && isFinite(saved[k])) s[k] = saved[k];
    }
  }
  spawnMonster(s);
  return s;
}

function spawnMonster(s) {
  s.monsterMax = monsterMaxHp(s.stage);
  s.monsterHp = s.monsterMax;
}

// ---- combat ------------------------------------------------------------
function hitMonster(s, dmg) {
  s.monsterHp -= dmg;
  if (s.monsterHp <= 0) killMonster(s);
}

function tap(s)      { hitMonster(s, tapDamage(s)); }
function tick(s, dt) {
  if (s.flash > 0) s.flash = Math.max(0, s.flash - dt);
  hitMonster(s, autoDps(s) * dt);
}

function killMonster(s) {
  s.gold += killGold(s.stage);
  s.flash = 0.25;
  s.kills++;
  if (s.kills >= KILLS_PER_STAGE) {
    s.kills = 0;
    s.stage++;
    if (s.stage > s.bestStage) s.bestStage = s.stage;
    if (s.stage > s.lifetimeBest) s.lifetimeBest = s.stage;
  }
  spawnMonster(s);
}

// ---- upgrades ----------------------------------------------------------
function buyBlade(s)  { const c = bladeCost(s);  if (s.gold >= c) { s.gold -= c; s.blade++;  return true; } return false; }
function buySquire(s) { const c = squireCost(s); if (s.gold >= c) { s.gold -= c; s.squire++; return true; } return false; }

function ascend(s) {
  if (!canAscend(s)) return false;
  s.relics += relicsFor(s.bestStage);
  s.gold = 0; s.stage = 1; s.kills = 0;
  s.blade = 0; s.squire = 0;
  s.bestStage = 1;
  spawnMonster(s);
  return true;
}

// ---- idle catch-up -----------------------------------------------------
// Estimate the gold the squires would have ground out while you were away.
function offlineEarnings(s, seconds) {
  const t = Math.min(Math.max(0, seconds), MAX_OFFLINE);
  const killsPerSec = autoDps(s) / Math.max(1, monsterMaxHp(s.stage));
  return killsPerSec * killGold(s.stage) * t;
}

// ---- number formatting -------------------------------------------------
const SUFFIX = ['', 'K', 'M', 'B', 'T', 'aa', 'ab', 'ac', 'ad', 'ae', 'af', 'ag', 'ah'];
function fmt(n) {
  if (!isFinite(n)) return '∞';
  if (n < 1000) return (n < 10 ? n.toFixed(1) : Math.floor(n).toString());
  let tier = 0;
  while (n >= 1000 && tier < SUFFIX.length - 1) { n /= 1000; tier++; }
  return n.toFixed(2) + SUFFIX[tier];
}
