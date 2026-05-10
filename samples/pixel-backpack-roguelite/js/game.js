const canvas = document.getElementById('battleCanvas');
const ctx = canvas.getContext('2d');
const backpackEl = document.getElementById('backpack');
const shopGrid = document.getElementById('shopGrid');
const upgradeChoices = document.getElementById('upgradeChoices');
const logEl = document.getElementById('combatLog');

const GAP = 2;

const TEXT = {
  en: {
    title: 'Infinite Backpack',
    labels: { wave: 'Wave', life: 'Life', gold: 'Gold', runes: 'Runes' },
    packTitle: 'Backpack Grid',
    shop: 'Shop',
    upgrades: 'Post-Wave Upgrade',
    phases: { prep: 'Pack phase', combat: 'Auto combat', reward: 'Choose reward', gameover: 'Gate broken' },
    buttons: { rotate: 'Rotate', merge: 'Merge', forge: 'Forge', sell: 'Sell', reroll: 'Reroll', expand: 'Expand', start: 'Start Wave', restart: 'Restart' },
    selectedEmpty: 'Select an item to inspect it. Gems, gears, and batteries empower adjacent weapons.',
    adjacent: 'Adjacent',
    none: 'None',
    sellFor: 'Sell',
    gold: 'gold',
    current: 'Current',
    prepRound: wave => `Preparing wave ${wave}`,
    combatRound: wave => `Wave ${wave} in progress`,
    bought: name => `Bought ${name}.`,
    noSpace: 'No free backpack space.',
    needGold: amount => `Need ${amount} more gold.`,
    rerolled: 'Shop rerolled.',
    maxPack: 'Backpack is already at max size.',
    expandNeed: cost => `Expansion costs ${cost} gold.`,
    expanded: 'Backpack expanded.',
    merged: (name, tier) => `${name} merged into T${tier}.`,
    noMerge: 'No matching same-tier items can merge.',
    selectForge: 'Select an item with a forge partner.',
    forgeNoSpace: 'Forged item does not fit.',
    forged: name => `Forged ${name}.`,
    noForge: 'No adjacent forge recipe found.',
    rotateBlocked: 'Rotation would collide or leave the grid.',
    sold: name => `Sold ${name}.`,
    upgraded: name => `Upgrade: ${name}. Repack for the next wave.`,
    incoming: (wave, count) => `Wave ${wave} incoming: ${count} enemies.`,
    broken: wave => `The gate broke on wave ${wave}.`,
    victory: gold => `Victory. Earned ${gold} gold. Choose one upgrade.`,
    intro: 'Buy items, arrange the backpack, then start the wave.',
    canvasPrep: 'Pack your build, then start the wave',
    canvasReward: 'Victory: choose an upgrade',
    langButton: '中文',
    coin: 'Gold',
  },
  zh: {
    title: '无限背包',
    labels: { wave: '波次', life: '生命', gold: '金币', runes: '符文' },
    packTitle: '背包矩阵',
    shop: '商店',
    upgrades: '战后升级',
    phases: { prep: '整理背包', combat: '自动战斗', reward: '选择奖励', gameover: '防线崩溃' },
    buttons: { rotate: '旋转', merge: '合成', forge: '锻造', sell: '出售', reroll: '刷新', expand: '扩容', start: '开始战斗', restart: '重新开始' },
    selectedEmpty: '选择道具查看效果。宝石、齿轮、电池会强化相邻武器。',
    adjacent: '相邻',
    none: '无',
    sellFor: '出售',
    gold: '金币',
    current: '当前',
    prepRound: wave => `准备第 ${wave} 波`,
    combatRound: wave => `第 ${wave} 波进行中`,
    bought: name => `买入 ${name}。`,
    noSpace: '背包没有可用空间。',
    needGold: amount => `金币不足，还需要 ${amount}。`,
    rerolled: '商店已刷新。',
    maxPack: '背包已经达到最大尺寸。',
    expandNeed: cost => `扩容需要 ${cost} 金币。`,
    expanded: '背包扩容完成。',
    merged: (name, tier) => `${name} 合成为 T${tier}。`,
    noMerge: '没有可合成的同名同阶道具。',
    selectForge: '先选择一个可锻造道具。',
    forgeNoSpace: '锻造产物放不下。',
    forged: name => `锻造出 ${name}。`,
    noForge: '相邻道具没有可用锻造配方。',
    rotateBlocked: '旋转后会碰撞或越界。',
    sold: name => `出售 ${name}。`,
    upgraded: name => `升级：${name}。整理背包准备下一波。`,
    incoming: (wave, count) => `第 ${wave} 波来袭：${count} 个敌人。`,
    broken: wave => `防线在第 ${wave} 波崩溃。`,
    victory: gold => `胜利，获得 ${gold} 金币。选择一项升级。`,
    intro: '买入道具，整理背包，然后开始战斗。',
    canvasPrep: '整理背包后开始战斗',
    canvasReward: '战斗胜利：选择升级',
    langButton: 'EN',
    coin: '金币',
  },
};

