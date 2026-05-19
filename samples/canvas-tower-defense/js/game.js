const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const W = 960;
const H = 540;
const BEST_KEY = 'canvas-tower-defense-best';

const path = [
  { x: 24, y: 318 },
  { x: 166, y: 318 },
  { x: 166, y: 150 },
  { x: 382, y: 150 },
  { x: 382, y: 384 },
  { x: 646, y: 384 },
  { x: 646, y: 226 },
  { x: 936, y: 226 },
];

const pads = [
  { x: 105, y: 216 }, { x: 246, y: 264 }, { x: 304, y: 86 },
  { x: 488, y: 224 }, { x: 538, y: 452 }, { x: 722, y: 324 },
  { x: 776, y: 148 }, { x: 838, y: 316 }, { x: 270, y: 420 },
];

const TOWERS = {
  arrow: {
    name: 'Arrow',
    cost: 55,
    range: 126,
    damage: 18,
    cooldown: 34,
    color: '#7fe8ff',
    desc: 'Fast single-target tower',
  },
  cannon: {
    name: 'Cannon',
    cost: 85,
    range: 108,
    damage: 38,
    cooldown: 64,
    splash: 46,
    color: '#f4c85a',
    desc: 'Slow splash damage',
  },
  frost: {
    name: 'Frost',
    cost: 75,
    range: 118,
    damage: 9,
    cooldown: 46,
    slow: 0.46,
    slowTime: 95,
    color: '#9ceaff',
    desc: 'Slows enemies in range',
  },
};

const ENEMIES = {
  runner: { name: 'Runner', hp: 42, speed: 1.55, reward: 7, color: '#ff9c5f', size: 20 },
  brute: { name: 'Brute', hp: 105, speed: 0.82, reward: 14, color: '#e85d75', size: 28 },
  swarm: { name: 'Swarm', hp: 25, speed: 1.24, reward: 5, color: '#b77cff', size: 16 },
};

const state = {
  gold: 150,
  lives: 20,
  wave: 1,
  score: 0,
  mode: 'build',
  selectedType: 'arrow',
  selectedTower: null,
  selectedPad: null,
  waveActive: false,
  spawning: false,
  spawnQueue: [],
  spawnTimer: 0,
  enemies: [],
  towers: [],
  shots: [],
  particles: [],
  messages: [t('introMsg')],
  best: +(localStorage.getItem(BEST_KEY) || 0),
  over: false,
  shake: 0,
  frame: 0,
};

function wavePlan(wave) {
  const queue = [];
  const runners = 7 + wave * 2;
  const brutes = Math.floor(wave / 2);
  const swarms = wave >= 3 ? 5 + wave : 0;
  for (let i = 0; i < runners; i++) queue.push('runner');
  for (let i = 0; i < brutes; i++) queue.push('brute');
  for (let i = 0; i < swarms; i++) queue.push('swarm');
  return queue.sort(() => Math.random() - 0.5);
}

function enemyStats(type) {
  const base = ENEMIES[type];
  const hpMul = 1 + (state.wave - 1) * 0.22;
  return {
    ...base,
    hp: Math.round(base.hp * hpMul),
    reward: base.reward + Math.floor(state.wave * 0.9),
  };
}

function log(text) {
  state.messages.unshift(text);
  state.messages.length = Math.min(state.messages.length, 4);
  updateLog();
}

function updateLog() {
  const el = document.getElementById('log');
  if (el) el.innerHTML = state.messages.map(m => `<div>${m}</div>`).join('');
}

function startWave() {
  if (state.waveActive || state.enemies.length || state.lives <= 0) return;
  state.waveActive = true;
  state.spawning = true;
  state.spawnQueue = wavePlan(state.wave);
  state.spawnTimer = 0;
  state.selectedTower = null;
  state.selectedPad = null;
  log(t('waveIncoming', state.wave, state.spawnQueue.length));
}

