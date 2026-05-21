// Pixel Elemancer - turn-based elemental-form battle RPG. Pure battle logic.
//
// One hero, four elemental forms - Ember, Bramble, Gale, Tide - each with its
// own health pool, attack power and a once-per-battle special. Elements run a
// four-cycle: fire > grass > storm > water > fire. Striking in a super form
// hits hard; the same form also resists that foe. A voluntary shift costs the
// turn (the foe acts); a forced shift after a knock-out is free. GUARD halves
// the incoming hit and empowers the next strike. Drop the foe before all four
// forms are spent.

const VW = 360, VH = 480;

// element indices: 0 fire, 1 grass, 2 storm, 3 water. Each beats the next.
const ELEM_NAME = [['Fire', '火'], ['Grass', '草'], ['Storm', '雷'], ['Water', '水']];
function beats(a) { return (a + 1) % 4; }
function typeMult(atk, def) {
  if (beats(atk) === def) return 1.6;      // super-effective
  if (beats(def) === atk) return 0.625;    // resisted
  return 1.0;                              // neutral
}
function effLabel(atk, def) {
  if (beats(atk) === def) return 'super';
  if (beats(def) === atk) return 'weak';
  return 'normal';
}

const GUARD_MULT = 0.5;       // incoming damage when guarding
const EMPOWER_MULT = 1.6;     // next strike after a guard

const FORMS = [
  { key: 'ember',   elem: 0, name: ['Ember', '焰烬'],   maxHP: 30, power: 12, special: 'pyre' },
  { key: 'bramble', elem: 1, name: ['Bramble', '荆藤'], maxHP: 46, power: 9,  special: 'bulwark' },
  { key: 'gale',    elem: 2, name: ['Gale', '疾岚'],    maxHP: 36, power: 11, special: 'tempo' },
  { key: 'tide',    elem: 3, name: ['Tide', '潮汐'],    maxHP: 40, power: 10, special: 'mend' },
];

const FOES = [
  { name: ['Sprout', '萌芽'],    elem: 1, hp: 44,  power: 6,  chargeEvery: 0, chargeMult: 1 },
  { name: ['Gust', '疾风'],      elem: 2, hp: 58,  power: 8,  chargeEvery: 0, chargeMult: 1 },
  { name: ['Brine', '咸潮'],     elem: 3, hp: 74,  power: 9,  chargeEvery: 0, chargeMult: 1 },
  { name: ['Cinder', '炽烬'],    elem: 0, hp: 92,  power: 11, chargeEvery: 0, chargeMult: 1 },
  { name: ['Thornlord', '荆王'], elem: 1, hp: 108, power: 11, chargeEvery: 3, chargeMult: 1.6 },
  { name: ['Maelstrom', '涡神'], elem: 2, hp: 124, power: 12, chargeEvery: 3, chargeMult: 1.7 },
];
const FOE_COUNT = FOES.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function newBattle(foeIndex) {
  const fc = FOES[foeIndex];
  return {
    foeIndex,
    foe: { name: fc.name, elem: fc.elem, maxHP: fc.hp, hp: fc.hp, power: fc.power,
           chargeEvery: fc.chargeEvery, chargeMult: fc.chargeMult, turnsTaken: 0 },
    forms: FORMS.map(f => ({
      key: f.key, elem: f.elem, name: f.name, maxHP: f.maxHP, hp: f.maxHP,
      power: f.power, special: f.special, specialUsed: false, exhausted: false,
    })),
    current: 0,
    guarding: false,
    empowered: false,
    mustShift: false,
    rng: seededRandom(7919 + foeIndex * 613),
    turn: 0,
    over: false, won: false,
  };
}

function variance(b) { return 0.9 + b.rng() * 0.2; }
function rollDamage(power, atk, def, v) {
  return Math.max(1, Math.round(power * typeMult(atk, def) * v));
}
// consumed once per offensive action: an empowered strike hits harder
function offenseMult(b) {
  if (b.empowered) { b.empowered = false; return EMPOWER_MULT; }
  return 1;
}

function nextFoeCharged(b) {
  return b.foe.chargeEvery > 0 && ((b.foe.turnsTaken + 1) % b.foe.chargeEvery === 0);
}
// estimated next foe hit on the current form (mid variance)
function foeThreat(b) {
  const cur = b.forms[b.current];
  let raw = b.foe.power * typeMult(b.foe.elem, cur.elem);
  if (nextFoeCharged(b)) raw *= b.foe.chargeMult;
  return Math.max(1, Math.round(raw));
}
// estimated current-form strike on the foe (mid variance, includes empower)
function strikeEstimate(b) {
  const cur = b.forms[b.current];
  let raw = cur.power * typeMult(cur.elem, b.foe.elem);
  if (b.empowered) raw *= EMPOWER_MULT;
  return Math.max(1, Math.round(raw));
}
function livingForms(b) {
  const out = [];
  for (let i = 0; i < b.forms.length; i++) if (!b.forms[i].exhausted) out.push(i);
  return out;
}
function formsAlive(b) { return livingForms(b).length; }

