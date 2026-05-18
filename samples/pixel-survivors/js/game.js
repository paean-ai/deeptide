// Pixel Survivors - engine, rendering, input, UI
(() => {
'use strict';

const SAVE_KEY = 'pixel-survivors-save';
const $ = id => document.getElementById(id);
const canvas = $('game');
const ctx = canvas.getContext('2d');
ctx.imageSmoothingEnabled = false;

// ---- save / meta -------------------------------------------------------
function loadSave() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)) || { gold: 0, meta: {} }; }
  catch (e) { return { gold: 0, meta: {} }; }
}
let save = loadSave();
function persist() { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); }
function metaLvl(id) { return save.meta[id] || 0; }

// ---- screens -----------------------------------------------------------
const SCREENS = ['title', 'armory', 'game'];
function showScreen(id) {
  SCREENS.forEach(s => $('screen-' + s).classList.toggle('hidden', s !== id));
}

// ---- world state -------------------------------------------------------
let G = null;
let rafId = 0, lastT = 0;
let enemyIdSeq = 0;

function newRun() {
  const maxHp = 100 + metaBonus('vigor', metaLvl('vigor'));
  G = {
    time: 0, kills: 0, gold: 0,
    player: {
      x: 0, y: 0, hp: maxHp, maxHp,
      baseSpeed: 132 * (1 + metaBonus('speed', metaLvl('speed'))),
      level: 1, xp: 0, xpNeed: xpForLevel(1),
      face: 1, invuln: 0, hitFlash: 0,
      weapons: [{ id: 'dagger', level: 1, cd: 0, orbAngle: 0 }],
      passives: {},
      revives: metaBonus('revive', metaLvl('revive')),
      anim: 0,
    },
    dmgMul: 1 + metaBonus('power', metaLvl('power')),
    xpMul: 1 + metaBonus('growth', metaLvl('growth')),
    goldMul: 1 + metaBonus('fortune', metaLvl('fortune')),
    regen: metaBonus('recovery', metaLvl('recovery')),
    enemies: [], projectiles: [], gems: [], coins: [], pickups: [],
    effects: [], particles: [],
    spawnTimer: 0, bossSchedule: [180, 360, 540, 720, RUN_GOAL_SECONDS],
    bossIndex: 0, levelQueue: 0, paused: false, over: false, won: false,
    cam: { x: 0, y: 0 },
  };
}
function xpForLevel(lv) { return 5 + lv * 5 + lv * lv * 2; }

// pull a pickup toward the player; returns true when collected.
// homing is "sticky" — once a pickup starts homing it follows even out of range.
function magnetToward(o, dt, range) {
  const p = G.player;
  const dx = p.x - o.x, dy = p.y - o.y;
  const d = Math.hypot(dx, dy) || 1;
  if (d < 20) return true;
  o.age = (o.age || 0) + dt;
  if (o.homing || d < range || o.age > 9) {
    o.homing = true;
    const sp = 260 + (range - Math.min(d, range)) * 4;
    const step = Math.min(d, sp * dt);
    o.x += dx / d * step;
    o.y += dy / d * step;
  }
  return false;
}