function spawnEnemy(type) {
  const stats = enemyStats(type);
  state.enemies.push({
    ...stats,
    type,
    x: path[0].x,
    y: path[0].y,
    node: 1,
    maxHp: stats.hp,
    slowTimer: 0,
    slowMul: 1,
    burn: 0,
    wobble: Math.random() * Math.PI * 2,
    progress: 0,
  });
}

function buildCost(type) {
  return TOWERS[type].cost;
}

function upgradeCost(tower) {
  return Math.round(TOWERS[tower.type].cost * (0.75 + tower.level * 0.7));
}

function sellValue(tower) {
  return Math.floor(tower.spent * 0.65);
}

function towerAtPad(pad) {
  return state.towers.find(t => t.pad === pad);
}

function nearestPad(x, y) {
  let best = null;
  let dist = Infinity;
  for (const pad of pads) {
    const d = Math.hypot(x - pad.x, y - pad.y);
    if (d < dist) {
      best = pad;
      dist = d;
    }
  }
  return dist <= 42 ? best : null;
}

function handleCanvasTap(x, y) {
  const pad = nearestPad(x, y);
  if (!pad) {
    state.selectedPad = null;
    state.selectedTower = null;
    return;
  }
  const tower = towerAtPad(pad);
  state.selectedPad = pad;
  state.selectedTower = tower || null;
  if (!tower) buildTower(pad, state.selectedType);
}

function buildTower(pad, type) {
  if (towerAtPad(pad)) return;
  const def = TOWERS[type];
  if (state.gold < def.cost) {
    log(t('needGoldFor', def.cost, towerName(type)));
    pulse(pad.x, pad.y, '#e85d75', 8);
    return;
  }
  state.gold -= def.cost;
  const tower = {
    pad,
    type,
    level: 1,
    cooldown: 0,
    angle: -Math.PI / 2,
    spent: def.cost,
    kills: 0,
  };
  state.towers.push(tower);
  state.selectedTower = tower;
  log(t('towerBuilt', towerName(type)));
  pulse(pad.x, pad.y, def.color, 12);
}

function upgradeSelected() {
  const tower = state.selectedTower;
  if (!tower) return;
  const cost = upgradeCost(tower);
  if (state.gold < cost) {
    log(t('needGoldUpgrade', cost));
    return;
  }
  state.gold -= cost;
  tower.level++;
  tower.spent += cost;
  pulse(tower.pad.x, tower.pad.y, TOWERS[tower.type].color, 18);
  log(t('towerUpgraded', towerName(tower.type), tower.level));
}

function sellSelected() {
  const tower = state.selectedTower;
  if (!tower) return;
  state.gold += sellValue(tower);
  state.towers = state.towers.filter(t => t !== tower);
  state.selectedTower = null;
  log(t('towerSold'));
}

function towerStat(tower, key) {
  const def = TOWERS[tower.type];
  if (key === 'range') return def.range + (tower.level - 1) * 15;
  if (key === 'damage') return Math.round(def.damage * (1 + (tower.level - 1) * 0.42));
  if (key === 'cooldown') return Math.max(14, def.cooldown - (tower.level - 1) * 5);
  if (key === 'splash') return def.splash ? def.splash + (tower.level - 1) * 8 : 0;
  return def[key];
}

function acquireTarget(tower) {
  const range = towerStat(tower, 'range');
  let best = null;
  let bestProgress = -Infinity;
  for (const e of state.enemies) {
    if (e.done || e.dead || e.hp <= 0) continue;
    const d = Math.hypot(e.x - tower.pad.x, e.y - tower.pad.y);
    if (d <= range && e.progress > bestProgress) {
      best = e;
      bestProgress = e.progress;
    }
  }
  return best;
}