const state = {
  lang: 'en',
  phase: 'prep',
  wave: 1,
  gold: 36,
  runes: 0,
  lives: 20,
  maxLives: 20,
  cols: 6,
  rows: 5,
  items: [],
  shop: [],
  selected: null,
  dragging: null,
  dragOffset: { x: 0, y: 0 },
  dragStart: { x: 0, y: 0 },
  dragMoved: false,
  enemies: [],
  projectiles: [],
  particles: [],
  floaters: [],
  spawnQueue: [],
  spawnTimer: 0,
  waveGold: 0,
  rewardChoices: [],
  frame: 0,
  last: 0,
  upgrades: {
    weaponDamage: 0,
    quickHands: 0,
    merchant: 0,
    vitality: 0,
    alchemy: 0,
    salvage: 0,
    lucky: 0,
    deepPack: 0,
  },
  messages: [],
};

let nextItemId = 1;

function uid() {
  return nextItemId++;
}

function itemDef(item) {
  return ITEM_DEFS[item.type];
}

function tx() {
  return TEXT[state.lang];
}

function localName(def) {
  return state.lang === 'zh' ? def.name : (def.nameEn || def.name);
}

function localDesc(def) {
  return state.lang === 'zh' ? def.desc : (def.descEn || def.desc);
}

function setLanguage(lang) {
  state.lang = lang;
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';
  renderAll();
  logEl.innerHTML = state.messages.map(m => `<div>${m}</div>`).join('');
}

function cloneShape(shape) {
  return shape.map(row => row.slice());
}

function rotateShape(shape) {
  const h = shape.length;
  const w = shape[0].length;
  const out = Array.from({ length: w }, () => Array(h).fill(0));
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) out[x][h - 1 - y] = shape[y][x];
  }
  return out;
}

function itemShape(item) {
  let shape = cloneShape(itemDef(item).shape);
  if (item.rotated) shape = rotateShape(shape);
  return shape;
}

function shapeSize(shape) {
  return { w: shape[0].length, h: shape.length };
}

function occupiedCells(item, atX = item.x, atY = item.y, shape = itemShape(item)) {
  const cells = [];
  for (let y = 0; y < shape.length; y++) {
    for (let x = 0; x < shape[y].length; x++) {
      if (shape[y][x]) cells.push({ x: atX + x, y: atY + y });
    }
  }
  return cells;
}

function canPlace(item, x, y, rotated = item.rotated, ignoreId = item.id) {
  const shape = rotated ? rotateShape(cloneShape(itemDef(item).shape)) : cloneShape(itemDef(item).shape);
  for (const cell of occupiedCells(item, x, y, shape)) {
    if (cell.x < 0 || cell.y < 0 || cell.x >= state.cols || cell.y >= state.rows) return false;
    const hit = state.items.find(other => other.id !== ignoreId && occupiedCells(other).some(c => c.x === cell.x && c.y === cell.y));
    if (hit) return false;
  }
  return true;
}

function firstFit(type, tier = 1) {
  const probe = { id: -1, type, tier, x: 0, y: 0, rotated: false };
  for (let y = 0; y < state.rows; y++) {
    for (let x = 0; x < state.cols; x++) {
      if (canPlace(probe, x, y, false, -1)) return { x, y, rotated: false };
      if (canPlace(probe, x, y, true, -1)) return { x, y, rotated: true };
    }
  }
  return null;
}

function price(type, tier = 1) {
  const discount = 1 - state.upgrades.merchant * 0.1;
  return Math.max(1, Math.floor(ITEM_DEFS[type].cost * tier * discount));
}

function sellValue(item) {
  const base = Math.floor(price(item.type, item.tier) * (state.upgrades.salvage ? 0.8 : 0.6));
  return Math.max(1, base);
}

function log(text) {
  state.messages.unshift(text);
  state.messages.length = 4;
  logEl.innerHTML = state.messages.map(m => `<div>${m}</div>`).join('');
}

function rollShop(extraReward = false) {
  state.shop = [];
  for (let i = 0; i < 4; i++) {
    let type = SHOP_POOL[Math.floor(Math.random() * SHOP_POOL.length)];
    let tier = 1;
    const highChance = extraReward ? 0.24 + state.upgrades.lucky * 0.08 : 0.05 + state.upgrades.lucky * 0.04;
    if (Math.random() < highChance) tier = 2;
    if (Math.random() < 0.03 + state.upgrades.lucky * 0.02) {
      const artifacts = Object.values(ITEM_DEFS).filter(d => d.artifact);
      const artifact = artifacts[Math.floor(Math.random() * artifacts.length)];
      type = Object.keys(ITEM_DEFS).find(k => ITEM_DEFS[k] === artifact);
      tier = 1;
    }
    state.shop.push({ type, tier });
  }
}

function createItem(type, tier = 1, x = 0, y = 0, rotated = false) {
  return { id: uid(), type, tier, x, y, rotated, cooldown: Math.random() * 30 };
}

