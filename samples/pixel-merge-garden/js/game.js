const SIZE = 25;
const WIDTH = 5;
const CROPS = [
  { name: 'Sprout', color: '#62d879', stem: '#244b2d' },
  { name: 'Mintbud', color: '#75e3a2', stem: '#2f6d3a' },
  { name: 'Dewcap', color: '#64c7ff', stem: '#315f80' },
  { name: 'Glowroot', color: '#aa7dff', stem: '#594b94' },
  { name: 'Sunpod', color: '#f4c85a', stem: '#8c671e' },
  { name: 'Pepperstar', color: '#ff9266', stem: '#8a4a30' },
  { name: 'Roseflare', color: '#f0647f', stem: '#8b3148' },
  { name: 'Pearlfruit', color: '#ffffff', stem: '#798499' },
];
const MUTATIONS = {
  plain: { label: '', mult: 1, color: '#ffffff' },
  silver: { label: 'S', mult: 2.2, color: '#c9d7e8' },
  gold: { label: 'G', mult: 5, color: '#f4c85a' },
};

const state = {
  coins: 24,
  board: Array(SIZE).fill(null),
  selected: null,
  bank: 0,
  best: 1,
  tick: 0,
  rain: 0,
  streak: 0,
  greenhouse: 1,
  order: null,
  floaters: [],
};

function crop(level = 1, mutation = 'plain') {
  return { level, mutation, age: 0 };
}

function cropName(level) {
  return CROPS[(level - 1) % CROPS.length].name;
}

function seedCost() {
  const filled = state.board.filter(Boolean).length;
  return Math.floor(9 + filled * 2.4 + state.best * 3.5 + state.greenhouse * 5);
}

function waterCost() {
  return 30 + state.greenhouse * 18 + state.best * 5;
}

function cropValue(c) {
  if (!c) return 0;
  return Math.pow(2, c.level - 1) * 0.16 * MUTATIONS[c.mutation].mult * state.greenhouse * (state.rain > 0 ? 2 : 1);
}

function incomeRate() {
  return state.board.reduce((sum, c) => sum + cropValue(c), 0);
}

function newOrder() {
  const target = Math.max(2, Math.min(state.best, 2 + Math.floor(Math.random() * Math.max(1, state.best))));
  state.order = {
    level: target,
    need: 2 + Math.floor(target / 3),
    have: 0,
    reward: Math.floor(35 * Math.pow(1.72, target - 1)),
  };
}

function addFloater(text, color = '#f4c85a') {
  state.floaters.push({ text, color, life: 70 });
}

function rollMutation(a, b) {
  const rank = { plain: 0, silver: 1, gold: 2 };
  let base = Math.max(rank[a.mutation], rank[b.mutation]);
  const chance = 0.05 + state.greenhouse * 0.015 + state.streak * 0.01;
  if (Math.random() < chance) base++;
  return Object.keys(rank)[Math.min(base, 2)];
}

function drawCrop(c) {
  const canvas = document.createElement('canvas');
  canvas.width = 56;
  canvas.height = 56;
  canvas.className = 'crop';
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  const data = CROPS[(c.level - 1) % CROPS.length];
  ctx.fillStyle = '#172016';
  ctx.fillRect(0, 0, 56, 56);
  ctx.fillStyle = '#233a22';
  ctx.fillRect(8, 44, 40, 5);
  ctx.fillStyle = data.stem;
  ctx.fillRect(25, 27, 7, 19);
  ctx.fillRect(19, 33, 18, 5);
  ctx.fillStyle = data.color;
  const size = Math.min(34, 13 + c.level * 3);
  ctx.fillRect(28 - size / 2, 24 - size / 2, size, size);
  ctx.fillRect(28 - size / 3, 15, Math.max(8, size / 1.5), 8);
  ctx.fillStyle = 'rgba(255,255,255,0.35)';
  ctx.fillRect(19, 16, 14, 5);
  if (c.mutation !== 'plain') {
    ctx.strokeStyle = MUTATIONS[c.mutation].color;
    ctx.lineWidth = 3;
    ctx.strokeRect(5, 5, 46, 46);
    ctx.fillStyle = MUTATIONS[c.mutation].color;
    ctx.fillRect(40, 8, 8, 8);
  }
  return canvas;
}