function fireTower(tower, target) {
  const def = TOWERS[tower.type];
  const damage = towerStat(tower, 'damage');
  const dx = target.x - tower.pad.x;
  const dy = target.y - tower.pad.y;
  tower.angle = Math.atan2(dy, dx);
  tower.cooldown = towerStat(tower, 'cooldown');

  if (tower.type === 'cannon') {
    state.shots.push({ kind: 'shell', x: tower.pad.x, y: tower.pad.y - 10, tx: target.x, ty: target.y, life: 18, maxLife: 18, damage, splash: towerStat(tower, 'splash'), color: def.color, tower });
  } else {
    hitEnemy(target, damage, tower);
    if (tower.type === 'frost') {
      target.slowMul = Math.min(target.slowMul, def.slow);
      target.slowTimer = def.slowTime + tower.level * 15;
    }
    state.shots.push({ kind: 'beam', x: tower.pad.x, y: tower.pad.y - 12, tx: target.x, ty: target.y, life: 8, maxLife: 8, color: def.color });
  }
}

function hitEnemy(enemy, damage, tower) {
  enemy.hp -= damage;
  pulse(enemy.x, enemy.y, TOWERS[tower.type].color, 5);
  if (enemy.hp <= 0 && !enemy.dead) {
    enemy.dead = true;
    tower.kills++;
    state.gold += enemy.reward;
    state.score += enemy.reward * 5;
    pulse(enemy.x, enemy.y, enemy.color, 16);
  }
}

function splashDamage(x, y, radius, damage, tower) {
  for (const e of state.enemies) {
    if (e.dead || e.done) continue;
    const d = Math.hypot(e.x - x, e.y - y);
    if (d <= radius) hitEnemy(e, Math.round(damage * (1 - d / radius * 0.45)), tower);
  }
  state.shake = Math.max(state.shake, 5);
  pulse(x, y, '#f4c85a', 24);
}

function pulse(x, y, color, count) {
  for (let i = 0; i < count; i++) {
    state.particles.push({
      x,
      y,
      vx: (Math.random() - 0.5) * 3.8,
      vy: (Math.random() - 0.75) * 3.8,
      life: 24 + Math.random() * 16,
      color,
      size: 2 + Math.random() * 3,
    });
  }
}

function update() {
  state.frame++;
  if (state.spawning) {
    state.spawnTimer--;
    if (state.spawnTimer <= 0 && state.spawnQueue.length) {
      spawnEnemy(state.spawnQueue.shift());
      state.spawnTimer = Math.max(18, 48 - state.wave * 1.5);
    }
    if (!state.spawnQueue.length) state.spawning = false;
  }

  for (const e of state.enemies) {
    if (e.slowTimer > 0) e.slowTimer--;
    else e.slowMul = 1;
    const target = path[e.node];
    const dx = target.x - e.x;
    const dy = target.y - e.y;
    const d = Math.hypot(dx, dy);
    const speed = e.speed * e.slowMul;
    if (d < speed) {
      e.x = target.x;
      e.y = target.y;
      e.node++;
      if (e.node >= path.length) {
        e.done = true;
        state.lives--;
        state.shake = Math.max(state.shake, 7);
        pulse(e.x, e.y, '#e85d75', 18);
      }
    } else {
      e.x += dx / d * speed;
      e.y += dy / d * speed;
    }
    e.progress = e.node * 10000 - d;
    e.wobble += 0.15;
  }

  for (const tower of state.towers) {
    tower.cooldown--;
    if (tower.cooldown > 0) continue;
    const target = acquireTarget(tower);
    if (target) fireTower(tower, target);
  }

  for (const shot of state.shots) {
    shot.life--;
    if (shot.kind === 'shell' && shot.life <= 0 && !shot.done) {
      shot.done = true;
      splashDamage(shot.tx, shot.ty, shot.splash, shot.damage, shot.tower);
    }
  }
  state.shots = state.shots.filter(s => s.life > 0 && !s.done);

  state.enemies = state.enemies.filter(e => !e.done && !e.dead);
  for (const p of state.particles) {
    p.x += p.vx;
    p.y += p.vy;
    p.vy += 0.04;
    p.life--;
  }
  state.particles = state.particles.filter(p => p.life > 0);
  state.shake *= 0.86;

  if (state.waveActive && !state.spawning && !state.enemies.length && !state.spawnQueue.length) {
    state.waveActive = false;
    state.wave++;
    const bonus = 28 + state.wave * 4;
    state.gold += bonus;
    log(t('waveCleared', bonus));
  }
  updateHud();
}