function buyOffer(index) {
  if (state.phase !== 'prep') return;
  const offer = state.shop[index];
  if (!offer) return;
  const cost = price(offer.type, offer.tier);
  const spot = firstFit(offer.type, offer.tier);
  if (!spot) {
    log(tx().noSpace);
    return;
  }
  if (state.gold < cost) {
    log(tx().needGold(cost - state.gold));
    return;
  }
  state.gold -= cost;
  state.items.push(createItem(offer.type, offer.tier, spot.x, spot.y, spot.rotated));
  state.shop.splice(index, 1);
  log(tx().bought(localName(ITEM_DEFS[offer.type])));
  renderAll();
}

function rerollShop() {
  if (state.phase !== 'prep' || state.gold < 3) return;
  state.gold -= 3;
  rollShop();
  log(tx().rerolled);
  renderAll();
}

function expandPack() {
  if (state.phase !== 'prep') return;
  let cost = Math.max(8, 32 + (state.cols + state.rows - 11) * 12 - state.upgrades.deepPack * 18);
  if (state.cols >= 8 && state.rows >= 7) {
    log(tx().maxPack);
    return;
  }
  if (state.gold < cost) {
    log(tx().expandNeed(cost));
    return;
  }
  state.gold -= cost;
  if (state.cols <= state.rows && state.cols < 8) state.cols++;
  else if (state.rows < 7) state.rows++;
  else state.cols++;
  if (state.upgrades.deepPack > 0) state.upgrades.deepPack--;
  log(tx().expanded);
  renderAll();
}

function adjacentItems(item) {
  const cells = occupiedCells(item);
  return state.items.filter(other => {
    if (other.id === item.id) return false;
    const ocells = occupiedCells(other);
    return cells.some(a => ocells.some(b => Math.abs(a.x - b.x) + Math.abs(a.y - b.y) === 1));
  });
}

function mergeItems() {
  if (state.phase !== 'prep') return;
  const candidates = state.items.slice().sort((a, b) => a.y - b.y || a.x - b.x);
  for (const a of candidates) {
    const b = candidates.find(other => other.id !== a.id && other.type === a.type && other.tier === a.tier && other.tier < 3);
    if (!b) continue;
    state.items = state.items.filter(item => item.id !== b.id);
    a.tier++;
    a.cooldown = 0;
    if (!canPlace(a, a.x, a.y, a.rotated, a.id)) {
      const spot = firstFit(a.type, a.tier);
      if (spot) Object.assign(a, spot);
    }
    state.selected = a.id;
    log(tx().merged(localName(itemDef(a)), a.tier));
    renderAll();
    return;
  }
  log(tx().noMerge);
}

function forgeArtifact() {
  if (state.phase !== 'prep') return;
  const selected = state.items.find(i => i.id === state.selected);
  if (!selected) {
    log(tx().selectForge);
    return;
  }
  const near = adjacentItems(selected);
  for (const other of near) {
    const recipe = ARTIFACT_RECIPES.find(r =>
      (r.a === selected.type && r.b === other.type) || (r.b === selected.type && r.a === other.type)
    );
    if (!recipe) continue;
    const result = createItem(recipe.result, 1, selected.x, selected.y, false);
    state.items = state.items.filter(i => i.id !== selected.id && i.id !== other.id);
    if (!canPlace(result, result.x, result.y, result.rotated, result.id)) {
      const spot = firstFit(result.type, 1);
      if (!spot) {
        state.items.push(selected, other);
        log(tx().forgeNoSpace);
        renderAll();
        return;
      }
      Object.assign(result, spot);
    }
    state.items.push(result);
    state.selected = result.id;
    log(tx().forged(localName(ITEM_DEFS[result.type])));
    renderAll();
    return;
  }
  log(tx().noForge);
}

function rotateSelected() {
  const item = state.items.find(i => i.id === state.selected);
  if (!item || state.phase !== 'prep') return;
  const next = !item.rotated;
  if (canPlace(item, item.x, item.y, next, item.id)) {
    item.rotated = next;
    renderAll();
  } else {
    log(tx().rotateBlocked);
  }
}

function sellSelected() {
  const item = state.items.find(i => i.id === state.selected);
  if (!item || state.phase !== 'prep') return;
  state.gold += sellValue(item);
  state.items = state.items.filter(i => i.id !== item.id);
  state.selected = null;
  log(tx().sold(localName(itemDef(item))));
  renderAll();
}

function chooseUpgrade(id) {
  const up = UPGRADES.find(u => u.id === id);
  if (!up || state.phase !== 'reward') return;
  state.upgrades[id]++;
  if (id === 'vitality') {
    state.maxLives += 3;
    state.lives = Math.min(state.maxLives, state.lives + 3);
  }
  if (id === 'deepPack') state.gold += 6;
  state.phase = 'prep';
  state.wave++;
  state.rewardChoices = [];
  rollShop(true);
  log(tx().upgraded(localName(up)));
  renderAll();
}

