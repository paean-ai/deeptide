// Pixel Tower Defense - engine, rendering, input, UI
(() => {
'use strict';

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
canvas.width = FIELD_W;
canvas.height = FIELD_H;
ctx.imageSmoothingEnabled = false;

const SAVE_KEY = 'pixel-td-save';
function loadSave() {
  try { return JSON.parse(localStorage.getItem(SAVE_KEY)) || { levels: {} }; }
  catch (e) { return { levels: {} }; }
}
function persist() { localStorage.setItem(SAVE_KEY, JSON.stringify(save)); }
let save = loadSave();

// ---- screen management -------------------------------------------------
const screens = {
  title: document.getElementById('screen-title'),
  select: document.getElementById('screen-select'),
  game: document.getElementById('screen-game'),
};
function showScreen(id) {
  for (const k in screens) screens[k].classList.toggle('hidden', k !== id);
}

// ---- game state --------------------------------------------------------
let G = null;          // active game
let placingType = null; // tower type pending placement
let selectedTower = null;
let rafId = 0;
let lastTime = 0;
let hoverTile = null;

function newGame(level, endless) {
  const lv = LEVELS[level];
  // build pixel path + path tile set
  const path = lv.path.map(([tx, ty]) => ({ x: tx * TILE + TILE / 2, y: ty * TILE + TILE / 2 }));
  const pathTiles = new Set();
  for (let i = 0; i < lv.path.length - 1; i++) {
    let [x0, y0] = lv.path[i], [x1, y1] = lv.path[i + 1];
    const steps = Math.abs(x1 - x0) + Math.abs(y1 - y0);
    for (let s = 0; s <= steps; s++) {
      const tx = Math.round(x0 + (x1 - x0) * (s / steps));
      const ty = Math.round(y0 + (y1 - y0) * (s / steps));
      pathTiles.add(tx + ',' + ty);
    }
  }
  const decoTiles = new Set(lv.decos.map(([x, y]) => x + ',' + y));
  // path total length
  let pathLen = 0;
  for (let i = 0; i < path.length - 1; i++) pathLen += dist(path[i], path[i + 1]);

  G = {
    level, levelDef: lv, endless: !!endless,
    path, pathTiles, decoTiles, pathLen,
    gold: lv.startGold, lives: lv.startLives,
    waves: buildWaves(lv), waveIndex: 0,
    waveActive: false, prepTimer: 6, prepMax: 6, awaitingFirst: true,
    spawnQueue: [], enemies: [], towers: [], projectiles: [],
    beams: [], particles: [], texts: [],
    speed: 1, paused: false, over: false, won: false,
    time: 0, livesLost: 0, totalSpawned: 0, totalKilled: 0,
  };
  placingType = null;
  selectedTower = null;
  hoverTile = null;
}

function dist(a, b) { return Math.hypot(a.x - b.x, a.y - b.y); }

// ---- wave system -------------------------------------------------------
function startWave() {
  if (!G || G.waveActive || G.over) return;
  // early-call bonus
  if (!G.awaitingFirst && G.prepTimer > 0.4) {
    const bonus = Math.ceil(G.prepTimer) * 3;
    G.gold += bonus;
    addText(FIELD_W / 2, 60, '+' + bonus, '#ffd34d');
  }
  G.awaitingFirst = false;
  G.waveActive = true;
  const wave = G.endless || G.waveIndex >= G.waves.length
    ? buildEndlessWave(G.waveIndex - G.waves.length + 1, G.levelDef)
    : G.waves[G.waveIndex];
  G.currentHpMul = wave.hpMul;
  for (const grp of wave.groups) {
    for (let i = 0; i < grp.count; i++) {
      G.spawnQueue.push({ type: grp.type, at: grp.delay + i * grp.interval });
    }
  }
  G.spawnQueue.sort((a, b) => a.at - b.at);
  G.spawnTimer = 0;
  updateHUD();
}

function spawnEnemy(type) {
  const base = ENEMIES[type];
  const hp = Math.round(base.hp * G.currentHpMul);
  const e = {
    type, x: G.path[0].x, y: G.path[0].y,
    maxHp: hp, hp, speed: base.speed, baseSpeed: base.speed,
    armor: base.armor, resist: base.resist, air: base.air,
    reward: base.reward, size: base.size, leak: base.leak,
    seg: 0, dist: 0, alive: true,
    slowAmt: 0, slowTimer: 0, frozenTimer: 0,
    burnDps: 0, burnTimer: 0, shredAmt: 0, shredTimer: 0,
    healRate: base.healRate || 0, healRange: base.healRange || 0,
    anim: Math.random() * 10,
  };
  G.enemies.push(e);
  G.totalSpawned++;
}

// ---- combat ------------------------------------------------------------
function damageEnemy(e, amount, element, opts) {
  if (!e.alive) return;
  opts = opts || {};
  let dmg = amount;
  if (element === 'physical') {
    dmg = Math.max(1, dmg - Math.max(0, e.armor - e.shredAmt));
  } else {
    dmg = dmg * (1 - e.resist);
  }
  if (opts.crit) dmg *= opts.critMult || 2;
  e.hp -= dmg;
  if (e.hp <= 0) killEnemy(e);
}

function killEnemy(e) {
  if (!e.alive) return;
  e.alive = false;
  G.gold += e.reward;
  G.totalKilled++;
  addText(e.x, e.y - e.size * 0.5, '+' + e.reward, '#ffd34d');
  burst(e.x, e.y, e.type === 'boss' ? 28 : 10, ENEMY_PALETTE[e.type].b);
}

function applySlow(e, amt, dur) {
  if (amt >= e.slowAmt || e.slowTimer < 0.15) { e.slowAmt = amt; }
  e.slowTimer = Math.max(e.slowTimer, dur);
}

// ---- towers ------------------------------------------------------------
function placeTower(gx, gy, type) {
  const def = TOWERS[type];
  if (G.gold < def.baseCost) { flash(t('cantAfford')); return false; }
  if (!isBuildable(gx, gy)) { flash(t('blocked')); return false; }
  G.gold -= def.baseCost;
  const stats = towerStats(type, 1);
  G.towers.push({
    type, gx, gy,
    x: gx * TILE + TILE / 2, y: gy * TILE + TILE / 2,
    tier: 1, branch: null, stats,
    angle: 0, cooldown: 0, targetMode: 'first',
  });
  updateHUD();
  return true;
}

function isBuildable(gx, gy) {
  if (gx < 0 || gy < 0 || gx >= GRID_W || gy >= GRID_H) return false;
  const key = gx + ',' + gy;
  if (G.pathTiles.has(key) || G.decoTiles.has(key)) return false;
  return !G.towers.some(tw => tw.gx === gx && tw.gy === gy);
}

function upgradeTower(tw, branch) {
  if (tw.tier >= 3) return;
  let cost;
  if (tw.tier === 1) cost = TOWERS[tw.type].tiers[1].cost;
  else cost = TOWERS[tw.type].branches[branch].cost;
  if (G.gold < cost) { flash(t('cantAfford')); return; }
  G.gold -= cost;
  tw.tier++;
  if (tw.tier === 3) tw.branch = branch;
  tw.stats = towerStats(tw.type, tw.tier, tw.branch);
  burst(tw.x, tw.y, 14, '#ffd34d');
  updateHUD();
  renderTowerPanel();
}

function sellTower(tw) {
  const invested = towerInvested(tw.type, tw.tier, tw.branch);
  const refund = Math.floor(invested * 0.7);
  G.gold += refund;
  addText(tw.x, tw.y, '+' + refund, '#ffd34d');
  G.towers = G.towers.filter(x => x !== tw);
  selectedTower = null;
  document.getElementById('tower-panel').classList.add('hidden');
  updateHUD();
}

function towerInRange(tw, e) {
  if (!e.alive) return false;
  if (e.air && !tw.stats.air) return false;
  return dist(tw, e) <= tw.stats.range;
}

function pickTarget(tw) {
  let best = null, bestVal = -Infinity;
  for (const e of G.enemies) {
    if (!towerInRange(tw, e)) continue;
    let val;
    switch (tw.targetMode) {
      case 'last': val = -e.dist; break;
      case 'strong': val = e.hp; break;
      case 'close': val = -dist(tw, e); break;
      default: val = e.dist;
    }
    if (val > bestVal) { bestVal = val; best = e; }
  }
  return best;
}

function fireTower(tw) {
  const s = tw.stats;
  if (s.mode === 'aura') {
    // blizzard: hit everything in range
    let hit = false;
    for (const e of G.enemies) {
      if (!towerInRange(tw, e)) continue;
      hit = true;
      damageEnemy(e, s.damage, s.element);
      applySlow(e, s.slow, s.slowDur);
    }
    if (hit) {
      G.beams.push({ aura: true, x: tw.x, y: tw.y, r: s.range, life: 0.3, color: TOWERS[tw.type].color });
    }
    return;
  }
  const target = pickTarget(tw);
  if (!target) return;
  tw.angle = Math.atan2(target.y - tw.y, target.x - tw.x);
  const crit = Math.random() < (s.crit || 0);

  if (s.mode === 'projectile') {
    G.projectiles.push({
      x: tw.x, y: tw.y - 6, target, type: tw.type,
      speed: s.projSpeed, damage: s.damage, element: s.element,
      splash: s.splash || 0, shred: s.shred || 0,
      crit, critMult: s.critMult || 2, color: TOWERS[tw.type].color,
      lastX: target.x, lastY: target.y,
    });
  } else if (s.mode === 'beam') {
    if (tw.type === 'arcane') {
      // chain lightning
      const hitList = [target];
      let from = target;
      const chain = s.chain || 1;
      for (let c = 1; c < chain; c++) {
        let next = null, nd = s.chainRange;
        for (const e of G.enemies) {
          if (hitList.includes(e) || !e.alive) continue;
          if (e.air && !s.air) continue;
          const d = dist(from, e);
          if (d < nd) { nd = d; next = e; }
        }
        if (!next) break;
        hitList.push(next);
        from = next;
      }
      let prev = { x: tw.x, y: tw.y - 6 };
      for (const e of hitList) {
        damageEnemy(e, s.damage, s.element, { crit, critMult: s.critMult || 2 });
        G.beams.push({ x1: prev.x, y1: prev.y, x2: e.x, y2: e.y, life: 0.18, color: TOWERS[tw.type].color });
        burst(e.x, e.y, 4, TOWERS[tw.type].color);
        prev = e;
      }
    } else {
      // frost beam
      damageEnemy(target, s.damage, s.element, { crit });
      applySlow(target, s.slow, s.slowDur);
      if (s.freezeChance && Math.random() < s.freezeChance) {
        target.frozenTimer = Math.max(target.frozenTimer, s.freezeDur);
      }
      G.beams.push({ x1: tw.x, y1: tw.y - 6, x2: target.x, y2: target.y, life: 0.16, color: TOWERS[tw.type].color });
      burst(target.x, target.y, 4, TOWERS[tw.type].color);
    }
  }
}

// ---- effects -----------------------------------------------------------
function burst(x, y, n, color) {
  for (let i = 0; i < n; i++) {
    const a = Math.random() * Math.PI * 2;
    const sp = 40 + Math.random() * 130;
    G.particles.push({
      x, y, vx: Math.cos(a) * sp, vy: Math.sin(a) * sp - 20,
      life: 0.4 + Math.random() * 0.4, maxLife: 0.8,
      size: 2 + Math.random() * 3, color,
    });
  }
}
function addText(x, y, str, color) {
  G.texts.push({ x, y, str, color, life: 0.9 });
}
let flashMsg = '', flashTimer = 0;
function flash(msg) { flashMsg = msg; flashTimer = 1.4; }

// ---- update ------------------------------------------------------------
function update(dt) {
  G.time += dt;
  if (flashTimer > 0) flashTimer -= dt;

  // wave prep / spawning
  if (!G.waveActive && !G.over) {
    const wavesLeft = G.endless || G.waveIndex < G.waves.length;
    if (!G.awaitingFirst && wavesLeft) {
      G.prepTimer -= dt;
      if (G.prepTimer <= 0) startWave();
    }
  }
  if (G.waveActive) {
    G.spawnTimer += dt;
    while (G.spawnQueue.length && G.spawnQueue[0].at <= G.spawnTimer) {
      spawnEnemy(G.spawnQueue.shift().type);
    }
    if (G.spawnQueue.length === 0 && G.enemies.length === 0) {
      // wave cleared
      G.waveActive = false;
      G.waveIndex++;
      if (!G.endless && G.waveIndex >= G.waves.length) {
        victory();
      } else {
        G.prepTimer = G.prepMax;
      }
    }
  }

  // enemies
  for (const e of G.enemies) {
    if (!e.alive) continue;
    // status timers
    if (e.slowTimer > 0) { e.slowTimer -= dt; if (e.slowTimer <= 0) e.slowAmt = 0; }
    if (e.frozenTimer > 0) e.frozenTimer -= dt;
    if (e.shredTimer > 0) { e.shredTimer -= dt; if (e.shredTimer <= 0) e.shredAmt = 0; }
    if (e.burnTimer > 0) {
      e.burnTimer -= dt;
      damageEnemy(e, e.burnDps * dt, 'magic');
      if (!e.alive) continue;
    }
    e.anim += dt;
    // healer aura
    if (e.healRate > 0) {
      for (const o of G.enemies) {
        if (o === e || !o.alive || o.hp >= o.maxHp) continue;
        if (dist(e, o) <= e.healRange) {
          o.hp = Math.min(o.maxHp, o.hp + e.healRate * dt);
        }
      }
    }
    // movement
    const mult = e.frozenTimer > 0 ? 0 : (1 - e.slowAmt);
    let move = e.speed * mult * dt;
    while (move > 0 && e.seg < G.path.length - 1) {
      const tgt = G.path[e.seg + 1];
      const d = Math.hypot(tgt.x - e.x, tgt.y - e.y);
      if (d <= move) {
        e.x = tgt.x; e.y = tgt.y; e.dist += d; move -= d; e.seg++;
      } else {
        e.x += (tgt.x - e.x) / d * move;
        e.y += (tgt.y - e.y) / d * move;
        e.dist += move; move = 0;
      }
    }
    if (e.seg >= G.path.length - 1) {
      e.alive = false;
      G.lives -= e.leak; G.livesLost += e.leak;
      flash('-' + e.leak + ' ' + t('lives'));
      if (G.lives <= 0) { G.lives = 0; defeat(); }
    }
  }
  G.enemies = G.enemies.filter(e => e.alive);

  // towers
  for (const tw of G.towers) {
    tw.cooldown -= dt;
    // aim at nearest even when not firing for feel
    if (tw.cooldown <= 0) {
      const before = G.enemies.length;
      fireTower(tw);
      if (tw.stats.mode === 'aura' || pickTargetExists(tw)) tw.cooldown = tw.stats.interval;
      else tw.cooldown = 0.05;
    } else {
      const tgt = pickTarget(tw);
      if (tgt) tw.angle = Math.atan2(tgt.y - tw.y, tgt.x - tw.x);
    }
  }

  // projectiles
  for (const p of G.projectiles) {
    let tx, ty;
    if (p.target && p.target.alive) { tx = p.target.x; ty = p.target.y; p.lastX = tx; p.lastY = ty; }
    else { tx = p.lastX; ty = p.lastY; }
    const d = Math.hypot(tx - p.x, ty - p.y);
    const step = p.speed * dt;
    if (d <= step) {
      p.x = tx; p.y = ty; p.done = true;
      if (p.splash > 0) {
        burst(tx, ty, 16, p.color);
        for (const e of G.enemies) {
          if (e.air) continue;
          if (dist({ x: tx, y: ty }, e) <= p.splash) {
            damageEnemy(e, p.damage, p.element, { crit: p.crit, critMult: p.critMult });
            if (p.shred) { e.shredAmt = Math.max(e.shredAmt, p.shred); e.shredTimer = 3.5; }
          }
        }
      } else if (p.target && p.target.alive) {
        damageEnemy(p.target, p.damage, p.element, { crit: p.crit, critMult: p.critMult });
        burst(tx, ty, 5, p.color);
      }
    } else {
      p.x += (tx - p.x) / d * step;
      p.y += (ty - p.y) / d * step;
    }
  }
  G.projectiles = G.projectiles.filter(p => !p.done);

  // particles / beams / texts
  for (const p of G.particles) {
    p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 220 * dt;
  }
  G.particles = G.particles.filter(p => p.life > 0);
  for (const b of G.beams) b.life -= dt;
  G.beams = G.beams.filter(b => b.life > 0);
  for (const tx of G.texts) { tx.life -= dt; tx.y -= 22 * dt; }
  G.texts = G.texts.filter(tx => tx.life > 0);

  updateHUD();
}
function pickTargetExists(tw) { return !!pickTarget(tw); }

// ---- rendering ---------------------------------------------------------
function render() {
  const lv = G.levelDef;
  // terrain
  for (let gy = 0; gy < GRID_H; gy++)
    for (let gx = 0; gx < GRID_W; gx++)
      drawTileBg(ctx, lv.theme, gx, gy);

  // road
  drawRoad(lv.theme);

  // decorations
  for (const key of G.decoTiles) {
    const [gx, gy] = key.split(',').map(Number);
    drawDeco(ctx, lv.theme, gx, gy);
  }

  // placement hover / range
  if (placingType && hoverTile) {
    const ok = isBuildable(hoverTile.x, hoverTile.y);
    ctx.fillStyle = ok ? 'rgba(120,255,140,0.3)' : 'rgba(255,90,90,0.35)';
    ctx.fillRect(hoverTile.x * TILE, hoverTile.y * TILE, TILE, TILE);
    if (ok) {
      const r = TOWERS[placingType].tiers[0].range;
      drawRangeCircle(hoverTile.x * TILE + TILE / 2, hoverTile.y * TILE + TILE / 2, r);
    }
  }
  if (selectedTower) {
    drawRangeCircle(selectedTower.x, selectedTower.y, selectedTower.stats.range);
    ctx.strokeStyle = '#ffd34d';
    ctx.lineWidth = 3;
    ctx.strokeRect(selectedTower.gx * TILE + 2, selectedTower.gy * TILE + 2, TILE - 4, TILE - 4);
  }

  // sort entities by y for depth
  const ents = [...G.towers, ...G.enemies].sort((a, b) => a.y - b.y);
  for (const en of ents) {
    if (en.seg !== undefined) drawEnemy(ctx, en.type, en.x, en.y, en.size, en.anim, en.hp / en.maxHp);
    else drawTower(ctx, en);
  }

  // projectiles
  for (const p of G.projectiles) {
    ctx.fillStyle = p.color;
    if (p.type === 'cannon') {
      ctx.beginPath(); ctx.arc(p.x, p.y, 6, 0, Math.PI * 2); ctx.fill();
      ctx.fillStyle = '#2a2535';
      ctx.fillRect(p.x - 2, p.y - 2, 4, 4);
    } else {
      const a = Math.atan2(p.lastY - p.y, p.lastX - p.x);
      ctx.save(); ctx.translate(p.x, p.y); ctx.rotate(a);
      ctx.fillRect(-6, -2, 12, 4);
      ctx.fillStyle = '#fff'; ctx.fillRect(2, -1, 4, 2);
      ctx.restore();
    }
  }

  // beams
  for (const b of G.beams) {
    const a = Math.max(0, b.life / 0.2);
    if (b.aura) {
      ctx.strokeStyle = b.color;
      ctx.globalAlpha = a * 0.5;
      ctx.lineWidth = 3;
      ctx.beginPath(); ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2); ctx.stroke();
      ctx.globalAlpha = 1;
    } else {
      ctx.strokeStyle = b.color;
      ctx.globalAlpha = a;
      ctx.lineWidth = 4;
      ctx.beginPath(); ctx.moveTo(b.x1, b.y1); ctx.lineTo(b.x2, b.y2); ctx.stroke();
      ctx.strokeStyle = '#fff'; ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.globalAlpha = 1;
    }
  }

  // particles
  for (const p of G.particles) {
    ctx.globalAlpha = Math.max(0, p.life / p.maxLife);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
  }
  ctx.globalAlpha = 1;

  // floating texts
  ctx.textAlign = 'center';
  ctx.font = 'bold 16px monospace';
  for (const tx of G.texts) {
    ctx.globalAlpha = Math.min(1, tx.life * 1.6);
    ctx.fillStyle = '#000';
    ctx.fillText(tx.str, tx.x + 1, tx.y + 1);
    ctx.fillStyle = tx.color;
    ctx.fillText(tx.str, tx.x, tx.y);
  }
  ctx.globalAlpha = 1;

  // core marker at path end
  const end = G.path[G.path.length - 1];
  const cx = Math.max(20, Math.min(FIELD_W - 20, end.x));
  const cy = Math.max(20, Math.min(FIELD_H - 20, end.y));
  ctx.fillStyle = '#ffd34d';
  ctx.fillRect(cx - 12, cy - 12, 24, 24);
  ctx.fillStyle = G.lives > 5 ? '#3aa7ff' : '#ff5a5a';
  ctx.fillRect(cx - 8, cy - 8, 16, 16);
  ctx.fillStyle = '#fff';
  ctx.fillRect(cx - 4, cy - 6, 4, 4);

  // flash message
  if (flashTimer > 0) {
    ctx.globalAlpha = Math.min(1, flashTimer);
    ctx.fillStyle = 'rgba(20,16,30,0.85)';
    ctx.fillRect(FIELD_W / 2 - 130, 16, 260, 34);
    ctx.fillStyle = '#ff9c5f';
    ctx.font = 'bold 16px monospace';
    ctx.fillText(flashMsg, FIELD_W / 2, 38);
    ctx.globalAlpha = 1;
  }

  // wave countdown banner
  if (!G.waveActive && !G.over && !G.awaitingFirst) {
    ctx.fillStyle = 'rgba(20,16,30,0.7)';
    ctx.fillRect(FIELD_W / 2 - 90, FIELD_H - 44, 180, 30);
    ctx.fillStyle = '#9ceaff';
    ctx.font = 'bold 14px monospace';
    ctx.fillText(t('nextIn') + ' ' + Math.ceil(G.prepTimer) + 's', FIELD_W / 2, FIELD_H - 24);
  }
}

function drawRoad(theme) {
  const th = THEME[theme] || THEME.grass;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.strokeStyle = th.roadEdge;
  ctx.lineWidth = TILE - 4;
  strokePath();
  ctx.strokeStyle = th.road;
  ctx.lineWidth = TILE - 12;
  strokePath();
  // dashed center
  ctx.strokeStyle = 'rgba(255,255,255,0.25)';
  ctx.lineWidth = 3;
  ctx.setLineDash([8, 10]);
  strokePath();
  ctx.setLineDash([]);
}
function strokePath() {
  ctx.beginPath();
  ctx.moveTo(G.path[0].x, G.path[0].y);
  for (let i = 1; i < G.path.length; i++) ctx.lineTo(G.path[i].x, G.path[i].y);
  ctx.stroke();
}
function drawRangeCircle(x, y, r) {
  ctx.fillStyle = 'rgba(127,232,255,0.10)';
  ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = 'rgba(127,232,255,0.6)';
  ctx.lineWidth = 2;
  ctx.setLineDash([6, 6]);
  ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.stroke();
  ctx.setLineDash([]);
}

// ---- loop --------------------------------------------------------------
function loop(now) {
  rafId = requestAnimationFrame(loop);
  if (!G) return;
  let dt = (now - lastTime) / 1000;
  lastTime = now;
  if (dt > 0.1) dt = 0.1;
  if (!G.paused && !G.over) {
    for (let i = 0; i < G.speed; i++) update(dt);
  }
  render();
}

// ---- win / lose --------------------------------------------------------
function victory() {
  if (G.over) return;
  G.over = true; G.won = true;
  const stars = G.livesLost === 0 ? 3 : G.lives >= G.levelDef.startLives * 0.5 ? 2 : 1;
  const rec = save.levels[G.level] || { stars: 0 };
  if (stars > rec.stars) { rec.stars = stars; }
  save.levels[G.level] = rec;
  persist();
  showResult(true, stars);
}
function defeat() {
  if (G.over) return;
  G.over = true; G.won = false;
  showResult(false, 0);
}

// ---- HUD ---------------------------------------------------------------
function updateHUD() {
  if (!G) return;
  document.getElementById('hud-lives').textContent = '♥ ' + G.lives;
  document.getElementById('hud-gold').textContent = '◆ ' + G.gold;
  const total = G.levelDef.totalWaves;
  document.getElementById('hud-wave').textContent =
    (G.endless || G.waveIndex >= total)
      ? t('waveEndless', G.waveIndex + 1)
      : t('waveOf', Math.min(G.waveIndex + 1, total), total);
  // build card affordability
  document.querySelectorAll('.build-card').forEach(card => {
    const type = card.dataset.type;
    const afford = G.gold >= TOWERS[type].baseCost;
    card.classList.toggle('disabled', !afford);
    card.classList.toggle('selected', placingType === type);
  });
  const startBtn = document.getElementById('btn-start-wave');
  if (G.waveActive) {
    startBtn.classList.add('hidden');
  } else if (!G.over) {
    startBtn.classList.remove('hidden');
    startBtn.textContent = G.awaitingFirst ? t('startWave')
      : t('callNext').replace('%G', Math.ceil(G.prepTimer) * 3);
  }
}

// ---- build bar + tower panel ------------------------------------------
function buildBuildBar() {
  const bar = document.getElementById('build-cards');
  bar.innerHTML = '';
  for (const type of ['arrow', 'cannon', 'frost', 'arcane']) {
    const def = TOWERS[type];
    const card = document.createElement('button');
    card.className = 'build-card';
    card.dataset.type = type;
    const cv = document.createElement('canvas');
    cv.width = 48; cv.height = 48;
    const cc = cv.getContext('2d');
    cc.imageSmoothingEnabled = false;
    drawTowerIcon(cc, type, 24, 30, 1.05);
    card.appendChild(cv);
    const nm = document.createElement('span');
    nm.className = 'bc-name';
    nm.textContent = name(type);
    card.appendChild(nm);
    const co = document.createElement('span');
    co.className = 'bc-cost';
    co.textContent = '◆' + def.baseCost;
    card.appendChild(co);
    card.onclick = () => {
      placingType = placingType === type ? null : type;
      selectedTower = null;
      document.getElementById('tower-panel').classList.add('hidden');
      updateHUD();
    };
    bar.appendChild(card);
  }
}

function renderTowerPanel() {
  const panel = document.getElementById('tower-panel');
  if (!selectedTower) { panel.classList.add('hidden'); return; }
  const tw = selectedTower;
  const s = tw.stats;
  panel.classList.remove('hidden');
  const title = tw.tier === 3 ? name(tw.branch) : name(tw.type);
  let html = `<div class="tp-head"><span class="tp-title">${title}</span>` +
    `<span class="tp-tier">T${tw.tier}</span>` +
    `<button class="tp-close" id="tp-close">✕</button></div>`;
  html += `<div class="tp-stats">` +
    `<span>${t('dmg')} ${s.damage}</span>` +
    `<span>${t('rng')} ${Math.round(s.range)}</span>` +
    `<span>${t('spd')} ${(1 / s.interval).toFixed(2)}/s</span>` +
    (s.splash ? `<span>AoE ${s.splash}</span>` : '') +
    (s.slow ? `<span>Slow ${Math.round(s.slow * 100)}%</span>` : '') +
    (s.chain ? `<span>Chain ${s.chain}</span>` : '') +
    `</div>`;
  // upgrade buttons
  html += `<div class="tp-actions">`;
  if (tw.tier === 1) {
    const c = TOWERS[tw.type].tiers[1].cost;
    html += `<button class="tp-up" data-branch="" data-cost="${c}">${t('upgrade')} T2 ◆${c}</button>`;
  } else if (tw.tier === 2) {
    for (const bk in TOWERS[tw.type].branches) {
      const c = TOWERS[tw.type].branches[bk].cost;
      html += `<button class="tp-up" data-branch="${bk}" data-cost="${c}">${name(bk)} ◆${c}</button>`;
    }
  } else {
    html += `<span class="tp-max">${t('maxTier')}</span>`;
  }
  html += `</div>`;
  // target mode + sell
  const modes = ['first', 'last', 'strong', 'close'];
  const modeLabel = { first: t('targetFirst'), last: t('targetLast'), strong: t('targetStrong'), close: t('targetClose') };
  html += `<div class="tp-foot">` +
    `<button class="tp-target" id="tp-target">${t('target')}: ${modeLabel[tw.targetMode]}</button>` +
    `<button class="tp-sell" id="tp-sell">${t('sells', Math.floor(towerInvested(tw.type, tw.tier, tw.branch) * 0.7))}</button>` +
    `</div>`;
  panel.innerHTML = html;

  panel.querySelector('#tp-close').onclick = () => {
    selectedTower = null; panel.classList.add('hidden');
  };
  panel.querySelectorAll('.tp-up').forEach(btn => {
    btn.onclick = () => upgradeTower(tw, btn.dataset.branch || null);
  });
  const tgt = panel.querySelector('#tp-target');
  if (tgt) tgt.onclick = () => {
    const i = modes.indexOf(tw.targetMode);
    tw.targetMode = modes[(i + 1) % modes.length];
    renderTowerPanel();
  };
  panel.querySelector('#tp-sell').onclick = () => sellTower(tw);
}

// ---- input -------------------------------------------------------------
let canvasRect = null;
function updateRect() { canvasRect = canvas.getBoundingClientRect(); }

function pointerToField(clientX, clientY) {
  if (!canvasRect) updateRect();
  const x = (clientX - canvasRect.left) / canvasRect.width * FIELD_W;
  const y = (clientY - canvasRect.top) / canvasRect.height * FIELD_H;
  return { x, y };
}

function handleTap(clientX, clientY) {
  if (!G || G.over) return;
  const p = pointerToField(clientX, clientY);
  const gx = Math.floor(p.x / TILE), gy = Math.floor(p.y / TILE);
  // tap existing tower?
  const tw = G.towers.find(t => t.gx === gx && t.gy === gy);
  if (tw && !placingType) {
    selectedTower = tw;
    renderTowerPanel();
    return;
  }
  if (placingType) {
    if (placeTower(gx, gy, placingType)) {
      if (G.gold < TOWERS[placingType].baseCost) {
        placingType = null;
      }
      updateHUD();
    }
    return;
  }
  // tap empty -> deselect
  selectedTower = null;
  document.getElementById('tower-panel').classList.add('hidden');
}

canvas.addEventListener('pointerdown', e => {
  e.preventDefault();
  handleTap(e.clientX, e.clientY);
});
canvas.addEventListener('pointermove', e => {
  if (!G || !placingType) { hoverTile = null; return; }
  const p = pointerToField(e.clientX, e.clientY);
  hoverTile = { x: Math.floor(p.x / TILE), y: Math.floor(p.y / TILE) };
});
canvas.addEventListener('pointerleave', () => { hoverTile = null; });

// ---- responsive --------------------------------------------------------
function resize() {
  const stage = document.getElementById('stage');
  const sw = stage.clientWidth, sh = stage.clientHeight;
  const scale = Math.min(sw / FIELD_W, sh / FIELD_H);
  canvas.style.width = Math.floor(FIELD_W * scale) + 'px';
  canvas.style.height = Math.floor(FIELD_H * scale) + 'px';
  updateRect();
}
window.addEventListener('resize', () => { resize(); });
window.addEventListener('orientationchange', () => setTimeout(resize, 200));

// ---- overlays: pause / result -----------------------------------------
function showPause(show) {
  if (!G) return;
  G.paused = show;
  document.getElementById('overlay-pause').classList.toggle('hidden', !show);
}
function showResult(won, stars) {
  const o = document.getElementById('overlay-result');
  o.classList.remove('hidden');
  document.getElementById('result-title').textContent = won ? t('victory') : t('defeat');
  document.getElementById('result-title').className = won ? 'win' : 'lose';
  document.getElementById('result-sub').textContent = won ? t('levelClear') : t('coreLost');
  // stars
  const sc = document.getElementById('result-stars');
  sc.innerHTML = '';
  if (won) {
    for (let i = 0; i < 3; i++) {
      const cv = document.createElement('canvas');
      cv.width = 56; cv.height = 56;
      drawStar(cv.getContext('2d'), 28, 30, 22, i < stars);
      sc.appendChild(cv);
    }
  }
  document.getElementById('result-info').textContent =
    `${t('wave')}: ${G.waveIndex}   ◆ ${G.gold}   ♥ ${G.lives}`;
  // buttons
  const nextBtn = document.getElementById('btn-result-next');
  const hasNext = G.level + 1 < LEVELS.length;
  if (won && hasNext) {
    nextBtn.classList.remove('hidden');
    nextBtn.textContent = t('nextLevel');
  } else if (won && !hasNext) {
    nextBtn.classList.remove('hidden');
    nextBtn.textContent = t('endless');
  } else {
    nextBtn.classList.add('hidden');
  }
  document.getElementById('btn-result-retry').textContent = t('retry');
  document.getElementById('btn-result-menu').textContent = t('menu');
}
function hideResult() { document.getElementById('overlay-result').classList.add('hidden'); }

// ---- level select ------------------------------------------------------
function buildLevelSelect() {
  const list = document.getElementById('level-list');
  list.innerHTML = '';
  LEVELS.forEach((lv, i) => {
    const unlocked = i === 0 || (save.levels[i - 1] && save.levels[i - 1].stars > 0);
    const rec = save.levels[i];
    const card = document.createElement('button');
    card.className = 'level-card' + (unlocked ? '' : ' locked');
    card.innerHTML =
      `<div class="lc-name">${lv.name[currentLang] || lv.name.en}</div>` +
      `<div class="lc-theme lc-${lv.theme}"></div>` +
      `<div class="lc-stars"></div>`;
    const stars = card.querySelector('.lc-stars');
    if (unlocked) {
      for (let s = 0; s < 3; s++) {
        const cv = document.createElement('canvas');
        cv.width = 26; cv.height = 26;
        drawStar(cv.getContext('2d'), 13, 14, 10, rec && s < rec.stars);
        stars.appendChild(cv);
      }
    } else {
      stars.textContent = '🔒 ' + t('locked');
    }
    if (unlocked) card.onclick = () => startLevel(i);
    list.appendChild(card);
  });
}

function startLevel(i) {
  newGame(i, false);
  showScreen('game');
  document.getElementById('hud-level').textContent = LEVELS[i].name[currentLang] || LEVELS[i].name.en;
  buildBuildBar();
  document.getElementById('tower-panel').classList.add('hidden');
  hideResult();
  showPause(false);
  setSpeed(1);
  updateHUD();
  resize();
  flash(t('tutorial'));
}

function setSpeed(s) {
  if (G) G.speed = s;
  document.querySelectorAll('.speed-btn').forEach(b => {
    b.classList.toggle('active', Number(b.dataset.speed) === s);
  });
}

// ---- wire up UI --------------------------------------------------------
function bindUI() {
  document.getElementById('btn-play').onclick = () => {
    buildLevelSelect();
    showScreen('select');
  };
  document.getElementById('btn-select-back').onclick = () => showScreen('title');
  document.getElementById('btn-start-wave').onclick = () => startWave();
  document.getElementById('btn-pause').onclick = () => showPause(true);
  document.getElementById('btn-resume').onclick = () => showPause(false);
  document.getElementById('btn-pause-restart').onclick = () => {
    showPause(false); startLevel(G.level);
  };
  document.getElementById('btn-pause-quit').onclick = () => {
    showPause(false); buildLevelSelect(); showScreen('select');
  };
  document.querySelectorAll('.speed-btn').forEach(b => {
    b.onclick = () => setSpeed(Number(b.dataset.speed));
  });
  document.getElementById('btn-result-retry').onclick = () => {
    hideResult(); startLevel(G.level);
  };
  document.getElementById('btn-result-menu').onclick = () => {
    hideResult(); buildLevelSelect(); showScreen('select');
  };
  document.getElementById('btn-result-next').onclick = () => {
    hideResult();
    if (G.won && G.level + 1 < LEVELS.length && !G.endless) {
      startLevel(G.level + 1);
    } else {
      // continue endless on same level
      G.endless = true; G.over = false; G.won = false;
      G.prepTimer = G.prepMax;
      updateHUD();
    }
  };
  setupLanguageToggle(() => {
    if (!screens.select.classList.contains('hidden')) buildLevelSelect();
    if (!screens.game.classList.contains('hidden')) {
      buildBuildBar();
      renderTowerPanel();
      updateHUD();
      document.getElementById('hud-level').textContent =
        LEVELS[G.level].name[currentLang] || LEVELS[G.level].name.en;
    }
  });
}

// ---- boot --------------------------------------------------------------
bindUI();
applyStaticText();
showScreen('title');
lastTime = performance.now();
rafId = requestAnimationFrame(loop);
window.addEventListener('load', resize);
setTimeout(resize, 60);

})();