function drawTerrain() {
  ctx.fillStyle = '#0f171f';
  ctx.fillRect(0, 0, W, H);

  const tile = 32;
  for (let y = 0; y < H; y += tile) {
    for (let x = 0; x < W; x += tile) {
      const n = (x * 17 + y * 31) % 7;
      ctx.fillStyle = n < 2 ? '#15251d' : n < 4 ? '#13211a' : '#101d18';
      ctx.fillRect(x, y, tile, tile);
      if (n === 0) {
        ctx.fillStyle = '#20382a';
        ctx.fillRect(x + 7, y + 9, 8, 5);
        ctx.fillRect(x + 19, y + 21, 5, 4);
      }
    }
  }

  ctx.fillStyle = 'rgba(70, 154, 112, 0.12)';
  ctx.fillRect(0, 74, W, 4);
  ctx.fillRect(0, H - 64, W, 4);
}

function drawPath() {
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.strokeStyle = '#2a201a';
  ctx.lineWidth = 66;
  ctx.beginPath();
  ctx.moveTo(path[0].x, path[0].y);
  for (let i = 1; i < path.length; i++) ctx.lineTo(path[i].x, path[i].y);
  ctx.stroke();

  ctx.strokeStyle = '#72583f';
  ctx.lineWidth = 54;
  ctx.stroke();
  ctx.strokeStyle = '#b2895b';
  ctx.lineWidth = 38;
  ctx.stroke();

  ctx.setLineDash([14, 16]);
  ctx.strokeStyle = 'rgba(255, 232, 170, 0.18)';
  ctx.lineWidth = 4;
  ctx.stroke();
  ctx.setLineDash([]);
}

function drawGate() {
  drawPixelPanel(4, 284, 52, 68, '#274a38', '#68da86');
  ctx.fillStyle = '#071018';
  ctx.font = '10px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(t('rift'), 30, 323);
  drawPixelPanel(904, 184, 52, 86, '#562632', '#e85d75');
  ctx.fillStyle = '#071018';
  ctx.fillText(t('core'), 930, 231);
}

function drawPixelPanel(x, y, w, h, body, trim) {
  ctx.fillStyle = '#05080d';
  ctx.fillRect(x + 4, y + 5, w, h);
  ctx.fillStyle = body;
  ctx.fillRect(x, y, w, h);
  ctx.fillStyle = trim;
  ctx.fillRect(x + 5, y + 5, w - 10, 6);
  ctx.fillRect(x + 5, y + h - 11, w - 10, 6);
}

function drawPads() {
  for (const pad of pads) {
    const tower = towerAtPad(pad);
    const selected = state.selectedPad === pad;
    ctx.fillStyle = selected ? '#f4c85a' : tower ? '#2a3648' : '#243225';
    ctx.fillRect(pad.x - 28, pad.y - 28, 56, 56);
    ctx.fillStyle = tower ? '#111827' : '#304332';
    ctx.fillRect(pad.x - 22, pad.y - 22, 44, 44);
    ctx.strokeStyle = selected ? '#fff1a5' : 'rgba(230,240,255,0.22)';
    ctx.lineWidth = 2;
    ctx.strokeRect(pad.x - 28, pad.y - 28, 56, 56);
    if (tower) drawTower(tower);
    else {
      ctx.fillStyle = 'rgba(255,255,255,0.15)';
      ctx.fillRect(pad.x - 10, pad.y - 2, 20, 4);
      ctx.fillRect(pad.x - 2, pad.y - 10, 4, 20);
    }
  }
}