// ---- input -------------------------------------------------------------
const keys = {};
let joy = { active: false, ox: 0, oy: 0, dx: 0, dy: 0, id: null };
window.addEventListener('keydown', e => {
  keys[e.key.toLowerCase()] = true;
  if ([' ', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright'].includes(e.key.toLowerCase())) e.preventDefault();
});
window.addEventListener('keyup', e => { keys[e.key.toLowerCase()] = false; });

function moveVector() {
  let dx = 0, dy = 0;
  if (keys['a'] || keys['arrowleft']) dx -= 1;
  if (keys['d'] || keys['arrowright']) dx += 1;
  if (keys['w'] || keys['arrowup']) dy -= 1;
  if (keys['s'] || keys['arrowdown']) dy += 1;
  if (joy.active) { dx += joy.dx; dy += joy.dy; }
  const m = Math.hypot(dx, dy);
  if (m > 1) { dx /= m; dy /= m; }
  return { x: dx, y: dy };
}

function setupTouch() {
  const stage = $('stage');
  stage.addEventListener('touchstart', e => {
    for (const tch of e.changedTouches) {
      if (joy.id === null) {
        joy.id = tch.identifier; joy.active = true;
        joy.ox = tch.clientX; joy.oy = tch.clientY; joy.dx = 0; joy.dy = 0;
        updateJoyVisual(tch.clientX, tch.clientY, tch.clientX, tch.clientY);
      }
    }
    e.preventDefault();
  }, { passive: false });
  stage.addEventListener('touchmove', e => {
    for (const tch of e.changedTouches) {
      if (tch.identifier === joy.id) {
        let dx = tch.clientX - joy.ox, dy = tch.clientY - joy.oy;
        const m = Math.hypot(dx, dy), max = 56;
        if (m > max) { dx = dx / m * max; dy = dy / m * max; }
        joy.dx = dx / max; joy.dy = dy / max;
        updateJoyVisual(joy.ox, joy.oy, joy.ox + dx, joy.oy + dy);
      }
    }
    e.preventDefault();
  }, { passive: false });
  const end = e => {
    for (const tch of e.changedTouches) {
      if (tch.identifier === joy.id) {
        joy.id = null; joy.active = false; joy.dx = joy.dy = 0;
        $('joystick').classList.add('hidden');
      }
    }
  };
  stage.addEventListener('touchend', end);
  stage.addEventListener('touchcancel', end);
}
function updateJoyVisual(ox, oy, kx, ky) {
  const j = $('joystick'), k = $('joy-knob');
  j.classList.remove('hidden');
  j.style.left = ox + 'px'; j.style.top = oy + 'px';
  k.style.left = (kx - ox) + 'px'; k.style.top = (ky - oy) + 'px';
}

// ---- spawning ----------------------------------------------------------
function spawnWave() {
  const prof = spawnProfile(G.time / 60);
  for (let i = 0; i < prof.batch; i++) {
    const id = prof.pool[Math.floor(Math.random() * prof.pool.length)];
    spawnEnemy(id, prof.hpMul, prof.speedMul);
  }
}
function spawnEnemy(id, hpMul, speedMul, atX, atY) {
  if (G.enemies.length > 220) return;
  const def = ENEMIES[id] || BOSSES[id];
  const ang = Math.random() * Math.PI * 2;
  const dist = Math.max(canvas.width, canvas.height) * 0.62 + 40;
  const e = {
    eid: ++enemyIdSeq, id, def,
    x: atX != null ? atX : G.player.x + Math.cos(ang) * dist,
    y: atY != null ? atY : G.player.y + Math.sin(ang) * dist,
    hp: def.hp * (def.boss ? 1 : hpMul), maxHp: def.hp * (def.boss ? 1 : hpMul),
    speed: def.speed * (def.boss ? 1 : speedMul) * (0.85 + Math.random() * 0.3),
    dmg: def.dmg, size: def.size, color: def.color, sprite: def.sprite,
    boss: !!def.boss, xp: def.xp, anim: Math.random() * 9,
    hitFlash: 0, slow: 0, orbitCd: 0, knock: { x: 0, y: 0 },
  };
  G.enemies.push(e);
  return e;
}
function spawnBoss() {
  const order = ['warden', 'reaper', 'warden', 'reaper', 'overlord'];
  const id = order[Math.min(G.bossIndex, order.length - 1)];
  const def = BOSSES[id];
  const scale = 1 + G.bossIndex * 0.5;
  const e = spawnEnemy(id, 1, 1);
  e.hp = e.maxHp = def.hp * scale;
  e.dmg = def.dmg + G.bossIndex * 6;
  if (id === 'overlord') G.isOverlord = e;
  G.effects.push({ type: 'banner', life: 2.2, max: 2.2 });
  G.bossIndex++;
}

// ---- weapons -----------------------------------------------------------
function nearestEnemy(x, y, maxD) {
  let best = null, bd = maxD || 1e9;
  for (const e of G.enemies) {
    const d = Math.hypot(e.x - x, e.y - y);
    if (d < bd) { bd = d; best = e; }
  }
  return best;
}
function weaponStat(w) {
  return WEAPONS[w.id].tiers[w.level - 1];
}
function dmgMul() {
  const p = G.player;
  let m = G.dmgMul;
  if (p.passives.might) m *= 1 + passiveBonus('might', p.passives.might);
  return m;
}
function cdMul() {
  const p = G.player;
  let m = 1;
  if (p.passives.haste) m *= 1 - passiveBonus('haste', p.passives.haste);
  return Math.max(0.25, m);
}

function fireWeapon(w, dt) {
  const s = weaponStat(w);
  const p = G.player;
  if (w.id === 'orbit') {
    w.orbAngle += s.speed * dt;
    return;
  }
  w.cd -= dt;
  if (w.cd > 0) return;

  if (w.id === 'dagger') {
    const tgt = nearestEnemy(p.x, p.y);
    if (!tgt) { w.cd = 0.15; return; }
    const base = Math.atan2(tgt.y - p.y, tgt.x - p.x);
    for (let i = 0; i < s.count; i++) {
      const a = base + (i - (s.count - 1) / 2) * 0.18;
      G.projectiles.push({
        type: 'dagger', x: p.x, y: p.y,
        vx: Math.cos(a) * s.speed, vy: Math.sin(a) * s.speed,
        dmg: s.dmg * dmgMul(), pierce: s.pierce, life: 1.6, hit: new Set(),
        angle: a, color: WEAPONS.dagger.color,
      });
    }
    w.cd = s.cd * cdMul();
  } else if (w.id === 'aura') {
    let any = false;
    for (const e of G.enemies) {
      if (Math.hypot(e.x - p.x, e.y - p.y) < s.radius + e.size * 0.5) {
        hurtEnemy(e, s.dmg * dmgMul(), 0);
        any = true;
      }
    }
    w.cd = s.cd * cdMul();
    G.effects.push({ type: 'aura', x: p.x, y: p.y, r: s.radius, life: 0.22, max: 0.22 });
  } else if (w.id === 'bolt') {
    const targets = [];
    const pool = G.enemies.slice();
    for (let i = 0; i < s.count && pool.length; i++) {
      const idx = Math.floor(Math.random() * pool.length);
      targets.push(pool.splice(idx, 1)[0]);
    }
    for (const e of targets) {
      hurtEnemy(e, s.dmg * dmgMul(), 0);
      G.effects.push({ type: 'bolt', x: e.x, y: e.y, life: 0.2, max: 0.2 });
      burst(e.x, e.y, 6, WEAPONS.bolt.color);
    }
    w.cd = s.cd * cdMul();
  } else if (w.id === 'nova') {
    for (const e of G.enemies) {
      if (Math.hypot(e.x - p.x, e.y - p.y) < s.radius + e.size * 0.5) {
        hurtEnemy(e, s.dmg * dmgMul(), 0);
        e.slow = Math.max(e.slow, 1.2);
        e.slowAmt = s.slow;
      }
    }
    G.effects.push({ type: 'nova', x: p.x, y: p.y, r: s.radius, life: 0.5, max: 0.5 });
    w.cd = s.cd * cdMul();
  } else if (w.id === 'fireball') {
    for (let i = 0; i < s.count; i++) {
      const tgt = G.enemies.length
        ? G.enemies[Math.floor(Math.random() * G.enemies.length)]
        : null;
      const a = tgt ? Math.atan2(tgt.y - p.y, tgt.x - p.x) : Math.random() * 6.28;
      const tx = tgt ? tgt.x : p.x + Math.cos(a) * 200;
      const ty = tgt ? tgt.y : p.y + Math.sin(a) * 200;
      G.projectiles.push({
        type: 'fireball', x: p.x, y: p.y, tx, ty,
        vx: Math.cos(a) * s.speed, vy: Math.sin(a) * s.speed,
        dmg: s.dmg * dmgMul(), splash: s.splash, life: 3,
        spin: 0, color: WEAPONS.fireball.color,
      });
    }
    w.cd = s.cd * cdMul();
  }
}

function updateOrbiters(dt) {
  const p = G.player;
  for (const w of p.weapons) {
    if (w.id !== 'orbit') continue;
    const s = weaponStat(w);
    for (let i = 0; i < s.count; i++) {
      const a = w.orbAngle + (i / s.count) * Math.PI * 2;
      const ox = p.x + Math.cos(a) * s.radius;
      const oy = p.y + Math.sin(a) * s.radius;
      for (const e of G.enemies) {
        if (e.orbitCd > 0) continue;
        if (Math.hypot(e.x - ox, e.y - oy) < 14 + e.size * 0.5) {
          hurtEnemy(e, s.dmg * dmgMul(), 18, e.x - p.x, e.y - p.y);
          e.orbitCd = 0.45;
        }
      }
    }
  }
}

// ---- combat ------------------------------------------------------------
function hurtEnemy(e, dmg, knock, kx, ky) {
  e.hp -= dmg;
  e.hitFlash = 0.18;
  if (knock && !e.boss) {
    const m = Math.hypot(kx || 1, ky || 0) || 1;
    e.knock.x += (kx || 1) / m * knock;
    e.knock.y += (ky || 0) / m * knock;
  }
  if (e.hp <= 0) killEnemy(e);
}
function killEnemy(e) {
  e.dead = true;
  G.kills++;
  burst(e.x, e.y, e.boss ? 30 : 9, e.color);
  // drops
  if (e.boss) {
    G.pickups.push({ type: 'chest', x: e.x, y: e.y, anim: 0 });
    for (let i = 0; i < 8; i++)
      G.coins.push({ x: e.x + (Math.random() - 0.5) * 50, y: e.y + (Math.random() - 0.5) * 50, anim: Math.random() * 6, vy: -60 - Math.random() * 60 });
    if (e === G.isOverlord) victory();
  } else {
    G.gems.push({ x: e.x, y: e.y, value: e.xp, anim: Math.random() * 6 });
    if (Math.random() < 0.045) G.pickups.push({ type: 'heart', x: e.x, y: e.y, anim: 0 });
    if (Math.random() < 0.16) G.coins.push({ x: e.x, y: e.y, anim: Math.random() * 6, vy: -40 });
  }
}
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * 6.28, sp = 50 + Math.random() * 160;
    G.particles.push({ x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      life: 0.4 + Math.random() * 0.3, max: 0.7, size: 2 + Math.random() * 3, color });
  }
}

