// Pixel Card Spire - run, map, combat engine, UI
(() => {
'use strict';

const SAVE_KEY = 'pixel-spire-run';
const $ = id => document.getElementById(id);

// ---- screens -----------------------------------------------------------
const SCREENS = ['title','map','combat','reward','rest','shop','event','result'];
function showScreen(id) {
  SCREENS.forEach(s => $('screen-' + s).classList.toggle('hidden', s !== id));
}

// ---- run state ---------------------------------------------------------
let run = null;
let combat = null;
let rafId = 0, lastT = 0;

function loadRun() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)); } catch (e) { return null; }
}
function saveRun() { if (run) localStorage.setItem(SAVE_KEY, JSON.stringify(run)); }
function clearRun() { localStorage.removeItem(SAVE_KEY); }

function newRun() {
  run = {
    hp: 80, maxHp: 80, gold: 99,
    deck: STARTER_DECK.map(id => ({ id, upg: false })),
    relics: [], floor: -1, node: null,
    map: generateMap(), seenFloor: -1,
  };
  // give a random starting relic for flavour
  saveRun();
}

// ---- map generation ----------------------------------------------------
const MAP_FLOORS = 12;
function generateMap() {
  const floors = [];
  for (let f = 0; f < MAP_FLOORS; f++) {
    const nodes = [];
    let count;
    if (f === 0) count = 3;
    else if (f === MAP_FLOORS - 1) count = 1;
    else count = 2 + (Math.random() < 0.5 ? 1 : 0);
    // spread columns 0..3
    const cols = [0,1,2,3].sort(() => Math.random() - 0.5).slice(0, count).sort((a,b)=>a-b);
    for (const c of cols) {
      let type;
      if (f === 0) type = 'monster';
      else if (f === MAP_FLOORS - 1) type = 'boss';
      else if (f === MAP_FLOORS - 2) type = 'rest';
      else {
        const r = Math.random();
        if (r < 0.44) type = 'monster';
        else if (r < 0.64) type = 'event';
        else if (f >= 3 && r < 0.78) type = 'elite';
        else if (r < 0.88) type = 'shop';
        else type = 'rest';
      }
      nodes.push({ f, c, type, edges: [], done: false });
    }
    floors.push(nodes);
  }
  // connect each node to 1-2 nodes on next floor (nearest columns)
  for (let f = 0; f < MAP_FLOORS - 1; f++) {
    for (const n of floors[f]) {
      const next = floors[f + 1].slice().sort((a, b) => Math.abs(a.c - n.c) - Math.abs(b.c - n.c));
      const k = 1 + (Math.random() < 0.45 && next.length > 1 ? 1 : 0);
      for (let i = 0; i < k; i++) n.edges.push(floors[f + 1].indexOf(next[i]));
    }
    // ensure every next-floor node is reachable
    floors[f + 1].forEach((nn, idx) => {
      if (!floors[f].some(n => n.edges.includes(idx))) {
        const src = floors[f].slice().sort((a, b) => Math.abs(a.c - nn.c) - Math.abs(b.c - nn.c))[0];
        src.edges.push(idx);
      }
    });
  }
  return floors;
}

// ---- card resolution ---------------------------------------------------
function resolveCard(inst) {
  const def = CARD_DEFS[inst.id];
  const r = {
    id: inst.id, name: L(def.name), type: def.type, rarity: def.rarity,
    cost: def.cost, effects: def.effects, hits: def.hits || 1, aoe: !!def.aoe,
    power: def.power, powerV: def.powerV, special: def.special, upg: inst.upg,
  };
  if (inst.upg && def.u) {
    if (def.u.cost != null) r.cost = def.u.cost;
    if (def.u.effects) r.effects = def.u.effects;
    if (def.u.hits != null) r.hits = def.u.hits;
    if (def.u.powerV != null) r.powerV = def.u.powerV;
    if (def.u.special) r.special = def.u.special;
  }
  return r;
}
function cardNeedsTarget(eff) {
  return !eff.aoe && eff.type !== 'power' &&
    eff.effects.some(e => ['dmg','vuln','weak','poison'].includes(e.op));
}
function buildCardText(eff) {
  if (eff.special === 'rampage') return t('spRampage');
  if (eff.special === 'reaper') {
    return eff.effects.map(e => effectWord(e, eff)).join('. ') + '. ' + t('spReaper');
  }
  if (eff.special === 'limitBreak') return t('spLimit');
  if (eff.power) {
    const m = { demon: 'pwDemon', berserk: 'pwBerserk', juggernaut: 'pwJugg', metal: 'pwMetal' };
    return t(m[eff.power], eff.powerV);
  }
  return eff.effects.map(e => effectWord(e, eff)).join('. ') + '.';
}
function effectWord(e, eff) {
  switch (e.op) {
    case 'dmg': return t('dmgWord', e.v) + (eff.aoe ? t('toAll') : '') + (eff.hits > 1 ? t('timesWord', eff.hits) : '');
    case 'block': return t('blockWord', e.v);
    case 'draw': return t('drawWord', e.v);
    case 'energy': return t('energyWord', e.v);
    case 'str': return t('strWord', e.v);
    case 'vuln': return t('vulnWord', e.v) + (eff.aoe ? t('toAll') : '');
    case 'weak': return t('weakWord', e.v) + (eff.aoe ? t('toAll') : '');
    case 'poison': return t('poisonWord', e.v) + (eff.aoe ? t('toAll') : '');
    case 'heal': return t('healWord', e.v);
    case 'loseHp': return t('loseHpWord', e.v);
    default: return '';
  }
}

// ---- combat ------------------------------------------------------------
const scene = $('scene');
const sctx = scene.getContext('2d');
const SCENE_W = 720, SCENE_H = 440;
scene.width = SCENE_W; scene.height = SCENE_H;
sctx.imageSmoothingEnabled = false;

let pending = [];  // scheduled actions {at, fn}
let particles = [], floatTexts = [];