function drawTower(tower) {
  const { pad } = tower;
  const def = TOWERS[tower.type];
  ctx.save();
  ctx.translate(pad.x, pad.y);
  ctx.fillStyle = '#0a0e15';
  ctx.fillRect(-18, 14, 36, 8);
  ctx.fillStyle = '#263448';
  ctx.fillRect(-17, -17, 34, 34);
  ctx.fillStyle = def.color;
  ctx.fillRect(-12, -12, 24, 8);
  ctx.rotate(tower.angle);
  if (tower.type === 'cannon') {
    ctx.fillStyle = '#1c2432';
    ctx.fillRect(-8, -8, 24, 16);
    ctx.fillStyle = def.color;
    ctx.fillRect(8, -5, 22, 10);
  } else if (tower.type === 'frost') {
    ctx.fillStyle = def.color;
    ctx.fillRect(0, -4, 28, 8);
    ctx.fillRect(18, -10, 8, 20);
  } else {
    ctx.fillStyle = def.color;
    ctx.fillRect(0, -3, 30, 6);
    ctx.fillRect(20, -8, 8, 16);
  }
  ctx.restore();
  ctx.fillStyle = '#edf4ff';
  ctx.font = '11px monospace';
  ctx.textAlign = 'center';
  ctx.fillText(`L${tower.level}`, pad.x, pad.y + 39);
}

function drawSelection() {
  const tower = state.selectedTower;
  if (!tower) return;
  ctx.strokeStyle = 'rgba(127,232,255,0.24)';
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.arc(tower.pad.x, tower.pad.y, towerStat(tower, 'range'), 0, Math.PI * 2);
  ctx.stroke();
}

function drawEnemies() {
  for (const e of state.enemies) {
    const bob = Math.sin(e.wobble) * 2;
    ctx.fillStyle = 'rgba(0,0,0,0.28)';
    ctx.fillRect(e.x - e.size * 0.62, e.y + e.size * 0.42, e.size * 1.2, 7);

    if (e.type === 'runner') drawRunner(e.x, e.y + bob, e.color);
    else if (e.type === 'brute') drawBrute(e.x, e.y + bob, e.color);
    else drawSwarm(e.x, e.y + bob, e.color);

    if (e.slowTimer > 0) {
      ctx.strokeStyle = 'rgba(156,234,255,0.65)';
      ctx.lineWidth = 2;
      ctx.strokeRect(e.x - e.size / 2 - 4, e.y - e.size / 2 - 4, e.size + 8, e.size + 8);
    }

    ctx.fillStyle = '#05080d';
    ctx.fillRect(e.x - 20, e.y - e.size / 2 - 15, 40, 5);
    ctx.fillStyle = e.hp / e.maxHp > 0.45 ? '#68da86' : '#e85d75';
    ctx.fillRect(e.x - 20, e.y - e.size / 2 - 15, 40 * Math.max(0, e.hp / e.maxHp), 5);
  }
}

function drawRunner(x, y, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x - 9, y - 11, 18, 22);
  ctx.fillStyle = '#ffd6c7';
  ctx.fillRect(x - 5, y - 6, 4, 4);
  ctx.fillRect(x + 2, y - 6, 4, 4);
  ctx.fillStyle = '#5a2230';
  ctx.fillRect(x - 12, y + 7, 7, 7);
  ctx.fillRect(x + 5, y + 7, 7, 7);
}

function drawBrute(x, y, color) {
  ctx.fillStyle = '#4b1723';
  ctx.fillRect(x - 17, y - 15, 34, 30);
  ctx.fillStyle = color;
  ctx.fillRect(x - 13, y - 20, 26, 28);
  ctx.fillStyle = '#ffd6c7';
  ctx.fillRect(x - 7, y - 11, 5, 5);
  ctx.fillRect(x + 3, y - 11, 5, 5);
}