// ---- level up ----------------------------------------------------------
function gainXp(amount) {
  const p = G.player;
  p.xp += amount * G.xpMul;
  while (p.xp >= p.xpNeed) {
    p.xp -= p.xpNeed;
    p.level++;
    p.xpNeed = xpForLevel(p.level);
    G.levelQueue++;
  }
}
function buildDraft() {
  const p = G.player;
  const pool = [];
  // weapon level-ups
  for (const w of p.weapons) {
    if (w.level < 6) pool.push({ kind: 'wup', id: w.id, level: w.level + 1 });
  }
  // new weapons
  if (p.weapons.length < 6) {
    for (const id of WEAPON_IDS) {
      if (!p.weapons.some(w => w.id === id)) pool.push({ kind: 'wnew', id });
    }
  }
  // passive level-ups
  for (const id in p.passives) {
    if (p.passives[id] < PASSIVES[id].max) pool.push({ kind: 'pup', id, level: p.passives[id] + 1 });
  }
  // new passives
  if (Object.keys(p.passives).length < 6) {
    for (const id of PASSIVE_IDS) {
      if (!p.passives[id]) pool.push({ kind: 'pnew', id });
    }
  }
  // shuffle + take 3
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  const picks = pool.slice(0, 3);
  while (picks.length < 3) picks.push({ kind: 'heal' });
  return picks;
}
function showLevelUp() {
  G.paused = true;
  const draft = buildDraft();
  const box = $('levelup-cards');
  box.innerHTML = '';
  draft.forEach(d => {
    const card = document.createElement('button');
    card.className = 'lvl-card';
    let icon, name, desc, tag, evo = false;
    if (d.kind === 'wup') {
      icon = WEAPONS[d.id].icon; evo = d.level === 6;
      name = tw(d.id, evo); desc = evo ? t('evolved') : t('lvlOf', d.level);
      tag = t('weapons'); card.classList.add('is-weapon');
      if (evo) card.classList.add('is-evo');
    } else if (d.kind === 'wnew') {
      icon = WEAPONS[d.id].icon; name = tw(d.id, false);
      desc = twDesc(d.id); tag = t('newWeapon');
      card.classList.add('is-weapon', 'is-new');
    } else if (d.kind === 'pup') {
      icon = PASSIVES[d.id].icon; name = tp(d.id);
      desc = tpDesc(d.id) + ' (' + t('lvlOf', d.level) + ')'; tag = t('items');
      card.classList.add('is-item');
    } else if (d.kind === 'pnew') {
      icon = PASSIVES[d.id].icon; name = tp(d.id);
      desc = tpDesc(d.id); tag = t('newItem');
      card.classList.add('is-item', 'is-new');
    } else {
      icon = '❤'; name = currentLang === 'zh' ? '回复' : 'Recover';
      desc = currentLang === 'zh' ? '恢复 40 点生命' : 'Heal 40 HP'; tag = '';
      card.classList.add('is-heal');
    }
    card.innerHTML = `<div class="lc-tag">${tag}</div>` +
      `<div class="lc-icon">${icon}</div>` +
      `<div class="lc-name">${name}</div>` +
      `<div class="lc-desc">${desc}</div>`;
    card.onclick = () => applyDraft(d);
    box.appendChild(card);
  });
  $('overlay-levelup').classList.remove('hidden');
}
function applyDraft(d) {
  const p = G.player;
  if (d.kind === 'wup') {
    p.weapons.find(w => w.id === d.id).level = d.level;
  } else if (d.kind === 'wnew') {
    p.weapons.push({ id: d.id, level: 1, cd: 0, orbAngle: Math.random() * 6.28 });
  } else if (d.kind === 'pup') {
    p.passives[d.id] = d.level;
    if (d.id === 'vitality') p.maxHp += 24, p.hp += 24;
  } else if (d.kind === 'pnew') {
    p.passives[d.id] = 1;
    if (d.id === 'vitality') p.maxHp += 24, p.hp += 24;
  } else {
    p.hp = Math.min(p.maxHp, p.hp + 40);
  }
  $('overlay-levelup').classList.add('hidden');
  G.levelQueue--;
  if (G.levelQueue > 0) showLevelUp();
  else G.paused = false;
  renderHud();
}