function schedule(delay, fn) { pending.push({ at: combat.clock + delay, fn }); }

function hasRelic(id) { return run.relics.includes(id); }

function startCombat(node) {
  const enemyIds = pickEnemies(node);
  const player = {
    side: 'player', hp: run.hp, maxHp: run.maxHp,
    block: 0, str: 0, vuln: 0, weak: 0, poison: 0,
    energy: 0, maxEnergy: 3, powers: {},
    hitFlash: 0, lunge: 0, ironCharmUsed: false,
  };
  const enemies = enemyIds.map((id, i) => makeEnemy(id, i, enemyIds.length));
  combat = {
    node, player, enemies, turn: 0, phase: 'busy', clock: 0,
    drawPile: [], hand: [], discardPile: [], exhaustPile: [],
    rampageBonus: 0, reaperHeal: 0, selectedCard: null, won: false, lost: false,
    isElite: node.type === 'elite', isBoss: node.type === 'boss',
  };
  // build draw pile (shuffled deck)
  combat.drawPile = shuffle(run.deck.map(c => ({ id: c.id, upg: c.upg })));
  particles = []; floatTexts = []; pending = [];
  layoutEnemies();
  combat.enemies.forEach(chooseIntent);

  // combat-start relics
  if (hasRelic('anchor')) addBlock(player, 8, true);
  if (hasRelic('warDrum')) player.str += 1;
  if (hasRelic('handCannon')) enemies.forEach(e => dealDamage(null, e, 8, false));
  if (hasRelic('crackedCore')) player.coreBonus = 1;

  showScreen('combat');
  resizeScene();
  schedule(0.35, startPlayerTurn);
  renderCombatHud();
}

function pickEnemies(node) {
  if (node.type === 'boss') return ['warden'];
  if (node.type === 'elite') return [Math.random() < 0.5 ? 'golem' : 'twins'];
  const pool = ['slime','bat','cultist','bandit','spiker','shield','witch','wolf'];
  const f = node.f;
  let count = f < 2 ? 1 + (Math.random() < 0.5 ? 1 : 0) : f < 7 ? 2 : 2 + (Math.random() < 0.5 ? 1 : 0);
  const ids = [];
  while (ids.length < count) {
    const id = pool[Math.floor(Math.random() * pool.length)];
    if (ids.filter(x => x === id).length < 2) ids.push(id);
  }
  return ids;
}

function makeEnemy(id, idx, total) {
  const def = ENEMY_DEFS[id];
  const hp = def.hp[0] + Math.floor(Math.random() * (def.hp[1] - def.hp[0] + 1));
  const e = {
    side: 'enemy', id, def, name: L(def.name),
    hp, maxHp: hp, block: 0, str: 0, vuln: 0, weak: 0, poison: 0,
    sprite: def.sprite, color: def.color,
    thorns: def.thorns || 0, patternIdx: 0, growBonus: 0,
    intent: null, hitFlash: 0, lunge: 0, dead: 0, alive: true,
  };
  // layout
  const big = def.boss ? 2.3 : def.elite ? 2.0 : 1.62;
  e.scale = big;
  e._idx = idx; e._total = total;
  return e;
}
function layoutEnemies() {
  if (!combat) return;
  const n = combat.enemies.length;
  const spread = n === 1 ? [470] : n === 2 ? [385, 565] : [320, 475, 615];
  combat.enemies.forEach((e, i) => {
    e.sx = spread[i];
    e.sy = 300 + (e.def.boss ? 6 : 0);
  });
}