function renderBackpack() {
  const cell = cellSize();
  backpackEl.style.setProperty('--cols', state.cols);
  backpackEl.style.setProperty('--rows', state.rows);
  backpackEl.innerHTML = '';
  for (let i = 0; i < state.cols * state.rows; i++) {
    const cell = document.createElement('div');
    cell.className = 'cell';
    backpackEl.appendChild(cell);
  }
  for (const item of state.items) {
    const def = itemDef(item);
    const shape = itemShape(item);
    const size = shapeSize(shape);
    const btn = document.createElement('button');
    btn.className = `pack-item${state.selected === item.id ? ' selected' : ''}`;
    btn.style.setProperty('--item-color', def.color);
    btn.style.left = `${6 + item.x * (cell + GAP)}px`;
    btn.style.top = `${6 + item.y * (cell + GAP)}px`;
    btn.style.width = `${size.w * cell + (size.w - 1) * GAP}px`;
    btn.style.height = `${size.h * cell + (size.h - 1) * GAP}px`;
    btn.dataset.id = item.id;
    btn.innerHTML = `${svgGlyph(item.type, def.color)}<span class="item-tier">T${item.tier}</span>`;
    btn.addEventListener('pointerdown', startDrag);
    btn.addEventListener('click', () => {
      state.selected = item.id;
      renderAll();
    });
    backpackEl.appendChild(btn);
  }
}

function cellSize() {
  const cell = backpackEl.querySelector('.cell');
  if (cell) return cell.getBoundingClientRect().width || 44;
  const raw = getComputedStyle(document.documentElement).getPropertyValue('--cell').trim();
  if (raw.startsWith('min(')) {
    const vw = window.innerWidth || 560;
    return Math.max(30, Math.min(38, (vw - 72) / 8));
  }
  return Number.parseFloat(raw) || 44;
}

function renderShop() {
  shopGrid.classList.toggle('hidden', state.phase === 'reward');
  upgradeChoices.classList.toggle('hidden', state.phase !== 'reward');
  shopGrid.innerHTML = '';
  state.shop.forEach((offer, index) => {
    const def = ITEM_DEFS[offer.type];
    const card = document.createElement('button');
    card.className = 'shop-card';
    card.style.setProperty('--item-color', def.color);
    card.disabled = state.phase !== 'prep';
    card.innerHTML = `
      ${svgGlyph(offer.type, def.color)}
      <div class="card-name">${localName(def)} T${offer.tier}</div>
      <div class="card-desc">${localDesc(def)}</div>
      <div class="card-cost">${price(offer.type, offer.tier)} ${tx().gold}</div>
    `;
    card.addEventListener('click', () => buyOffer(index));
    shopGrid.appendChild(card);
  });
}

function renderUpgradeChoices() {
  if (state.phase !== 'reward') return;
  upgradeChoices.innerHTML = '';
  if (!state.rewardChoices.length) {
    state.rewardChoices = UPGRADES.slice().sort(() => Math.random() - 0.5).slice(0, 3).map(up => up.id);
  }
  for (const id of state.rewardChoices) {
    const up = UPGRADES.find(entry => entry.id === id);
    const card = document.createElement('button');
    card.className = 'upgrade-card';
    card.innerHTML = `<strong>${localName(up)}</strong><span>${localDesc(up)}</span><span>${tx().current} Lv.${state.upgrades[up.id]}</span>`;
    card.addEventListener('click', () => chooseUpgrade(up.id));
    upgradeChoices.appendChild(card);
  }
}

function selectedText() {
  const item = state.items.find(i => i.id === state.selected);
  if (!item) return tx().selectedEmpty;
  const def = itemDef(item);
  const near = adjacentItems(item).map(i => localName(itemDef(i))).join(state.lang === 'zh' ? '、' : ', ') || tx().none;
  return `${localName(def)} T${item.tier} | ${localDesc(def)}<br>${tx().adjacent}: ${near} | ${tx().sellFor} ${sellValue(item)} ${tx().gold}`;
}