// ---- update ------------------------------------------------------------
function update(dt) {
  const p = G.player;
  G.time += dt;

  // boss timing
  if (G.bossIndex < G.bossSchedule.length && G.time >= G.bossSchedule[G.bossIndex]) {
    spawnBoss();
  }

  // spawn
  G.spawnTimer -= dt;
  if (G.spawnTimer <= 0 && G.time < RUN_GOAL_SECONDS + 30) {
    const prof = spawnProfile(G.time / 60);
    G.spawnTimer = prof.interval;
    spawnWave();
  }

  // player movement
  const mv = moveVector();
  if (mv.x !== 0) p.face = mv.x > 0 ? 1 : -1;
  let speed = p.baseSpeed;
  if (p.passives.swift) speed *= 1 + passiveBonus('swift', p.passives.swift);
  p.x += mv.x * speed * dt;
  p.y += mv.y * speed * dt;
  p.anim += dt * (mv.x || mv.y ? 1 : 0.2);
  if (p.invuln > 0) p.invuln -= dt;
  if (p.hitFlash > 0) p.hitFlash -= dt * 3;
  if (G.regen > 0) p.hp = Math.min(p.maxHp, p.hp + G.regen * dt);

  // camera
  G.cam.x += (p.x - G.cam.x) * Math.min(1, dt * 9);
  G.cam.y += (p.y - G.cam.y) * Math.min(1, dt * 9);

  // weapons
  for (const w of p.weapons) fireWeapon(w, dt);
  updateOrbiters(dt);

  // enemies
  for (const e of G.enemies) {
    if (e.dead) continue;
    if (e.hitFlash > 0) e.hitFlash -= dt;
    if (e.slow > 0) e.slow -= dt;
    e.anim += dt;
    const sMul = e.slow > 0 ? (1 - (e.slowAmt || 0.4)) : 1;
    const dx = p.x - e.x, dy = p.y - e.y;
    const d = Math.hypot(dx, dy) || 1;
    e.x += dx / d * e.speed * sMul * dt + e.knock.x * dt;
    e.y += dy / d * e.speed * sMul * dt + e.knock.y * dt;
    e.knock.x *= 0.88; e.knock.y *= 0.88;
    if (e.orbitCd > 0) e.orbitCd -= dt;
    // contact
    if (d < 14 + e.size * 0.46 && p.invuln <= 0) {
      let dmg = e.dmg;
      if (p.passives.armor) dmg = Math.max(1, dmg - passiveBonus('armor', p.passives.armor));
      p.hp -= dmg;
      p.invuln = 0.75; p.hitFlash = 1;
      if (p.hp <= 0) handleDeath();
    }
  }
  G.enemies = G.enemies.filter(e => !e.dead);

  // projectiles
  for (const pr of G.projectiles) {
    pr.life -= dt;
    if (pr.type === 'dagger') {
      pr.x += pr.vx * dt; pr.y += pr.vy * dt;
      for (const e of G.enemies) {
        if (pr.hit.has(e.eid)) continue;
        if (Math.hypot(e.x - pr.x, e.y - pr.y) < e.size * 0.5 + 6) {
          hurtEnemy(e, pr.dmg, 20, pr.vx, pr.vy);
          pr.hit.add(e.eid); pr.pierce--;
          burst(pr.x, pr.y, 4, pr.color);
          if (pr.pierce < 0) { pr.life = 0; break; }
        }
      }
    } else if (pr.type === 'fireball') {
      pr.x += pr.vx * dt; pr.y += pr.vy * dt; pr.spin += dt * 12;
      let boom = pr.life <= 0;
      if (Math.hypot(pr.x - pr.tx, pr.y - pr.ty) < 16) boom = true;
      for (const e of G.enemies) {
        if (Math.hypot(e.x - pr.x, e.y - pr.y) < e.size * 0.5 + 8) { boom = true; break; }
      }
      if (boom) {
        pr.life = 0;
        G.effects.push({ type: 'boom', x: pr.x, y: pr.y, r: pr.splash, life: 0.34, max: 0.34 });
        burst(pr.x, pr.y, 18, pr.color);
        for (const e of G.enemies) {
          if (Math.hypot(e.x - pr.x, e.y - pr.y) < pr.splash + e.size * 0.5)
            hurtEnemy(e, pr.dmg, 30, e.x - pr.x, e.y - pr.y);
        }
      }
    }
  }
  G.projectiles = G.projectiles.filter(pr => pr.life > 0);

  // gems / coins / pickups magnet + collect
  let pickRange = 112;
  if (p.passives.magnet) pickRange *= 1 + passiveBonus('magnet', p.passives.magnet);
  for (const g of G.gems) {
    g.anim += dt;
    if (magnetToward(g, dt, pickRange)) { g.taken = true; gainXp(g.value); }
  }
  G.gems = G.gems.filter(g => !g.taken);
  for (const c of G.coins) {
    c.anim += dt;
    if (c.tossT == null) c.tossT = 0;
    c.tossT += dt;
    if (c.tossT > 0.35) {
      if (magnetToward(c, dt, pickRange)) { c.taken = true; G.gold += Math.round(3 * G.goldMul); }
    }
  }
  G.coins = G.coins.filter(c => !c.taken);
  for (const pk of G.pickups) {
    pk.anim += dt;
    const d = Math.hypot(pk.x - p.x, pk.y - p.y);
    if (d < 22) {
      pk.taken = true;
      if (pk.type === 'heart') p.hp = Math.min(p.maxHp, p.hp + 35);
      else { // chest
        G.gold += Math.round(60 * G.goldMul);
        G.levelQueue += 2;
      }
    }
  }
  G.pickups = G.pickups.filter(pk => !pk.taken);

  // effects / particles
  for (const ef of G.effects) ef.life -= dt;
  G.effects = G.effects.filter(ef => ef.life > 0);
  for (const pt of G.particles) { pt.life -= dt; pt.x += pt.vx * dt; pt.y += pt.vy * dt; pt.vx *= 0.92; pt.vy *= 0.92; }
  G.particles = G.particles.filter(pt => pt.life > 0);

  if (G.levelQueue > 0 && !G.paused) showLevelUp();
}