function shuffle(arr) {
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

// --- turn flow ---
function startPlayerTurn() {
  if (combat.won || combat.lost) return;
  combat.turn++;
  const p = combat.player;
  p.block = 0;
  p.energy = p.maxEnergy + (hasRelic('energyCore') ? 1 : 0);
  if (combat.turn === 1 && p.coreBonus) p.energy += p.coreBonus;
  // powers
  if (p.powers.demon) p.str += p.powers.demon;
  if (p.powers.berserk) p.energy += p.powers.berserk;
  // poison tick
  tickPoison(p);
  if (combat.lost) return;
  // draw
  let drawN = 5;
  if (combat.turn === 1 && hasRelic('oldTome')) drawN++;
  drawCards(drawN);
  combat.phase = 'player';
  renderHand();
  renderCombatHud();
}

function endTurn() {
  if (combat.phase !== 'player' || pending.length) return;
  const p = combat.player;
  if (p.powers.metal) addBlock(p, p.powers.metal, true);
  // vuln/weak tick down at end of own turn
  if (p.vuln > 0) p.vuln--;
  if (p.weak > 0) p.weak--;
  // discard hand
  combat.discardPile.push(...combat.hand);
  combat.hand = [];
  combat.selectedCard = null;
  combat.phase = 'enemy';
  renderHand();
  renderCombatHud();
  schedule(0.4, enemyTurn);
}

function enemyTurn() {
  let delay = 0;
  for (const e of combat.enemies) {
    if (!e.alive) continue;
    schedule(delay, () => doEnemyAction(e));
    delay += 0.75;
  }
  schedule(delay + 0.3, () => {
    if (!combat.won && !combat.lost) startPlayerTurn();
  });
}

function doEnemyAction(e) {
  if (!e.alive || combat.lost) return;
  e.block = 0;
  tickPoison(e);
  if (!e.alive) { afterEnemyAct(e); return; }
  const m = e.intent;
  if (m.kind === 'attack') {
    e.lunge = 1;
    const hits = m.hits || 1;
    for (let h = 0; h < hits; h++) {
      schedule(0.12 + h * 0.16, () => {
        if (!e.alive) return;
        let dmg = m.dmg + (m.grow ? e.growBonus : 0);
        dealDamage(e, combat.player, dmg, true);
        combat.player.hitFlash = 1;
        if (combat.lost) return;
      });
    }
    if (m.grow) e.growBonus += 3;
    if (m.selfStr) schedule(0.1, () => { e.str += m.selfStr; spawnText(e.sx, e.sy - 70, '+' + m.selfStr + ' ' + t('str'), '#ffd34d'); });
    if (m.debuff) schedule(0.2, () => applyDebuffToPlayer(m.debuff, m.debV));
  } else if (m.kind === 'block') {
    addBlock(e, m.block, true);
    spawnText(e.sx, e.sy - 70, '+' + m.block + ' ' + t('block'), '#7fb8ff');
  } else if (m.kind === 'buff') {
    e.str += m.str;
    spawnText(e.sx, e.sy - 70, '+' + m.str + ' ' + t('str'), '#ffd34d');
  } else if (m.kind === 'debuff') {
    applyDebuffToPlayer(m.debuff, m.debV);
  }
  schedule(0.55, () => afterEnemyAct(e));
}
function afterEnemyAct(e) {
  if (e.alive) chooseIntent(e);
  renderCombatHud();
}

function applyDebuffToPlayer(kind, v) {
  const p = combat.player;
  if (kind === 'weak' && hasRelic('gingerRoot')) {
    spawnText(150, 220, 'IMMUNE', '#7fe8ff'); return;
  }
  if (kind === 'vuln') p.vuln += v;
  if (kind === 'weak') p.weak += v;
  spawnText(150, 230, '+' + v + ' ' + (kind === 'vuln' ? t('vuln') : t('weak')), '#c69bff');
  p.hitFlash = 0.6;
}

function chooseIntent(e) {
  const pat = e.def.pattern;
  const noLoop = e.id === 'cultist';
  let idx = noLoop ? Math.min(e.patternIdx, pat.length - 1) : e.patternIdx % pat.length;
  e.intent = parseMove(pat[idx]);
  e.patternIdx++;
}
function parseMove(str) {
  const p = str.split(':');
  switch (p[0]) {
    case 'attack': return { kind: 'attack', dmg: +p[1], hits: 1 };
    case 'multi': return { kind: 'attack', dmg: +p[1], hits: +p[2] };
    case 'grow': return { kind: 'attack', dmg: +p[1], hits: 1, grow: true };
    case 'block': return { kind: 'block', block: +p[1] };
    case 'buff': return { kind: 'buff', str: +p[1] };
    case 'attbuff': return { kind: 'attack', dmg: +p[1], hits: 1, selfStr: +p[2] };
    case 'atkdebuff': return { kind: 'attack', dmg: +p[1], hits: 1, debuff: p[2], debV: +p[3] };
    case 'debuff': return { kind: 'debuff', debuff: p[1], debV: +p[2] };
    default: return { kind: 'block', block: 0 };
  }
}

// --- damage / block / status ---
function dealDamage(source, target, raw, isAttack) {
  let amount = raw;
  if (isAttack && source) {
    amount += source.str || 0;
    if (source.weak > 0) amount = Math.floor(amount * 0.75);
  }
  if (target.vuln > 0) amount = Math.floor(amount * 1.5);
  amount = Math.max(0, amount);
  let unblocked = amount;
  if (target.block > 0) {
    const b = Math.min(target.block, unblocked);
    target.block -= b; unblocked -= b;
  }
  target.hp -= unblocked;
  target.hitFlash = 1;
  if (unblocked > 0) {
    spawnText(target.side === 'player' ? 150 : target.sx,
              (target.side === 'player' ? 235 : target.sy - 80) - 8,
              '-' + unblocked, '#ff6a5a');
    burst(target.side === 'player' ? 150 : target.sx,
          target.side === 'player' ? 240 : target.sy - 50, 8, '#ff6a5a');
  } else if (amount === 0) {
    // fully blocked or zero
  }
  if (target.side === 'player') checkPlayerState();
  else checkEnemyDeath(target);
  return unblocked;
}
function addBlock(c, amount, silent) {
  if (amount <= 0) return;
  c.block += amount;
  if (!silent && c.side === 'player') spawnText(150, 230, '+' + amount + ' ' + t('block'), '#7fb8ff');
  // juggernaut
  if (c.side === 'player' && c.powers.juggernaut) {
    const alive = combat.enemies.filter(e => e.alive);
    if (alive.length) {
      const tgt = alive[Math.floor(Math.random() * alive.length)];
      schedule(0.1, () => { if (tgt.alive) dealDamage(null, tgt, c.powers.juggernaut, false); });
    }
  }
}
function tickPoison(c) {
  if (c.poison > 0) {
    c.hp -= c.poison;
    spawnText(c.side === 'player' ? 150 : c.sx,
              (c.side === 'player' ? 235 : c.sy - 80), '-' + c.poison, '#9fe07a');
    c.poison--;
    if (c.side === 'player') checkPlayerState();
    else checkEnemyDeath(c);
  }
}
function checkEnemyDeath(e) {
  if (e.alive && e.hp <= 0) {
    e.hp = 0; e.alive = false; e.dead = 1;
    burst(e.sx, e.sy - 40, 22, e.color);
    if (hasRelic('vampFang')) healPlayer(3);
    schedule(0.5, () => {
      if (combat.enemies.every(x => !x.alive) && !combat.won && !combat.lost) winCombat();
    });
  }
}
function checkPlayerState() {
  const p = combat.player;
  if (p.hp <= 0 && !combat.lost) {
    p.hp = 0; combat.lost = true; combat.phase = 'busy';
    schedule(0.8, loseRun);
  }
  // iron charm
  if (!p.ironCharmUsed && hasRelic('ironCharm') && p.hp <= p.maxHp / 2 && p.hp > 0) {
    p.ironCharmUsed = true; p.str += 3;
    spawnText(150, 210, '+3 ' + t('str'), '#ffd34d');
  }
}
function healPlayer(n) {
  const p = combat.player;
  p.hp = Math.min(p.maxHp, p.hp + n);
  spawnText(150, 230, '+' + n, '#5fe07a');
}

// --- cards in combat ---
function drawCards(n) {
  for (let i = 0; i < n; i++) {
    if (combat.drawPile.length === 0) {
      if (combat.discardPile.length === 0) break;
      combat.drawPile = shuffle(combat.discardPile);
      combat.discardPile = [];
    }
    if (combat.hand.length >= 10) break;
    combat.hand.push(combat.drawPile.pop());
  }
}

function tryPlayCard(handIdx, targetEnemy) {
  if (combat.phase !== 'player' || pending.length) return;
  const inst = combat.hand[handIdx];
  if (!inst) return;
  const eff = resolveCard(inst);
  if (combat.player.energy < eff.cost) { flash(t('notEnough')); return; }
  if (cardNeedsTarget(eff) && !targetEnemy) {
    // arm card, wait for target
    combat.selectedCard = combat.selectedCard === handIdx ? null : handIdx;
    renderHand();
    return;
  }
  // resolve
  combat.player.energy -= eff.cost;
  combat.hand.splice(handIdx, 1);
  combat.selectedCard = null;
  combat.player.lunge = eff.type === 'attack' ? 1 : 0;
  applyCardEffects(eff, targetEnemy);
  // move card to discard / exhaust
  if (eff.type === 'power' || eff.special === 'limitBreak' || inst.id === 'offering' || inst.id === 'bloodlet') {
    combat.exhaustPile.push(inst);
  } else {
    combat.discardPile.push(inst);
  }
  renderHand();
  renderCombatHud();
}

function applyCardEffects(eff, targetEnemy) {
  const p = combat.player;
  const targets = eff.aoe ? combat.enemies.filter(e => e.alive)
                          : (targetEnemy ? [targetEnemy] : []);
  for (const e of eff.effects) {
    switch (e.op) {
      case 'dmg': {
        let val = e.v;
        if (eff.special === 'rampage') val += combat.rampageBonus;
        if (hasRelic('whetstone') && eff.id === 'strike') val += 2;
        for (let h = 0; h < eff.hits; h++) {
          for (const tg of targets) {
            if (!tg.alive) continue;
            schedule(h * 0.12, () => {
              if (!tg.alive) return;
              const dealt = dealDamage(p, tg, val, true);
              if (eff.special === 'reaper') healPlayer(dealt);
              if (tg.thorns) dealDamage(null, p, tg.thorns, false);
            });
          }
        }
        break;
      }
      case 'block': addBlock(p, e.v); break;
      case 'draw': drawCards(e.v); break;
      case 'energy': p.energy += e.v; break;
      case 'str': p.str += e.v; spawnText(150, 200, '+' + e.v + ' ' + t('str'), '#ffd34d'); break;
      case 'vuln': for (const tg of targets) if (tg.alive) { tg.vuln += e.v; spawnText(tg.sx, tg.sy - 95, '+' + e.v + ' ' + t('vuln'), '#c69bff'); } break;
      case 'weak': for (const tg of targets) if (tg.alive) { tg.weak += e.v; spawnText(tg.sx, tg.sy - 95, '+' + e.v + ' ' + t('weak'), '#c69bff'); } break;
      case 'poison': for (const tg of targets) if (tg.alive) { tg.poison += e.v; spawnText(tg.sx, tg.sy - 95, '+' + e.v + ' ' + t('poison'), '#9fe07a'); } break;
      case 'heal': healPlayer(e.v); break;
      case 'loseHp': p.hp = Math.max(1, p.hp - e.v); spawnText(150, 235, '-' + e.v, '#ff9c5f'); break;
    }
  }
  if (eff.special === 'rampage') combat.rampageBonus += 4;
  if (eff.special === 'limitBreak') { p.str = p.str * 2; spawnText(150, 200, 'STR x2', '#ffd34d'); }
  if (eff.power) {
    p.powers[eff.power] = (p.powers[eff.power] || 0) + eff.powerV;
    spawnText(150, 195, '★ ' + eff.name, '#ffd34d');
  }
  renderHand();
}

// --- combat end ---
function winCombat() {
  combat.won = true;
  combat.phase = 'busy';
  run.hp = combat.player.hp;
  if (hasRelic('medkit')) run.hp = Math.min(run.maxHp, run.hp + 7);
  schedule(0.7, () => openReward());
}
function loseRun() {
  showResult(false);
  clearRun();
}

// ---- rewards -----------------------------------------------------------
function openReward() {
  combat.node.done = true;
  let gold = combat.isBoss ? 100 : combat.isElite ? 45 + Math.floor(Math.random()*20) : 18 + Math.floor(Math.random()*16);
  if (hasRelic('luckyCoin')) gold += 25;
  run.gold += gold;
  run.hp = combat.player.hp;
  const cardN = hasRelic('cloverLeaf') ? 4 : 3;
  const cards = rollCardReward(Math.random, cardN);
  const relicReward = combat.isElite || combat.isBoss ? rollRelic() : null;

  const box = $('reward-content');
  box.innerHTML = '';
  const gline = document.createElement('div');
  gline.className = 'reward-gold';
  gline.textContent = t('goldReward', gold);
  box.appendChild(gline);

  if (relicReward) {
    const rl = document.createElement('button');
    rl.className = 'reward-relic-btn';
    rl.innerHTML = `<b>${t('relicReward')}</b><span>${L(RELIC_DEFS[relicReward].name)}</span>` +
      `<small>${L(RELIC_DEFS[relicReward].desc)}</small>`;
    rl.onclick = () => { if (rl.disabled) return; grantRelic(relicReward); rl.disabled = true; rl.classList.add('taken'); };
    box.appendChild(rl);
  }

  const label = document.createElement('div');
  label.className = 'reward-label';
  label.textContent = t('cardReward');
  box.appendChild(label);

  const row = document.createElement('div');
  row.className = 'reward-cards';
  let picked = false;
  cards.forEach(id => {
    const el = makeCardEl({ id, upg: false }, false);
    el.onclick = () => {
      if (picked) return;
      picked = true;
      run.deck.push({ id, upg: false });
      row.querySelectorAll('.card').forEach(c => c.classList.add('dim'));
      el.classList.remove('dim'); el.classList.add('chosen');
      $('btn-reward-proceed').textContent = t('proceed');
    };
    row.appendChild(el);
  });
  box.appendChild(row);
  $('btn-reward-proceed').textContent = t('skip');
  showScreen('reward');
}
function rollRelic() {
  const owned = run.relics;
  const avail = RELIC_POOL.filter(r => !owned.includes(r));
  if (combat && combat.isBoss) {
    if (!owned.includes('energyCore')) return 'energyCore';
  }
  return avail.length ? avail[Math.floor(Math.random() * avail.length)] : null;
}
function grantRelic(id) {
  if (run.relics.includes(id)) return;
  run.relics.push(id);
  if (id === 'thickHide') { run.maxHp += 12; run.hp += 12; }
  flash('★ ' + L(RELIC_DEFS[id].name));
}

// ---- map screen --------------------------------------------------------
function renderMap() {
  saveRun();
  const area = $('map-area');
  area.innerHTML = '';
  const W = 100, H = 100; // percent space
  const floors = run.map;
  // svg edges
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('class', 'map-svg');
  svg.setAttribute('viewBox', '0 0 100 100');
  svg.setAttribute('preserveAspectRatio', 'none');
  const nodePos = (n) => {
    const fl = floors[n.f];
    const colSpan = 80, x0 = 10;
    const x = x0 + (fl.length === 1 ? colSpan / 2 : (n.c) / 3 * colSpan);
    const y = 94 - n.f / (MAP_FLOORS - 1) * 88;
    return { x, y };
  };
  for (let f = 0; f < floors.length - 1; f++) {
    for (const n of floors[f]) {
      const a = nodePos(n);
      for (const ei of n.edges) {
        const b = nodePos(floors[f + 1][ei]);
        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        line.setAttribute('x1', a.x); line.setAttribute('y1', a.y);
        line.setAttribute('x2', b.x); line.setAttribute('y2', b.y);
        const active = n.done && isReachable(floors[f + 1][ei]);
        line.setAttribute('class', 'map-edge' + (active ? ' active' : ''));
        svg.appendChild(line);
      }
    }
  }
  area.appendChild(svg);
  // nodes
  floors.forEach((fl, f) => {
    fl.forEach((n, idx) => {
      const p = nodePos(n);
      const btn = document.createElement('button');
      const reach = isReachable(n);
      btn.className = 'map-node node-' + n.type +
        (n.done ? ' done' : '') + (reach ? ' reachable' : '') +
        (run.node === n ? ' current' : '');
      btn.style.left = p.x + '%';
      btn.style.top = p.y + '%';
      btn.innerHTML = `<span class="node-ico">${NODE_ICON[n.type]}</span>`;
      if (reach) btn.onclick = () => enterNode(n);
      area.appendChild(btn);
    });
  });
  // hud
  $('map-hp').textContent = '♥ ' + run.hp + '/' + run.maxHp;
  $('map-gold').textContent = '◆ ' + run.gold;
  $('map-floor').textContent = t('floor') + ' ' + (run.floor + 2 > MAP_FLOORS ? MAP_FLOORS : run.floor + 2);
  renderRelicStrip($('map-relics'));
  showScreen('map');
}
const NODE_ICON = { monster: '⚔', elite: '☠', boss: '♛', shop: '$', rest: '♨', event: '?' };

function isReachable(n) {
  if (n.done) return false;
  if (run.floor < 0) return n.f === 0;
  if (!run.node) return n.f === 0;
  if (n.f !== run.floor + 1) return false;
  const idx = run.map[n.f].indexOf(n);
  return run.node.edges.includes(idx);
}

function enterNode(n) {
  run.floor = n.f;
  run.node = n;
  saveRun();
  if (n.type === 'monster' || n.type === 'elite' || n.type === 'boss') startCombat(n);
  else if (n.type === 'rest') openRest(n);
  else if (n.type === 'shop') openShop(n);
  else if (n.type === 'event') openEvent(n);
}

// ---- rest --------------------------------------------------------------
function openRest(node) {
  const box = $('rest-content');
  box.innerHTML = '';
  const heal = document.createElement('button');
  heal.className = 'rest-btn';
  heal.textContent = t('restHeal');
  heal.onclick = () => {
    run.hp = Math.min(run.maxHp, run.hp + Math.ceil(run.maxHp * 0.3));
    node.done = true; renderMap();
  };
  const forge = document.createElement('button');
  forge.className = 'rest-btn';
  forge.textContent = t('restUpgrade');
  forge.onclick = () => {
    openCardPicker(t('cardUpgradeHint'),
      c => !c.upg && !!CARD_DEFS[c.id].u,
      c => { c.upg = true; node.done = true; closePicker(); renderMap(); });
  };
  box.appendChild(heal); box.appendChild(forge);
  showScreen('rest');
}

// ---- shop --------------------------------------------------------------
function openShop(node) {
  const box = $('shop-content');
  box.innerHTML = '';
  const stock = rollCardReward(Math.random, 5).map(id => ({
    id, upg: false, price: cardPrice(id),
  }));
  const grid = document.createElement('div');
  grid.className = 'shop-grid';
  stock.forEach(item => {
    const wrap = document.createElement('div');
    wrap.className = 'shop-item';
    const el = makeCardEl({ id: item.id, upg: false }, false);
    wrap.appendChild(el);
    const buy = document.createElement('button');
    buy.className = 'shop-buy';
    buy.textContent = '◆ ' + item.price;
    buy.onclick = () => {
      if (item.sold) return;
      if (run.gold < item.price) { flash(t('notEnough')); return; }
      run.gold -= item.price;
      run.deck.push({ id: item.id, upg: false });
      item.sold = true;
      buy.textContent = t('sold'); buy.classList.add('sold');
      el.classList.add('dim');
      updateShopHud();
    };
    wrap.appendChild(buy);
    grid.appendChild(wrap);
  });
  box.appendChild(grid);
  const removeCost = 65;
  const rm = document.createElement('button');
  rm.className = 'shop-remove';
  rm.textContent = t('removeCard', removeCost);
  let removed = false;
  rm.onclick = () => {
    if (removed) return;
    if (run.gold < removeCost) { flash(t('notEnough')); return; }
    openCardPicker(t('cardRemoveHint'), () => true, c => {
      run.gold -= removeCost;
      run.deck.splice(run.deck.indexOf(c), 1);
      removed = true; rm.disabled = true; rm.classList.add('sold');
      closePicker(); updateShopHud();
    });
  };
  box.appendChild(rm);
  node._shopDone = node;
  showScreen('shop');
  updateShopHud();
}
function updateShopHud() { $('shop-gold').textContent = '◆ ' + run.gold; }
function cardPrice(id) {
  const r = CARD_DEFS[id].rarity;
  return r === 'rare' ? 110 + Math.floor(Math.random()*40)
       : r === 'uncommon' ? 60 + Math.floor(Math.random()*30)
       : 35 + Math.floor(Math.random()*20);
}

// ---- events ------------------------------------------------------------
function openEvent(node) {
  const ev = EVENT_DEFS[Math.floor(Math.random() * EVENT_DEFS.length)];
  const box = $('event-content');
  box.innerHTML = `<h2>${L(ev.title)}</h2><p class="event-text">${L(ev.text)}</p>`;
  const list = document.createElement('div');
  list.className = 'event-choices';
  ev.choices.forEach(ch => {
    const b = document.createElement('button');
    b.className = 'event-choice';
    b.textContent = L(ch.label);
    b.onclick = () => resolveEvent(ch.act, node);
    list.appendChild(b);
  });
  box.appendChild(list);
  showScreen('event');
}
function resolveEvent(act, node) {
  const finish = () => { node.done = true; renderMap(); };
  switch (act) {
    case 'shrine_relic': {
      run.hp = Math.max(1, run.hp - 8);
      const r = rollRelic();
      if (r) grantRelic(r);
      finish(); break;
    }
    case 'shrine_heal': run.hp = Math.min(run.maxHp, run.hp + 14); finish(); break;
    case 'shrine_heal2': run.hp = Math.min(run.maxHp, run.hp + 10); finish(); break;
    case 'merchant_upgrade':
      if (run.gold < 45) { flash(t('notEnough')); return; }
      openCardPicker(t('cardUpgradeHint'), c => !c.upg && !!CARD_DEFS[c.id].u, c => {
        run.gold -= 45; c.upg = true; closePicker(); finish();
      });
      break;
    case 'merchant_remove':
      if (run.gold < 30) { flash(t('notEnough')); return; }
      openCardPicker(t('cardRemoveHint'), () => true, c => {
        run.gold -= 30; run.deck.splice(run.deck.indexOf(c), 1); closePicker(); finish();
      });
      break;
    case 'training_card': {
      run.hp = Math.max(1, run.hp - 6);
      const id = rollCardReward(Math.random, 1)[0];
      run.deck.push({ id, upg: false });
      flash('+ ' + L(CARD_DEFS[id].name));
      finish(); break;
    }
    default: finish();
  }
}

// ---- card picker overlay ----------------------------------------------
function openCardPicker(hint, filter, onPick) {
  const ov = $('overlay-picker');
  $('picker-hint').textContent = hint;
  const grid = $('picker-grid');
  grid.innerHTML = '';
  run.deck.forEach(c => {
    const el = makeCardEl(c, false);
    if (filter(c)) {
      el.onclick = () => onPick(c);
    } else {
      el.classList.add('dim');
    }
    grid.appendChild(el);
  });
  ov.classList.remove('hidden');
}
function closePicker() { $('overlay-picker').classList.add('hidden'); }

// ---- deck / relic viewers ---------------------------------------------
function openDeckView() {
  const ov = $('overlay-picker');
  $('picker-hint').textContent = t('yourDeck') + ' (' + run.deck.length + ')';
  const grid = $('picker-grid');
  grid.innerHTML = '';
  run.deck.slice().sort((a,b)=> resolveCard(a).cost - resolveCard(b).cost)
    .forEach(c => grid.appendChild(makeCardEl(c, false)));
  ov.classList.remove('hidden');
}
function renderRelicStrip(el) {
  el.innerHTML = '';
  run.relics.forEach(id => {
    const r = document.createElement('span');
    r.className = 'relic-chip';
    r.textContent = relicIcon(id);
    r.title = L(RELIC_DEFS[id].name);
    r.onclick = () => flash(L(RELIC_DEFS[id].name) + ' — ' + L(RELIC_DEFS[id].desc));
    el.appendChild(r);
  });
}
function relicIcon(id) {
  const m = { anchor:'⚓', warDrum:'🥁', whetstone:'🔪', medkit:'✚', thickHide:'🛡',
    oldTome:'📖', crackedCore:'◈', gingerRoot:'🌱', vampFang:'🦇', handCannon:'💥',
    luckyCoin:'🪙', energyCore:'⚡', ironCharm:'🔱', cloverLeaf:'🍀' };
  return m[id] || '★';
}

// ---- card element ------------------------------------------------------
function makeCardEl(inst, playable) {
  const eff = resolveCard(inst);
  const el = document.createElement('div');
  el.className = 'card rarity-' + eff.rarity + ' type-' + eff.type + (inst.upg ? ' upg' : '');
  el.innerHTML =
    `<div class="card-cost">${eff.cost}</div>` +
    `<div class="card-name">${eff.name}${inst.upg ? '+' : ''}</div>` +
    `<div class="card-art">${CARD_GLYPH[eff.type]}</div>` +
    `<div class="card-text">${buildCardText(eff)}</div>` +
    `<div class="card-type">${typeLabel(eff.type)}</div>`;
  return el;
}
const CARD_GLYPH = { attack: '⚔', skill: '◈', power: '✦' };
function typeLabel(tp) {
  return currentLang === 'zh'
    ? ({ attack: '攻击', skill: '技能', power: '能力' }[tp])
    : tp.charAt(0).toUpperCase() + tp.slice(1);
}

// ---- combat UI ---------------------------------------------------------
function renderHand() {
  const area = $('cb-hand');
  area.innerHTML = '';
  combat.hand.forEach((inst, i) => {
    const el = makeCardEl(inst, true);
    const eff = resolveCard(inst);
    if (combat.player.energy < eff.cost) el.classList.add('unaffordable');
    if (combat.selectedCard === i) el.classList.add('selected');
    el.onclick = () => {
      if (combat.phase !== 'player' || pending.length) return;
      tryPlayCard(i, null);
    };
    area.appendChild(el);
  });
  $('cb-draw-count').textContent = combat.drawPile.length;
  $('cb-discard-count').textContent = combat.discardPile.length;
  const armed = combat.selectedCard != null;
  $('cb-stage').classList.toggle('targeting', armed);
}
function renderCombatHud() {
  if (!combat) return;
  const p = combat.player;
  $('cb-hp').textContent = '♥ ' + Math.max(0, p.hp) + '/' + p.maxHp;
  $('cb-energy').textContent = p.energy + '/' + (p.maxEnergy + (hasRelic('energyCore') ? 1 : 0));
  $('cb-gold').textContent = '◆ ' + run.gold;
  $('cb-turn').textContent = t('turn') + ' ' + combat.turn;
  const eb = $('btn-end-turn');
  eb.disabled = combat.phase !== 'player' || pending.length > 0;
}

// ---- scene rendering ---------------------------------------------------
function resizeScene() {
  const stage = $('cb-stage');
  const sw = stage.clientWidth, sh = stage.clientHeight;
  const scl = Math.min(sw / SCENE_W, sh / SCENE_H);
  scene.style.width = Math.floor(SCENE_W * scl) + 'px';
  scene.style.height = Math.floor(SCENE_H * scl) + 'px';
  sceneRect = scene.getBoundingClientRect();
}
let sceneRect = null;

function renderScene(dt) {
  // background
  const g = sctx.createLinearGradient(0, 0, 0, SCENE_H);
  g.addColorStop(0, '#2a2440');
  g.addColorStop(1, '#15121f');
  sctx.fillStyle = g;
  sctx.fillRect(0, 0, SCENE_W, SCENE_H);
  // floor
  sctx.fillStyle = '#1f1b30';
  sctx.fillRect(0, 320, SCENE_W, SCENE_H - 320);
  sctx.fillStyle = '#262138';
  for (let x = 0; x < SCENE_W; x += 40) sctx.fillRect(x, 320, 38, 6);
  // distant pillars
  sctx.fillStyle = 'rgba(255,255,255,0.03)';
  for (let i = 0; i < 5; i++) sctx.fillRect(60 + i * 150, 80, 50, 240);

  const p = combat.player;
  // decay anims
  p.hitFlash = Math.max(0, p.hitFlash - dt * 3);
  p.lunge = Math.max(0, p.lunge - dt * 3);
  const plx = 150 + p.lunge * 40;
  drawPlayer(sctx, plx, 360, 1.65, combat.clock, p.hitFlash);
  drawHpBar(sctx, 150, 270, 96, p.hp / p.maxHp, p.block);
  drawStatusPips(150, 286, p);

  // enemies
  combat.enemies.forEach(e => {
    e.hitFlash = Math.max(0, e.hitFlash - dt * 3);
    e.lunge = Math.max(0, e.lunge - dt * 3);
    if (!e.alive) { e.dead = Math.max(0, e.dead - dt * 1.6); }
    const ex = e.sx - e.lunge * 46;
    drawCreature(sctx, e.sprite, ex, e.sy, e.scale, e.color, combat.clock + e._idx,
      e.hitFlash, e.alive ? 0 : e.dead);
    if (e.alive || e.dead > 0) {
      const barY = e.sy - 78 - (e.def.boss ? 24 : e.def.elite ? 12 : 0);
      if (e.alive) {
        drawHpBar(sctx, e.sx, barY, e.def.boss ? 150 : 84, e.hp / e.maxHp, e.block);
        drawStatusPips(e.sx, barY + 16, e);
        if (e.intent && combat.phase !== 'busy') {
          drawIntentBadge(sctx, e.sx, barY - 26, e.intent.kind, intentValue(e), combat.clock);
        }
      }
    }
    // target highlight
    if (combat.selectedCard != null && e.alive) {
      sctx.strokeStyle = 'rgba(255,211,77,0.9)';
      sctx.lineWidth = 3;
      const pulse = 4 + Math.sin(combat.clock * 6) * 3;
      sctx.strokeRect(e.sx - 40 - pulse, e.sy - 92 - pulse, 80 + pulse * 2, 100 + pulse * 2);
    }
  });

  // particles
  for (const pt of particles) {
    sctx.globalAlpha = Math.max(0, pt.life / pt.max);
    sctx.fillStyle = pt.color;
    sctx.fillRect(pt.x - pt.s / 2, pt.y - pt.s / 2, pt.s, pt.s);
  }
  sctx.globalAlpha = 1;
  // floating text
  sctx.textAlign = 'center';
  sctx.font = 'bold 18px monospace';
  for (const ft of floatTexts) {
    sctx.globalAlpha = Math.min(1, ft.life * 1.4);
    sctx.fillStyle = '#000';
    sctx.fillText(ft.str, ft.x + 1, ft.y + 1);
    sctx.fillStyle = ft.color;
    sctx.fillText(ft.str, ft.x, ft.y);
  }
  sctx.globalAlpha = 1;

  if (flashTimer > 0) {
    sctx.globalAlpha = Math.min(1, flashTimer * 1.5);
    sctx.fillStyle = 'rgba(20,16,30,0.9)';
    sctx.fillRect(SCENE_W / 2 - 200, 14, 400, 32);
    sctx.fillStyle = '#ffd34d';
    sctx.font = 'bold 14px monospace';
    sctx.fillText(flashMsg, SCENE_W / 2, 35);
    sctx.globalAlpha = 1;
  }
}
function intentValue(e) {
  if (e.intent.kind !== 'attack') return null;
  let dmg = e.intent.dmg + (e.intent.grow ? e.growBonus : 0) + e.str;
  if (e.weak > 0) dmg = Math.floor(dmg * 0.75);
  if (combat.player.vuln > 0) dmg = Math.floor(dmg * 1.5);
  const hits = e.intent.hits || 1;
  return hits > 1 ? dmg + '×' + hits : dmg;
}
function drawStatusPips(x, y, c) {
  const pips = [];
  if (c.str > 0) pips.push(['S' + c.str, '#ffd34d']);
  if (c.vuln > 0) pips.push(['V' + c.vuln, '#ff6a5a']);
  if (c.weak > 0) pips.push(['W' + c.weak, '#c69bff']);
  if (c.poison > 0) pips.push(['P' + c.poison, '#9fe07a']);
  let px = x - (pips.length - 1) * 17;
  sctx.font = 'bold 11px monospace';
  sctx.textAlign = 'center';
  for (const [txt, col] of pips) {
    sctx.fillStyle = 'rgba(20,16,30,0.9)';
    sctx.fillRect(px - 15, y - 9, 30, 16);
    sctx.fillStyle = col;
    sctx.fillText(txt, px, y + 3);
    px += 34;
  }
}

// ---- effects -----------------------------------------------------------
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2, sp = 60 + Math.random() * 160;
    particles.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 40,
      life: 0.5, max: 0.5, s: 2 + Math.random() * 4, color });
  }
}
function spawnText(x, y, str, color) {
  floatTexts.push({ x, y, str, color, life: 1 });
}
let flashMsg = '', flashTimer = 0;
function flash(msg) { flashMsg = msg; flashTimer = 2.2; }