// the foe strikes the current form; resolves knock-outs and the loss state.
function foeTurn(b, res) {
  b.foe.turnsTaken++;
  const charged = b.foe.chargeEvery > 0 && (b.foe.turnsTaken % b.foe.chargeEvery === 0);
  const cur = b.forms[b.current];
  let dmg = rollDamage(b.foe.power, b.foe.elem, cur.elem, variance(b));
  if (charged) dmg = Math.round(dmg * b.foe.chargeMult);
  if (b.guarding) dmg = Math.max(1, Math.round(dmg * GUARD_MULT));
  cur.hp -= dmg;
  b.guarding = false;
  b.turn++;
  res.foeHit = { amount: dmg, eff: effLabel(b.foe.elem, cur.elem), charged: charged };
  if (cur.hp <= 0) {
    cur.hp = 0; cur.exhausted = true;
    res.knockedOut = true;
    if (formsAlive(b) === 0) { b.over = true; b.won = false; }
    else b.mustShift = true;
  }
}

function doShift(b, idx, free) {
  if (idx == null || idx < 0 || idx >= b.forms.length) return null;
  if (idx === b.current && !free) return null;
  if (b.forms[idx].exhausted) return null;
  b.current = idx;
  if (free) { b.mustShift = false; return { ok: true, type: 'shift', free: true, shiftedTo: idx }; }
  const res = { ok: true, type: 'shift', free: false, shiftedTo: idx };
  foeTurn(b, res);
  return res;
}

function finishOffense(b, res, dealt) {
  res.hit = { amount: dealt, eff: effLabel(b.forms[b.current].elem, b.foe.elem) };
  if (b.foe.hp <= 0) { b.foe.hp = 0; b.over = true; b.won = true; return res; }
  foeTurn(b, res);
  return res;
}

function doStrike(b) {
  const cur = b.forms[b.current];
  let dmg = rollDamage(cur.power, cur.elem, b.foe.elem, variance(b));
  dmg = Math.round(dmg * offenseMult(b));
  b.foe.hp -= dmg;
  return finishOffense(b, { ok: true, type: 'strike' }, dmg);
}

function doSpecial(b) {
  const cur = b.forms[b.current];
  if (cur.specialUsed) return null;
  cur.specialUsed = true;
  const res = { ok: true, type: 'special', special: cur.special };
  if (cur.special === 'pyre') {
    const base = rollDamage(cur.power, cur.elem, b.foe.elem, variance(b));
    const dmg = Math.round(base * 2.2 * offenseMult(b));
    b.foe.hp -= dmg;
    cur.hp = Math.max(1, cur.hp - 6);          // recoil never self-knocks-out
    res.recoil = 6;
    return finishOffense(b, res, dmg);
  }
  if (cur.special === 'tempo') {
    const d1 = rollDamage(cur.power, cur.elem, b.foe.elem, variance(b));
    const d2 = rollDamage(cur.power, cur.elem, b.foe.elem, variance(b));
    const dmg = Math.round((d1 + d2) * offenseMult(b));
    b.foe.hp -= dmg;
    return finishOffense(b, res, dmg);
  }
  if (cur.special === 'bulwark') {
    cur.hp = Math.min(cur.maxHP, cur.hp + 14);
    b.guarding = true;
    b.empowered = true;
    res.healed = 14;
    foeTurn(b, res);
    return res;
  }
  if (cur.special === 'mend') {
    for (const f of b.forms) if (!f.exhausted) f.hp = Math.min(f.maxHP, f.hp + 8);
    res.healed = 8;
    foeTurn(b, res);
    return res;
  }
  return null;
}

function doGuard(b) {
  b.guarding = true;
  b.empowered = true;
  const res = { ok: true, type: 'guard' };
  foeTurn(b, res);
  return res;
}

// action = { type:'strike'|'special'|'guard'|'shift', form? }
function act(b, action) {
  if (b.over || !action) return null;
  if (b.mustShift) {
    if (action.type !== 'shift') return null;
    return doShift(b, action.form, true);
  }
  if (action.type === 'shift')   return doShift(b, action.form, false);
  if (action.type === 'strike')  return doStrike(b);
  if (action.type === 'special') return doSpecial(b);
  if (action.type === 'guard')   return doGuard(b);
  return null;
}

function stars(b) {
  const alive = formsAlive(b);
  if (alive >= 3) return 3;
  if (alive === 2) return 2;
  return 1;
}
