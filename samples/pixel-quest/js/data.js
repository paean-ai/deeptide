// Pixel Quest - a turn-based JRPG party battler: heroes, skills, foes, combat.
//
// Three heroes fight a campaign of escalating encounters. Turn order is by
// speed; each hero may Attack, cast a Skill (MP), or Defend. Battles are
// self-contained - the party is restored at the start of every level - and the
// heroes' stats grow as the campaign goes on.

const VW = 360, VH = 480;

const HERO_DEFS = [
  { key: 'knight', name: ['Knight', '骑士'], hp: 40, mp: 12, atk: 11, def: 7, spd: 5, skill: 'cleave' },
  { key: 'mage',   name: ['Mage', '法师'],   hp: 24, mp: 30, atk: 8,  def: 4, spd: 8, skill: 'firestorm' },
  { key: 'cleric', name: ['Cleric', '牧师'], hp: 30, mp: 26, atk: 7,  def: 5, spd: 6, skill: 'mend' },
];
// per-level stat growth applied to every hero
const GROWTH = { hp: 7, mp: 3, atk: 2, def: 1, spd: 0 };

const SKILLS = {
  cleave:    { name: ['Cleave', '横扫'],     mp: 6,  kind: 'atkAll',  power: 0.85 },
  firestorm: { name: ['Firestorm', '烈焰'],  mp: 10, kind: 'magAll',  power: 10 },
  mend:      { name: ['Mend', '治愈'],       mp: 8,  kind: 'heal',    power: 24 },
};

// each level: a list of enemy stat blocks. ai 'focus' = hit weakest hero.
const LEVELS = [
  { name: ['Slime Hollow', '史莱姆洞'], enemies: [
    e('Slime', '史莱姆', 18, 7, 2, 3, 'random'),
    e('Slime', '史莱姆', 18, 7, 2, 3, 'random') ] },
  { name: ['Mossy Cave', '苔藓洞窟'], enemies: [
    e('Slime', '史莱姆', 20, 8, 2, 3, 'random'),
    e('Slime', '史莱姆', 20, 8, 2, 3, 'random'),
    e('Slime', '史莱姆', 20, 8, 2, 3, 'random') ] },
  { name: ['Goblin Camp', '哥布林营'], enemies: [
    e('Goblin', '哥布林', 30, 11, 4, 6, 'focus'),
    e('Goblin', '哥布林', 30, 11, 4, 6, 'focus'),
    e('Slime', '史莱姆', 22, 9, 2, 3, 'random') ] },
  { name: ['Wolf Den', '狼穴'], enemies: [
    e('Wolf', '野狼', 26, 13, 3, 10, 'focus'),
    e('Wolf', '野狼', 26, 13, 3, 10, 'focus'),
    e('Goblin', '哥布林', 34, 12, 5, 6, 'focus') ] },
  { name: ['Howling Pass', '嚎风隘口'], enemies: [
    e('Wolf', '野狼', 30, 15, 4, 10, 'focus'),
    e('Wolf', '野狼', 30, 15, 4, 10, 'focus'),
    e('Wolf', '野狼', 30, 15, 4, 10, 'focus') ] },
  { name: ['Ogre Bridge', '巨魔桥'], enemies: [
    e('Ogre', '巨魔', 78, 18, 6, 4, 'focus'),
    e('Goblin', '哥布林', 40, 14, 5, 6, 'focus'),
    e('Goblin', '哥布林', 40, 14, 5, 6, 'focus') ] },
  { name: ['Beast Warren', '兽窟'], enemies: [
    e('Ogre', '巨魔', 92, 20, 7, 4, 'focus'),
    e('Wolf', '野狼', 38, 17, 5, 11, 'focus'),
    e('Wolf', '野狼', 38, 17, 5, 11, 'focus') ] },
  { name: ['Dragon Lair', '巨龙巢穴'], enemies: [
    e('Dragon', '巨龙', 180, 23, 9, 7, 'focus'),
    e('Wolf', '野狼', 44, 18, 6, 11, 'random') ] },
];
const LEVEL_COUNT = LEVELS.length;