function renderHud() {
  document.getElementById('gameTitle').textContent = tx().title;
  document.getElementById('phaseLabel').textContent = tx().phases[state.phase];
  document.getElementById('waveLabel').textContent = tx().labels.wave;
  document.getElementById('lifeLabel').textContent = tx().labels.life;
  document.getElementById('goldLabel').textContent = tx().labels.gold;
  document.getElementById('runeLabel').textContent = tx().labels.runes;
  document.getElementById('waveStat').textContent = state.wave;
  document.getElementById('lifeStat').textContent = `${state.lives}/${state.maxLives}`;
  document.getElementById('goldStat').textContent = state.gold;
  document.getElementById('runeStat').textContent = state.runes;
  document.getElementById('packTitle').textContent = tx().packTitle;
  document.getElementById('packSize').textContent = `${state.cols} x ${state.rows}`;
  document.getElementById('selectedInfo').innerHTML = selectedText();
  document.getElementById('shopTitle').textContent = state.phase === 'reward' ? tx().upgrades : tx().shop;
  document.getElementById('roundInfo').textContent = state.phase === 'combat' ? tx().combatRound(state.wave) : tx().prepRound(state.wave);
  document.getElementById('langBtn').textContent = tx().langButton;
  document.getElementById('rotateBtn').textContent = tx().buttons.rotate;
  document.getElementById('mergeBtn').textContent = tx().buttons.merge;
  document.getElementById('forgeBtn').textContent = tx().buttons.forge;
  document.getElementById('sellBtn').textContent = tx().buttons.sell;
  document.getElementById('rerollBtn').textContent = `${tx().buttons.reroll} 3`;
  const expandCost = Math.max(8, 32 + (state.cols + state.rows - 11) * 12 - state.upgrades.deepPack * 18);
  document.getElementById('expandBtn').textContent = `${tx().buttons.expand} ${expandCost}`;
  document.getElementById('startBtn').textContent = tx().buttons.start;
  document.getElementById('restartBtn').textContent = tx().buttons.restart;
  document.getElementById('rerollBtn').disabled = state.phase !== 'prep' || state.gold < 3;
  document.getElementById('expandBtn').disabled = state.phase !== 'prep' || (state.cols >= 8 && state.rows >= 7);
  document.getElementById('startBtn').disabled = state.phase !== 'prep' || !state.items.some(i => itemDef(i).family === 'weapon');
  document.getElementById('startBtn').classList.toggle('hidden', state.phase === 'gameover');
  document.getElementById('restartBtn').classList.toggle('hidden', state.phase !== 'gameover');
  document.getElementById('rotateBtn').disabled = state.phase !== 'prep' || !state.selected;
  document.getElementById('mergeBtn').disabled = state.phase !== 'prep';
  document.getElementById('forgeBtn').disabled = state.phase !== 'prep' || !state.selected;
  document.getElementById('sellBtn').disabled = state.phase !== 'prep' || !state.selected;
}

function renderAll() {
  renderBackpack();
  renderShop();
  if (state.phase === 'reward') renderUpgradeChoices();
  renderHud();
}

function startDrag(ev) {
  if (state.phase !== 'prep') return;
  const item = state.items.find(i => i.id === Number(ev.currentTarget.dataset.id));
  if (!item) return;
  state.selected = item.id;
  state.dragging = { id: item.id, fromX: item.x, fromY: item.y };
  state.dragStart.x = ev.clientX;
  state.dragStart.y = ev.clientY;
  state.dragMoved = false;
  const rect = ev.currentTarget.getBoundingClientRect();
  state.dragOffset.x = ev.clientX - rect.left;
  state.dragOffset.y = ev.clientY - rect.top;
  ev.currentTarget.setPointerCapture(ev.pointerId);
  ev.currentTarget.classList.add('dragging');
  renderHud();
}

function pointerToCell(ev) {
  const rect = backpackEl.getBoundingClientRect();
  const cell = cellSize();
  return {
    x: Math.floor((ev.clientX - rect.left - 6) / (cell + GAP)),
    y: Math.floor((ev.clientY - rect.top - 6) / (cell + GAP)),
  };
}

window.addEventListener('pointermove', ev => {
  if (!state.dragging) return;
  const item = state.items.find(i => i.id === state.dragging.id);
  if (!item) return;
  const el = backpackEl.querySelector(`[data-id="${item.id}"]`);
  if (!el) return;
  if (Math.hypot(ev.clientX - state.dragStart.x, ev.clientY - state.dragStart.y) > 6) state.dragMoved = true;
  const rect = backpackEl.getBoundingClientRect();
  el.style.left = `${ev.clientX - rect.left - state.dragOffset.x}px`;
  el.style.top = `${ev.clientY - rect.top - state.dragOffset.y}px`;
});

function endDrag(ev) {
  if (!state.dragging) return;
  const item = state.items.find(i => i.id === state.dragging.id);
  const cell = pointerToCell(ev);
  if (item && state.dragMoved && canPlace(item, cell.x, cell.y, item.rotated, item.id)) {
    item.x = cell.x;
    item.y = cell.y;
  } else if (item) {
    item.x = state.dragging.fromX;
    item.y = state.dragging.fromY;
  }
  state.dragging = null;
  renderAll();
}

window.addEventListener('pointerup', endDrag);
window.addEventListener('pointercancel', endDrag);

function itemPower(item) {
  const def = itemDef(item);
  const near = adjacentItems(item);
  let damageMul = Math.pow(1.55, item.tier - 1) * (1 + state.upgrades.weaponDamage * 0.12);
  let cooldownMul = Math.pow(0.88, item.tier - 1) * (1 - state.upgrades.quickHands * 0.08);
  let splash = def.splash || 0;
  let chain = def.chain || 0;
  let elements = new Set(def.element ? [def.element] : []);
  for (const other of near) {
    const od = itemDef(other);
    const tierMul = 1 + (other.tier - 1) * 0.35;
    if (od.damageMul) damageMul *= 1 + (od.damageMul - 1) * tierMul;
    if (od.cooldownMul) cooldownMul *= Math.pow(od.cooldownMul, tierMul);
    if (od.element) elements.add(od.element);
    if (od.globalSpeed) cooldownMul *= od.globalSpeed;
  }
  for (const other of state.items) {
    const od = itemDef(other);
    if (od.globalSpeed && other.id !== item.id) cooldownMul *= od.globalSpeed;
  }
  if (state.upgrades.alchemy) damageMul *= 1 + elements.size * 0.08;
  return {
    damage: Math.round((def.damage || 0) * damageMul),
    cooldown: Math.max(14, (def.cooldown || 60) * cooldownMul),
    range: def.range || 220,
    splash,
    chain,
    bleed: def.bleed || 0,
    life: def.life || 0,
    elements,
  };
}