function handleDeath() {
  const p = G.player;
  if (p.revives > 0) {
    p.revives--;
    p.hp = Math.floor(p.maxHp * 0.55);
    p.invuln = 2.5;
    G.effects.push({ type: 'nova', x: p.x, y: p.y, r: 240, life: 0.6, max: 0.6 });
    for (const e of G.enemies) {
      if (Math.hypot(e.x - p.x, e.y - p.y) < 240) hurtEnemy(e, 9999, 0);
    }
    G.effects.push({ type: 'revive', life: 1.6, max: 1.6 });
  } else {
    gameOver(false);
  }
}

// ---- end of run --------------------------------------------------------
function endRun(won) {
  G.over = true; G.won = won;
  const earned = G.gold + Math.floor(G.kills / 4) + Math.floor(G.time / 12);
  save.gold += earned;
  persist();
  $('result-title').textContent = won ? t('victory') : t('gameOver');
  $('result-title').className = won ? 'win' : 'lose';
  $('result-stats').innerHTML =
    `<div>${t('survivedFor', G.time)}</div>` +
    `<div>${t('killsLabel')}: ${G.kills} &nbsp; ${t('levelLabel')}: ${G.player.level}</div>` +
    `<div class="result-gold">${t('goldEarned', earned)}</div>`;
  $('overlay-result').classList.remove('hidden');
}
function gameOver(won) { if (!G.over) endRun(won); }
function victory() { if (!G.over) endRun(true); }