function e(en, zh, hp, atk, def, spd, ai) {
  return { name: [en, zh], hp, atk, def, spd, ai };
}

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// ---- battle setup --------------------------------------------------------
function buildBattle(levelIndex) {
  const L = LEVELS[levelIndex];
  let uid = 0;
  const heroes = HERO_DEFS.map((d, i) => ({
    uid: uid++, side: 'hero', key: d.key, name: d.name, slot: i,
    maxhp: d.hp + GROWTH.hp * levelIndex,
    maxmp: d.mp + GROWTH.mp * levelIndex,
    atk: d.atk + GROWTH.atk * levelIndex,
    def: d.def + GROWTH.def * levelIndex,
    spd: d.spd, skill: d.skill, defending: false,
  }));
  heroes.forEach(h => { h.hp = h.maxhp; h.mp = h.maxmp; });
  const enemies = L.enemies.map((d, i) => ({
    uid: uid++, side: 'enemy', name: d.name, slot: i,
    maxhp: d.hp, hp: d.hp, atk: d.atk, def: d.def, spd: d.spd,
    ai: d.ai, defending: false,
  }));
  const s = {
    index: levelIndex, heroes, enemies,
    round: 0, queue: [], qi: 0, over: false, won: false,
    rng: seededRandom(1009 + levelIndex * 631), log: '',
  };
  newRound(s);
  return s;
}

function alive(list) { return list.filter(u => u.hp > 0); }
function allUnits(s) { return s.heroes.concat(s.enemies); }

function newRound(s) {
  s.round++;
  s.queue = alive(allUnits(s)).sort((a, b) =>
    b.spd - a.spd || (a.side === b.side ? a.slot - b.slot : a.side === 'hero' ? -1 : 1));
  s.qi = 0;
  primeTurn(s);
}

// the unit whose turn it is (null if round finished)
function currentUnit(s) {
  while (s.qi < s.queue.length && s.queue[s.qi].hp <= 0) s.qi++;
  return s.qi < s.queue.length ? s.queue[s.qi] : null;
}
function primeTurn(s) {
  const u = currentUnit(s);
  if (u) u.defending = false;   // a Defend lasts until the unit's next turn
}

function advance(s) {
  if (s.over) return;
  s.qi++;
  if (!currentUnit(s)) newRound(s);
  else primeTurn(s);
  checkOver(s);
}

// ---- damage --------------------------------------------------------------
function vary(s, base) {
  const d = Math.round(base * (0.85 + s.rng() * 0.3));
  return Math.max(1, d);
}
function hit(s, target, raw) {
  let dmg = Math.max(1, raw);
  if (target.defending) dmg = Math.max(1, Math.floor(dmg / 2));
  target.hp = Math.max(0, target.hp - dmg);
  return dmg;
}

// ---- hero actions --------------------------------------------------------
function physDamage(s, attacker, target) {
  return hit(s, target, vary(s, attacker.atk - target.def));
}
function heroAttack(s, target) {
  const u = currentUnit(s);
  if (!u || u.side !== 'hero' || target.hp <= 0) return false;
  physDamage(s, u, target);
  advance(s);
  return true;
}
function canCast(u) {
  return u && u.skill && SKILLS[u.skill] && u.mp >= SKILLS[u.skill].mp;
}
function heroSkill(s, target) {
  const u = currentUnit(s);
  if (!u || u.side !== 'hero' || !canCast(u)) return false;
  const sk = SKILLS[u.skill];
  u.mp -= sk.mp;
  if (sk.kind === 'atkAll') {
    for (const en of alive(s.enemies)) hit(s, en, vary(s, u.atk * sk.power - en.def));
  } else if (sk.kind === 'magAll') {
    for (const en of alive(s.enemies)) hit(s, en, vary(s, sk.power + u.atk * 0.6));
  } else if (sk.kind === 'heal') {
    if (!target || target.side !== 'hero' || target.hp <= 0) { u.mp += sk.mp; return false; }
    target.hp = Math.min(target.maxhp, target.hp + sk.power + Math.round(u.atk * 0.8));
  }
  advance(s);
  return true;
}
function heroDefend(s) {
  const u = currentUnit(s);
  if (!u || u.side !== 'hero') return false;
  u.defending = true;
  advance(s);
  return true;
}

// ---- enemy turn ----------------------------------------------------------
function enemyAct(s) {
  const u = currentUnit(s);
  if (!u || u.side !== 'enemy') return;
  const foes = alive(s.heroes);
  if (foes.length) {
    let target;
    if (u.ai === 'focus') {
      target = foes.reduce((a, b) => (b.hp < a.hp ? b : a));
    } else {
      target = foes[(s.rng() * foes.length) | 0];
    }
    physDamage(s, u, target);
  }
  advance(s);
}

function checkOver(s) {
  if (alive(s.enemies).length === 0) { s.over = true; s.won = true; return true; }
  if (alive(s.heroes).length === 0) { s.over = true; s.won = false; return true; }
  return false;
}