function waveQueue() {
  const queue = [];
  const w = state.wave;
  const push = (kind, count) => {
    for (let i = 0; i < count; i++) queue.push(kind);
  };
  push('rat', 6 + w);
  push('slime', 3 + Math.floor(w * 0.7));
  if (w >= 2) push('runner', 2 + Math.floor(w * 0.45));
  if (w >= 4) push('brute', Math.floor(w / 2));
  if (w >= 6) push('wisp', Math.floor(w / 2));
  if (w % 5 === 0) push('boss', 1);
  return queue.sort(() => Math.random() - 0.5);
}

function enemyStats(kind) {
  const base = ENEMY_DEFS[kind];
  const waveMul = 1 + (state.wave - 1) * 0.2;
  const bossMul = kind === 'boss' ? 1 + state.wave * 0.12 : 1;
  return {
    kind,
    name: base.name,
    x: -30,
    y: 236 + Math.sin(state.frame * 0.01) * 2,
    hp: Math.round(base.hp * waveMul * bossMul),
    maxHp: Math.round(base.hp * waveMul * bossMul),
    speed: base.speed * (1 + Math.min(0.45, state.wave * 0.015)),
    reward: base.reward + Math.floor(state.wave * 0.8),
    damage: base.damage,
    color: base.color,
    size: base.size,
    seed: Math.random() * 9,
    hit: 0,
    slow: 0,
    burn: 0,
    poison: 0,
    bleed: 0,
  };
}

function startCombat() {
  if (state.phase !== 'prep' || !state.items.some(i => itemDef(i).family === 'weapon')) return;
  state.phase = 'combat';
  state.enemies = [];
  state.projectiles = [];
  state.particles = [];
  state.spawnQueue = waveQueue();
  state.spawnTimer = 0;
  state.waveGold = 0;
  log(tx().incoming(state.wave, state.spawnQueue.length));
  renderAll();
}

function acquireTarget(item, power) {
  const origin = weaponOrigin(item);
  let best = null;
  let bestScore = -Infinity;
  for (const e of state.enemies) {
    const d = Math.hypot(e.x - origin.x, e.y - origin.y);
    if (d > power.range || e.hp <= 0) continue;
    const score = e.x + (e.kind === 'boss' ? 80 : 0) - d * 0.05;
    if (score > bestScore) {
      best = e;
      bestScore = score;
    }
  }
  return best;
}

function weaponOrigin(item) {
  const cells = occupiedCells(item);
  const cx = cells.reduce((sum, c) => sum + c.x, 0) / cells.length;
  const cy = cells.reduce((sum, c) => sum + c.y, 0) / cells.length;
  return {
    x: 80 + cx * 58,
    y: 70 + cy * 30,
  };
}

function damageEnemy(enemy, amount, power, source) {
  enemy.hp -= amount;
  enemy.hit = 5;
  if (power.elements.has('frost')) enemy.slow = Math.max(enemy.slow, 95 + state.upgrades.alchemy * 30);
  if (power.elements.has('ember')) enemy.burn = Math.max(enemy.burn, 110 + state.upgrades.alchemy * 28);
  if (power.elements.has('poison')) enemy.poison = Math.max(enemy.poison, 140 + state.upgrades.alchemy * 35);
  if (power.elements.has('shock')) power.chain += 1;
  if (power.bleed) enemy.bleed += power.bleed;
  state.floaters.push({ x: enemy.x, y: enemy.y - enemy.size, text: String(amount), color: source || '#f8fbff', life: 42 });
}

function fireWeapon(item, target, power) {
  const origin = weaponOrigin(item);
  const color = power.elements.has('frost') ? '#8fe7ff' : power.elements.has('ember') ? '#ff9a55' : power.elements.has('poison') ? '#92e36d' : '#f2c14e';
  state.projectiles.push({
    x: origin.x,
    y: origin.y,
    tx: target.x,
    ty: target.y,
    target,
    speed: itemDef(item).family === 'weapon' && item.type.includes('cannon') ? 9 : 13,
    damage: power.damage,
    splash: power.splash,
    chain: power.chain,
    color,
    power: { ...power, elements: new Set(power.elements) },
  });
}