// ---- rendering ---------------------------------------------------------
function render() {
  const W = canvas.width, H = canvas.height;
  const cx = G.cam.x, cy = G.cam.y;
  const ox = W / 2 - cx, oy = H / 2 - cy;
  drawGround(ctx, cx, cy, W, H);

  // gems
  for (const g of G.gems) drawGem(ctx, g.x + ox, g.y + oy, g.value, g.anim);
  for (const c of G.coins) drawCoin(ctx, c.x + ox, c.y + oy, c.anim);
  for (const pk of G.pickups) {
    if (pk.type === 'heart') drawHeart(ctx, pk.x + ox, pk.y + oy);
    else drawChest(ctx, pk.x + ox, pk.y + oy, pk.anim);
  }

  // effects under entities
  for (const ef of G.effects) {
    const a = ef.life / ef.max;
    if (ef.type === 'aura') {
      ctx.globalAlpha = a * 0.4; ctx.fillStyle = '#9cffd0';
      ctx.beginPath(); ctx.arc(ef.x + ox, ef.y + oy, ef.r, 0, 6.28); ctx.fill();
    } else if (ef.type === 'nova') {
      ctx.globalAlpha = a * 0.6; ctx.strokeStyle = '#9ce6ff'; ctx.lineWidth = 6;
      ctx.beginPath(); ctx.arc(ef.x + ox, ef.y + oy, ef.r * (1 - a) + 20, 0, 6.28); ctx.stroke();
    } else if (ef.type === 'boom') {
      ctx.globalAlpha = a; ctx.fillStyle = '#ff9c4d';
      ctx.beginPath(); ctx.arc(ef.x + ox, ef.y + oy, ef.r * (1.2 - a), 0, 6.28); ctx.fill();
    }
    ctx.globalAlpha = 1;
  }

  // enemies sorted by y
  const sorted = G.enemies.slice().sort((a, b) => a.y - b.y);
  for (const e of sorted) {
    drawEnemy(ctx, e.sprite, e.x + ox, e.y + oy, e.size, e.anim, e.color,
      e.hp / e.maxHp, e.hitFlash > 0 ? e.hitFlash / 0.18 : 0, e.slow);
  }

  // player
  const p = G.player;
  drawHero(ctx, p.x + ox, p.y + oy, p.anim, p.face, Math.max(0, p.hitFlash), p.invuln);

  // orbiters
  for (const w of p.weapons) {
    if (w.id !== 'orbit') continue;
    const s = weaponStat(w);
    for (let i = 0; i < s.count; i++) {
      const a = w.orbAngle + (i / s.count) * 6.28;
      const bx = p.x + Math.cos(a) * s.radius + ox;
      const by = p.y + Math.sin(a) * s.radius + oy;
      ctx.save(); ctx.translate(bx, by); ctx.rotate(a * 3);
      ctx.fillStyle = WEAPONS.orbit.color;
      ctx.fillRect(-9, -3, 18, 6); ctx.fillRect(-3, -9, 6, 18);
      ctx.fillStyle = '#fff'; ctx.fillRect(-2, -2, 4, 4);
      ctx.restore();
    }
  }

  // projectiles
  for (const pr of G.projectiles) {
    if (pr.type === 'dagger') {
      ctx.save(); ctx.translate(pr.x + ox, pr.y + oy); ctx.rotate(pr.angle);
      ctx.fillStyle = pr.color; ctx.fillRect(-7, -2, 14, 4);
      ctx.fillStyle = '#fff'; ctx.fillRect(3, -1, 5, 2);
      ctx.restore();
    } else {
      ctx.save(); ctx.translate(pr.x + ox, pr.y + oy); ctx.rotate(pr.spin);
      ctx.fillStyle = pr.color;
      ctx.fillRect(-6, -6, 12, 12);
      ctx.fillStyle = '#ffe0a0'; ctx.fillRect(-3, -3, 6, 6);
      ctx.restore();
    }
  }

  // bolt effects (over)
  for (const ef of G.effects) {
    if (ef.type === 'bolt') {
      const a = ef.life / ef.max;
      ctx.globalAlpha = a; ctx.strokeStyle = '#bda6ff'; ctx.lineWidth = 4;
      ctx.beginPath();
      let zx = ef.x + ox, zy = ef.y + oy - 240;
      ctx.moveTo(zx, zy);
      for (let i = 0; i < 6; i++) { zx += (Math.random() - 0.5) * 20; zy += 40; ctx.lineTo(zx, zy); }
      ctx.stroke(); ctx.globalAlpha = 1;
    }
  }

  // particles
  for (const pt of G.particles) {
    ctx.globalAlpha = Math.max(0, pt.life / pt.max);
    ctx.fillStyle = pt.color;
    ctx.fillRect(pt.x + ox - pt.size / 2, pt.y + oy - pt.size / 2, pt.size, pt.size);
  }
  ctx.globalAlpha = 1;

  // boss banner / goal text
  for (const ef of G.effects) {
    if (ef.type === 'banner') {
      ctx.globalAlpha = Math.min(1, ef.life);
      ctx.fillStyle = 'rgba(20,16,30,0.9)';
      ctx.fillRect(W / 2 - 150, 70, 300, 40);
      ctx.fillStyle = '#ff5a5a';
      ctx.font = 'bold 20px monospace'; ctx.textAlign = 'center';
      ctx.fillText('⚠ BOSS ⚠', W / 2, 96);
      ctx.globalAlpha = 1;
    } else if (ef.type === 'revive') {
      ctx.globalAlpha = Math.min(1, ef.life);
      ctx.fillStyle = '#ffd34d'; ctx.font = 'bold 30px monospace'; ctx.textAlign = 'center';
      ctx.fillText(t('revived'), W / 2, H / 2 - 60);
      ctx.globalAlpha = 1;
    }
  }
}