function render() {
  const board = document.getElementById('board');
  board.innerHTML = '';
  state.board.forEach((c, i) => {
    const cell = document.createElement('button');
    cell.className = `cell${state.selected === i ? ' selected' : ''}${c && c.mutation !== 'plain' ? ` ${c.mutation}` : ''}`;
    cell.type = 'button';
    cell.onclick = () => clickCell(i);
    if (c) {
      const mutationLabel = c.mutation === 'plain' ? '' : ` ${c.mutation}`;
      cell.setAttribute('aria-label', `Plot ${i + 1}: level ${c.level}${mutationLabel} ${cropName(c.level)}. Tap another matching crop to merge.`);
      cell.title = `L${c.level}${mutationLabel} ${cropName(c.level)}`;
      cell.appendChild(drawCrop(c));
      const label = document.createElement('span');
      label.className = 'level';
      label.textContent = `L${c.level}${MUTATIONS[c.mutation].label}`;
      cell.appendChild(label);
      if (state.order && c.level === state.order.level) {
        const pin = document.createElement('span');
        pin.className = 'pin';
        pin.textContent = '!';
        cell.appendChild(pin);
      }
    } else {
      cell.setAttribute('aria-label', `Plot ${i + 1}: empty`);
    }
    board.appendChild(cell);
  });
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('bank').textContent = Math.floor(state.bank);
  document.getElementById('best').textContent = `${state.best} ${cropName(state.best)}`;
  document.getElementById('income').textContent = `${incomeRate().toFixed(1)}/s${state.rain > 0 ? ' rain' : ''}`;
  document.getElementById('order').textContent = `L${state.order.level} ${state.order.have}/${state.order.need} +${state.order.reward}`;
  document.getElementById('buy').textContent = `Buy Seed ${seedCost()}`;
  document.getElementById('buy').disabled = state.coins < seedCost() || !state.board.includes(null);
  document.getElementById('collect').textContent = `Collect ${Math.floor(state.bank)}`;
  document.getElementById('water').textContent = `Water All ${waterCost()}`;
  document.getElementById('water').disabled = state.coins < waterCost() || state.rain > 0;
  renderFloaters();
}

function renderFloaters() {
  let layer = document.getElementById('floaters');
  if (!layer) {
    layer = document.createElement('div');
    layer.id = 'floaters';
    document.querySelector('.app').appendChild(layer);
  }
  layer.innerHTML = state.floaters.map(f => `<span style="color:${f.color};opacity:${Math.max(0, f.life / 70)}">${f.text}</span>`).join('');
}

function clickCell(i) {
  const current = state.board[i];
  if (!current) {
    state.selected = null;
    render();
    return;
  }
  if (state.selected === null) {
    state.selected = i;
  } else if (state.selected === i) {
    state.selected = null;
  } else {
    const aIndex = state.selected;
    const a = state.board[aIndex];
    if (a && current && a.level === current.level) {
      const next = crop(current.level + 1, rollMutation(a, current));
      state.board[i] = next;
      state.board[aIndex] = null;
      state.best = Math.max(state.best, next.level);
      state.streak++;
      const bonus = Math.floor(next.level * 3 * MUTATIONS[next.mutation].mult + state.streak);
      state.coins += bonus;
      if (state.order.level === next.level) state.order.have++;
      if (state.order.have >= state.order.need) {
        state.coins += state.order.reward;
        addFloater(`ORDER +${state.order.reward}`, '#f4c85a');
        newOrder();
      } else {
        addFloater(`MERGE +${bonus}`, MUTATIONS[next.mutation].color);
      }
      if (state.best % 4 === 0 && next.level === state.best) state.greenhouse = Math.max(state.greenhouse, 1 + Math.floor(state.best / 4));
    } else {
      state.streak = 0;
    }
    state.selected = null;
  }
  render();
}

function buySeed() {
  const cost = seedCost();
  const empty = state.board.map((v, i) => v ? -1 : i).filter(i => i >= 0);
  if (state.coins < cost || !empty.length) return;
  state.coins -= cost;
  const mutation = Math.random() < 0.04 + state.greenhouse * 0.01 ? 'silver' : 'plain';
  state.board[empty[Math.floor(Math.random() * empty.length)]] = crop(1, mutation);
  addFloater(mutation === 'silver' ? 'SILVER SEED' : 'SEED', mutation === 'silver' ? '#c9d7e8' : '#62d879');
  render();
}

function collect() {
  state.coins += Math.floor(state.bank);
  if (state.bank >= 1) addFloater(`COLLECT +${Math.floor(state.bank)}`, '#f4c85a');
  state.bank = 0;
  render();
}

function waterAll() {
  const cost = waterCost();
  if (state.coins < cost || state.rain > 0) return;
  state.coins -= cost;
  state.rain = 30;
  addFloater('RAIN BOOST', '#64c7ff');
  render();
}

function reset() {
  state.coins = 24;
  state.board = Array(SIZE).fill(null);
  state.selected = null;
  state.bank = 0;
  state.best = 1;
  state.rain = 0;
  state.streak = 0;
  state.greenhouse = 1;
  state.floaters = [];
  state.board[12] = crop(1);
  state.board[13] = crop(1);
  state.board[7] = crop(1);
  newOrder();
  render();
}

document.getElementById('buy').onclick = buySeed;
document.getElementById('collect').onclick = collect;
document.getElementById('water').onclick = waterAll;
document.getElementById('reset').onclick = reset;

setInterval(() => {
  state.bank += incomeRate();
  state.tick++;
  for (const c of state.board) if (c) c.age++;
  for (const f of state.floaters) f.life -= 14;
  state.floaters = state.floaters.filter(f => f.life > 0);
  if (state.rain > 0) state.rain--;
  render();
}, 1000);

reset();