function updateCombat(dt) {
  if (state.spawnQueue.length) {
    state.spawnTimer -= dt;
    if (state.spawnTimer <= 0) {
      state.enemies.push(enemyStats(state.spawnQueue.shift()));
      state.spawnTimer = Math.max(18, 58 - state.wave * 2);
    }
  }
  for (const enemy of state.enemies) {
    const slowMul = enemy.slow > 0 ? 0.48 : 1;
    enemy.x += enemy.speed * slowMul * dt;
    enemy.hit = Math.max(0, enemy.hit - dt);
    enemy.slow = Math.max(0, enemy.slow - dt);
    enemy.burn = Math.max(0, enemy.burn - dt);
    enemy.poison = Math.max(0, enemy.poison - dt);
    if (enemy.burn > 0 && state.frame % 18 === 0) enemy.hp -= 3 + state.wave;
    if (enemy.poison > 0 && state.frame % 24 === 0) enemy.hp -= 4 + state.upgrades.alchemy * 2;
    if (enemy.bleed > 0 && state.frame % 30 === 0) enemy.hp -= enemy.bleed;
  }
  for (const item of state.items) {
    const def = itemDef(item);
    if (def.family !== 'weapon') continue;
    item.cooldown -= dt;
    if (item.cooldown <= 0) {
      const power = itemPower(item);
      const target = acquireTarget(item, power);
      if (target) {
        fireWeapon(item, target, power);
        item.cooldown = power.cooldown;
      }
    }
  }
  updateProjectiles(dt);
  resolveEnemies();
  if (!state.spawnQueue.length && state.enemies.length === 0 && state.phase === 'combat') finishWave(true);
}

function updateProjectiles(dt) {
  for (const p of state.projectiles) {
    if (!p.target || p.target.hp <= 0) p.target = state.enemies.find(e => e.hp > 0) || null;
    if (!p.target) {
      p.done = true;
      continue;
    }
    const dx = p.target.x - p.x;
    const dy = p.target.y - p.y;
    const d = Math.max(1, Math.hypot(dx, dy));
    p.x += dx / d * p.speed * dt;
    p.y += dy / d * p.speed * dt;
    if (d < p.speed * dt + p.target.size) {
      damageEnemy(p.target, p.damage, p.power, p.color);
      if (p.splash) {
        for (const e of state.enemies) {
          if (e !== p.target && Math.hypot(e.x - p.target.x, e.y - p.target.y) < p.splash) {
            damageEnemy(e, Math.round(p.damage * 0.55), p.power, p.color);
          }
        }
      }
      let chainTarget = p.target;
      for (let i = 0; i < p.chain; i++) {
        const next = state.enemies.find(e => e !== chainTarget && e.hp > 0 && Math.hypot(e.x - chainTarget.x, e.y - chainTarget.y) < 120);
        if (!next) break;
        damageEnemy(next, Math.round(p.damage * 0.45), p.power, '#d8f25a');
        chainTarget = next;
      }
      p.done = true;
      state.particles.push({ x: p.target.x, y: p.target.y, r: p.splash || 24, color: p.color, life: 18 });
    }
  }
  state.projectiles = state.projectiles.filter(p => !p.done);
}

function resolveEnemies() {
  const armor = state.items.reduce((sum, item) => sum + (itemDef(item).armor || 0) * item.tier, 0);
  for (const enemy of state.enemies) {
    if (enemy.hp <= 0 && !enemy.dead) {
      enemy.dead = true;
      const bonus = 1 + state.items.reduce((sum, item) => sum + (itemDef(item).goldBonus || 0) * item.tier, 0);
      const gained = Math.max(1, Math.round(enemy.reward * bonus));
      state.gold += gained;
      state.waveGold += gained;
      if (enemy.kind === 'boss') state.runes += 2;
      for (const item of state.items) {
        if (itemDef(item).life && state.lives < state.maxLives) state.lives += itemDef(item).life;
      }
    }
    if (enemy.x > canvas.width - 64 && !enemy.escaped && enemy.hp > 0) {
      enemy.escaped = true;
      state.lives -= Math.max(1, enemy.damage - Math.floor(armor / 2));
      state.floaters.push({ x: canvas.width - 78, y: 188, text: `-${enemy.damage}`, color: '#e05243', life: 50 });
      if (state.lives <= 0) finishWave(false);
    }
  }
  state.enemies = state.enemies.filter(e => !e.dead && !e.escaped);
}

function finishWave(won) {
  if (!won) {
    state.phase = 'gameover';
    log(tx().broken(state.wave));
    renderAll();
    return;
  }
  const herbHeal = state.items.reduce((sum, item) => sum + (itemDef(item).heal || 0) * item.tier, 0);
  const runeBonus = state.items.reduce((sum, item) => sum + (itemDef(item).runeBonus || 0) * item.tier, 0) + state.upgrades.alchemy;
  const interest = state.items.reduce((sum, item) => sum + (itemDef(item).interest || 0) * item.tier, 0);
  state.lives = Math.min(state.maxLives, state.lives + herbHeal);
  state.runes += runeBonus;
  if (interest) state.gold += Math.floor(state.gold * interest);
  state.phase = 'reward';
  state.rewardChoices = [];
  log(tx().victory(state.waveGold));
  renderAll();
}