// ---- HUD ---------------------------------------------------------------
function renderHud() {
  if (!G) return;
  const p = G.player;
  $('hud-hp-fill').style.width = Math.max(0, p.hp / p.maxHp * 100) + '%';
  $('hud-hp-text').textContent = Math.max(0, Math.ceil(p.hp)) + '/' + p.maxHp;
  $('hud-xp-fill').style.width = (p.xp / p.xpNeed * 100) + '%';
  $('hud-level').textContent = t('lvl') + ' ' + p.level;
  $('hud-time').textContent = fmtTime(G.time);
  $('hud-kills').textContent = '☠ ' + G.kills;
  $('hud-gold').textContent = '◆ ' + G.gold;
  // weapon/item strip
  const strip = $('hud-weapons');
  strip.innerHTML = '';
  for (const w of p.weapons) {
    const d = document.createElement('div');
    d.className = 'wchip' + (w.level === 6 ? ' evo' : '');
    d.innerHTML = `<span>${WEAPONS[w.id].icon}</span><b>${w.level}</b>`;
    strip.appendChild(d);
  }
  for (const id in p.passives) {
    const d = document.createElement('div');
    d.className = 'wchip item';
    d.innerHTML = `<span>${PASSIVES[id].icon}</span><b>${p.passives[id]}</b>`;
    strip.appendChild(d);
  }
}