// ---- main loop ---------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.1) dt = 0.1;
  if (flashTimer > 0) flashTimer -= dt;
  if (combat && !$('screen-combat').classList.contains('hidden')) {
    combat.clock += dt;
    // run scheduled actions
    let ran = true;
    while (ran) {
      ran = false;
      for (let i = 0; i < pending.length; i++) {
        if (pending[i].at <= combat.clock) {
          const a = pending.splice(i, 1)[0];
          a.fn();
          ran = true;
          break;
        }
      }
    }
    for (const pt of particles) { pt.life -= dt; pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.vy += 260 * dt; }
    particles = particles.filter(p => p.life > 0);
    for (const ft of floatTexts) { ft.life -= dt; ft.y -= 30 * dt; }
    floatTexts = floatTexts.filter(f => f.life > 0);
    renderCombatHud();
    renderScene(dt);
  }
}

// ---- scene input -------------------------------------------------------
scene.addEventListener('pointerdown', e => {
  if (!combat || combat.phase !== 'player' || pending.length) return;
  if (combat.selectedCard == null) return;
  if (!sceneRect) sceneRect = scene.getBoundingClientRect();
  const x = (e.clientX - sceneRect.left) / sceneRect.width * SCENE_W;
  const y = (e.clientY - sceneRect.top) / sceneRect.height * SCENE_H;
  // hit-test enemies
  let hit = null;
  for (const en of combat.enemies) {
    if (!en.alive) continue;
    if (Math.abs(x - en.sx) < 52 && Math.abs(y - (en.sy - 40)) < 70) hit = en;
  }
  if (hit) tryPlayCard(combat.selectedCard, hit);
  else { combat.selectedCard = null; renderHand(); }
});