function drawSwarm(x, y, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x - 8, y - 8, 16, 16);
  ctx.fillStyle = '#ead8ff';
  ctx.fillRect(x - 4, y - 4, 3, 3);
  ctx.fillRect(x + 2, y - 4, 3, 3);
}

function drawShots() {
  for (const s of state.shots) {
    const a = Math.max(0, s.life / s.maxLife);
    ctx.globalAlpha = a;
    if (s.kind === 'beam') {
      ctx.strokeStyle = s.color;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(s.x, s.y);
      ctx.lineTo(s.tx, s.ty);
      ctx.stroke();
    } else {
      const t = 1 - s.life / s.maxLife;
      const x = s.x + (s.tx - s.x) * t;
      const y = s.y + (s.ty - s.y) * t - Math.sin(t * Math.PI) * 34;
      ctx.fillStyle = s.color;
      ctx.fillRect(x - 5, y - 5, 10, 10);
    }
    ctx.globalAlpha = 1;
  }
}

function drawParticles() {
  for (const p of state.particles) {
    ctx.globalAlpha = Math.max(0, p.life / 32);
    ctx.fillStyle = p.color;
    ctx.fillRect(p.x, p.y, p.size, p.size);
    ctx.globalAlpha = 1;
  }
}

function drawBuildBar() {
  const x = 14;
  const y = H - 58;
  const entries = Object.entries(TOWERS);
  entries.forEach(([id, def], i) => {
    const bx = x + i * 122;
    const active = state.selectedType === id;
    ctx.fillStyle = active ? def.color : 'rgba(7,10,16,0.82)';
    ctx.fillRect(bx, y, 112, 44);
    ctx.strokeStyle = active ? '#ffffff' : 'rgba(230,240,255,0.22)';
    ctx.strokeRect(bx, y, 112, 44);
    ctx.fillStyle = active ? '#071018' : def.color;
    ctx.font = '12px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(towerName(id), bx + 8, y + 16);
    ctx.fillText(`${def.cost}g`, bx + 8, y + 32);
  });
}

function drawInfoPanel() {
  const x = W - 245;
  const y = H - 74;
  ctx.fillStyle = 'rgba(7,10,16,0.84)';
  ctx.fillRect(x, y, 232, 60);
  ctx.strokeStyle = 'rgba(230,240,255,0.22)';
  ctx.strokeRect(x, y, 232, 60);
  ctx.fillStyle = '#9aacbf';
  ctx.font = '11px monospace';
  ctx.textAlign = 'left';
  if (state.selectedTower) {
    const tw = state.selectedTower;
    const def = TOWERS[tw.type];
    ctx.fillStyle = def.color;
    ctx.fillText(`${towerName(tw.type)} Lv.${tw.level}`, x + 10, y + 17);
    ctx.fillStyle = '#9aacbf';
    ctx.fillText(t('statLine', towerStat(tw, 'damage'), towerStat(tw, 'range')), x + 10, y + 34);
    ctx.fillText(t('actionLine', upgradeCost(tw), sellValue(tw)), x + 10, y + 51);
  } else {
    const def = TOWERS[state.selectedType];
    ctx.fillStyle = def.color;
    ctx.fillText(t('towerHeading', towerName(state.selectedType)), x + 10, y + 17);
    ctx.fillStyle = '#9aacbf';
    ctx.fillText(towerDesc(state.selectedType), x + 10, y + 34);
    ctx.fillText(t('buildHint'), x + 10, y + 51);
  }
}

function drawHudText() {
  ctx.fillStyle = 'rgba(7,10,16,0.74)';
  ctx.fillRect(14, 82, 270, 28);
  ctx.strokeStyle = 'rgba(230,240,255,0.18)';
  ctx.strokeRect(14, 82, 270, 28);
  ctx.fillStyle = '#9aacbf';
  ctx.font = '12px monospace';
  ctx.textAlign = 'left';
  const next = wavePlan(state.wave);
  const preview = state.waveActive
    ? t('remaining', state.enemies.length + state.spawnQueue.length)
    : t('enemiesNext', next.length);
  ctx.fillText(preview, 24, 101);
}