// ---- loop --------------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  let dt = (now - lastT) / 1000;
  lastT = now;
  if (dt > 0.05) dt = 0.05;
  if (G && !$('screen-game').classList.contains('hidden')) {
    if (!G.paused && !G.over) update(dt);
    render();
    renderHud();
  }
}

// ---- resize ------------------------------------------------------------
function resize() {
  const stage = $('stage');
  canvas.width = stage.clientWidth;
  canvas.height = stage.clientHeight;
  ctx.imageSmoothingEnabled = false;
}
window.addEventListener('resize', resize);
window.addEventListener('orientationchange', () => setTimeout(resize, 200));

// ---- armory ------------------------------------------------------------
function renderArmory() {
  $('armory-bank').textContent = '◆ ' + save.gold;
  const grid = $('armory-grid');
  grid.innerHTML = '';
  for (const id in META) {
    const lvl = metaLvl(id), m = META[id];
    const card = document.createElement('div');
    card.className = 'meta-card';
    const maxed = lvl >= m.max;
    const cost = metaCost(id, lvl);
    card.innerHTML =
      `<div class="mc-name">${tMeta(id)}</div>` +
      `<div class="mc-desc">${tMetaDesc(id)}</div>` +
      `<div class="mc-pips">${pips(lvl, m.max)}</div>`;
    const btn = document.createElement('button');
    btn.className = 'mc-buy';
    if (maxed) { btn.textContent = t('maxed'); btn.disabled = true; }
    else {
      btn.textContent = t('buy') + ' ◆' + cost;
      btn.disabled = save.gold < cost;
      btn.onclick = () => {
        if (save.gold < cost) return;
        save.gold -= cost;
        save.meta[id] = lvl + 1;
        persist();
        renderArmory();
      };
    }
    card.appendChild(btn);
    grid.appendChild(card);
  }
}
function pips(n, max) {
  let s = '';
  for (let i = 0; i < max; i++) s += `<i class="${i < n ? 'on' : ''}"></i>`;
  return s;
}

// ---- start / overlays --------------------------------------------------
function startRun() {
  newRun();
  showScreen('game');
  resize();
  $('overlay-result').classList.add('hidden');
  $('overlay-levelup').classList.add('hidden');
  $('overlay-pause').classList.add('hidden');
  G.player.weapons[0].cd = 0.3;
  renderHud();
}
function togglePause(on) {
  if (!G || G.over) return;
  if (G.levelQueue > 0) return;
  G.paused = on;
  $('overlay-pause').classList.toggle('hidden', !on);
}

function bindUI() {
  $('btn-play').onclick = startRun;
  $('btn-armory').onclick = () => { renderArmory(); showScreen('armory'); };
  $('btn-armory-back').onclick = () => showScreen('title');
  $('btn-pause').onclick = () => togglePause(true);
  $('btn-resume').onclick = () => togglePause(false);
  $('btn-pause-restart').onclick = () => { $('overlay-pause').classList.add('hidden'); startRun(); };
  $('btn-pause-quit').onclick = () => { $('overlay-pause').classList.add('hidden'); showScreen('title'); };
  $('btn-result-again').onclick = startRun;
  $('btn-result-menu').onclick = () => { $('overlay-result').classList.add('hidden'); showScreen('title'); };
  setupTouch();
  setupLanguageToggle(() => {
    if (!$('screen-armory').classList.contains('hidden')) renderArmory();
    if (G && !$('screen-game').classList.contains('hidden')) renderHud();
  });
}

bindUI();
applyStaticText();
showScreen('title');
resize();
lastT = performance.now();
rafId = requestAnimationFrame(loop);

})();