function drawBattle() {
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = '#111722';
  ctx.fillRect(0, 0, w, h);
  for (let y = 0; y < h; y += 32) {
    for (let x = 0; x < w; x += 32) {
      ctx.fillStyle = (x / 32 + y / 32) % 2 ? '#131b27' : '#101620';
      ctx.fillRect(x, y, 32, 32);
    }
  }
  ctx.fillStyle = '#202b3a';
  ctx.fillRect(0, 210, w, 58);
  ctx.fillStyle = '#2d394a';
  for (let x = 0; x < w; x += 34) ctx.fillRect(x, 234, 18, 4);
  ctx.fillStyle = 'rgba(242,193,78,0.14)';
  ctx.fillRect(36, 28, 390, 186);
  for (const item of state.items) {
    const def = itemDef(item);
    if (def.family !== 'weapon') continue;
    const o = weaponOrigin(item);
    const power = itemPower(item);
    ctx.globalAlpha = 0.14;
    ctx.fillStyle = def.color;
    ctx.beginPath();
    ctx.arc(o.x, o.y, power.range, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#0a0d13';
    ctx.fillRect(o.x - 12, o.y - 12, 24, 24);
    ctx.fillStyle = def.color;
    ctx.fillRect(o.x - 9, o.y - 9, 18, 18);
    ctx.fillStyle = '#f8fbff';
    ctx.fillRect(o.x - 3, o.y - 3, 6, 6);
  }
  drawGate(ctx, w - 46, 230, state.lives, state.maxLives);
  for (const p of state.projectiles) {
    ctx.fillStyle = p.color;
    ctx.fillRect(Math.round(p.x - 3), Math.round(p.y - 3), 6, 6);
    ctx.fillStyle = '#f8fbff';
    ctx.fillRect(Math.round(p.x - 1), Math.round(p.y - 1), 2, 2);
  }
  for (const enemy of state.enemies) drawPixelEnemy(ctx, enemy, state.frame);
  for (const p of state.particles) {
    ctx.globalAlpha = Math.max(0, p.life / 18) * 0.55;
    ctx.strokeStyle = p.color;
    ctx.lineWidth = 3;
    ctx.strokeRect(p.x - p.r / 2, p.y - p.r / 2, p.r, p.r);
    ctx.globalAlpha = 1;
    p.life--;
  }
  state.particles = state.particles.filter(p => p.life > 0);
  for (const f of state.floaters) {
    ctx.globalAlpha = Math.max(0, f.life / 42);
    ctx.fillStyle = f.color;
    ctx.font = '700 14px Courier New';
    ctx.fillText(f.text, f.x, f.y);
    ctx.globalAlpha = 1;
    f.y -= 0.45;
    f.life--;
  }
  state.floaters = state.floaters.filter(f => f.life > 0);
  ctx.fillStyle = '#f2c14e';
  ctx.font = '700 16px Courier New';
  if (state.phase !== 'combat') ctx.fillText(state.phase === 'reward' ? tx().canvasReward : tx().canvasPrep, 22, 34);
}

function loop(time) {
  const dt = Math.min(2.4, (time - state.last) / 16.67 || 1);
  state.last = time;
  state.frame++;
  if (state.phase === 'combat') updateCombat(dt);
  drawBattle();
  requestAnimationFrame(loop);
}

function resetGame() {
  Object.assign(state, {
    phase: 'prep',
    wave: 1,
    gold: 36,
    runes: 0,
    lives: 20,
    maxLives: 20,
    cols: 6,
    rows: 5,
    items: [],
    selected: null,
    enemies: [],
    projectiles: [],
    particles: [],
    floaters: [],
    spawnQueue: [],
    waveGold: 0,
    upgrades: {
      weaponDamage: 0,
      quickHands: 0,
      merchant: 0,
      vitality: 0,
      alchemy: 0,
      salvage: 0,
      lucky: 0,
      deepPack: 0,
    },
    rewardChoices: [],
    messages: [],
  });
  const starter = createItem('dagger', 1, 1, 1, false);
  const bow = createItem('bow', 1, 3, 1, false);
  const gem = createItem('gear', 1, 2, 1, false);
  state.items.push(starter, bow, gem);
  rollShop();
  logEl.innerHTML = '';
  log(tx().intro);
  renderAll();
}

document.getElementById('rerollBtn').addEventListener('click', rerollShop);
document.getElementById('expandBtn').addEventListener('click', expandPack);
document.getElementById('startBtn').addEventListener('click', startCombat);
document.getElementById('restartBtn').addEventListener('click', resetGame);
document.getElementById('rotateBtn').addEventListener('click', rotateSelected);
document.getElementById('mergeBtn').addEventListener('click', mergeItems);
document.getElementById('forgeBtn').addEventListener('click', forgeArtifact);
document.getElementById('sellBtn').addEventListener('click', sellSelected);
document.getElementById('langBtn').addEventListener('click', () => setLanguage(state.lang === 'en' ? 'zh' : 'en'));

resetGame();
requestAnimationFrame(loop);