function draw() {
  const sx = state.shake ? (Math.random() - 0.5) * state.shake : 0;
  const sy = state.shake ? (Math.random() - 0.5) * state.shake : 0;
  ctx.save();
  ctx.translate(sx, sy);
  drawTerrain();
  drawPath();
  drawGate();
  drawPads();
  drawSelection();
  drawEnemies();
  drawShots();
  drawParticles();
  drawBuildBar();
  drawInfoPanel();
  drawHudText();
  ctx.restore();

  if (state.lives <= 0) {
    ctx.fillStyle = 'rgba(0,0,0,0.68)';
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = '#f4c85a';
    ctx.font = '44px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(t('coreDestroyed'), W / 2, H / 2 - 10);
    ctx.fillStyle = '#9aacbf';
    ctx.font = '16px monospace';
    ctx.fillText(t('scoreLine', state.score), W / 2, H / 2 + 24);
  }
}

function updateHud() {
  if (state.lives <= 0 && !state.over) {
    state.over = true;
    if (state.score > state.best) {
      state.best = state.score;
      try { localStorage.setItem(BEST_KEY, state.best); } catch (e) { /* storage off */ }
    }
  }
  document.getElementById('gold').textContent = state.gold;
  document.getElementById('lives').textContent = state.lives;
  document.getElementById('wave').textContent = state.wave;
  document.getElementById('score').textContent = state.score;
  document.getElementById('best').textContent = Math.max(state.best, state.score);
  document.getElementById('tower-type').textContent = towerName(state.selectedType);
  const btn = document.getElementById('start');
  btn.disabled = state.waveActive || state.enemies.length > 0 || state.lives <= 0;
}

function canvasPoint(e) {
  const rect = canvas.getBoundingClientRect();
  return { x: ((e.clientX - rect.left) / rect.width) * W, y: ((e.clientY - rect.top) / rect.height) * H };
}

function handleCanvasPointer(e) {
  const p = canvasPoint(e);
  if (p.y >= H - 66 && p.x < 385) {
    const idx = Math.floor((p.x - 14) / 122);
    const ids = Object.keys(TOWERS);
    if (ids[idx]) {
      state.selectedType = ids[idx];
      state.selectedTower = null;
      state.selectedPad = null;
      return;
    }
  }
  if (p.x > W - 250 && p.y > H - 82 && state.selectedTower) {
    if (p.y > H - 44) sellSelected();
    else upgradeSelected();
    return;
  }
  handleCanvasTap(p.x, p.y);
}

canvas.addEventListener('pointerdown', handleCanvasPointer);
document.getElementById('start').onclick = startWave;
document.getElementById('upgrade').onclick = upgradeSelected;
document.getElementById('sell').onclick = sellSelected;
setupLanguageToggle(updateHud);
document.querySelectorAll('[data-tower]').forEach(btn => {
  btn.onclick = () => {
    state.selectedType = btn.dataset.tower;
    state.selectedTower = null;
    state.selectedPad = null;
    document.querySelectorAll('[data-tower]').forEach(b => b.classList.toggle('active', b === btn));
    updateHud();
  };
});

document.addEventListener('keydown', e => {
  if (e.key === ' ') {
    e.preventDefault();
    startWave();
  }
  if (e.key === '1') state.selectedType = 'arrow';
  if (e.key === '2') state.selectedType = 'cannon';
  if (e.key === '3') state.selectedType = 'frost';
  if (e.key.toLowerCase() === 'u') upgradeSelected();
  if (e.key.toLowerCase() === 's') sellSelected();
  document.querySelectorAll('[data-tower]').forEach(b => b.classList.toggle('active', b.dataset.tower === state.selectedType));
  updateHud();
});

function loop() {
  update();
  draw();
  requestAnimationFrame(loop);
}

updateHud();
updateLog();
loop();