// ---- result ------------------------------------------------------------
function showResult(won) {
  $('result-title').textContent = won ? t('victory') : t('defeat');
  $('result-title').className = won ? 'win' : 'lose';
  $('result-sub').textContent = won ? t('victorySub') : t('defeatSub', run.floor + 1);
  showScreen('result');
}

// ---- proceed from non-combat screens ----------------------------------
function proceedFromReward() {
  if (combat && combat.isBoss && combat.won) {
    showResult(true);
    clearRun();
    return;
  }
  combat = null;
  renderMap();
}

// ---- wire UI -----------------------------------------------------------
function bindUI() {
  $('btn-new').onclick = () => { newRun(); renderMap(); };
  $('btn-continue').onclick = () => { run = loadRun(); if (run) renderMap(); };
  $('btn-reward-proceed').onclick = proceedFromReward;
  $('btn-shop-leave').onclick = () => { run.node.done = true; combat = null; renderMap(); };
  $('btn-picker-close').onclick = closePicker;
  $('btn-deck-view').onclick = openDeckView;
  $('btn-end-turn').onclick = endTurn;
  $('btn-cb-menu').onclick = () => {
    if (confirm(t('confirmAbandon'))) { clearRun(); run = null; combat = null; bootTitle(); }
  };
  $('btn-result-menu').onclick = bootTitle;
  $('btn-result-new').onclick = () => { newRun(); renderMap(); };
  setupLanguageToggle(() => {
    if (!$('screen-combat').classList.contains('hidden')) { renderHand(); renderCombatHud(); }
    if (!$('screen-map').classList.contains('hidden')) renderMap();
  });
}

function bootTitle() {
  run = loadRun();
  $('btn-continue').classList.toggle('hidden', !run);
  showScreen('title');
}

window.addEventListener('resize', () => { if (combat) resizeScene(); });
window.addEventListener('orientationchange', () => setTimeout(() => combat && resizeScene(), 200));

bindUI();
applyStaticText();
bootTitle();
lastT = performance.now();
rafId = requestAnimationFrame(loop);

})();
