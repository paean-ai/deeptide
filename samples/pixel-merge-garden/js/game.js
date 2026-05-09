const SIZE = 25;
const state = {
  coins: 20,
  board: Array(SIZE).fill(0),
  selected: null,
  bank: 0,
  best: 1,
  tick: 0,
};

const colors = ['#62d879', '#75e3a2', '#64c7ff', '#aa7dff', '#f4c85a', '#ff9266', '#f0647f', '#ffffff'];

function seedCost() {
  const filled = state.board.filter(Boolean).length;
  return 10 + Math.floor(filled * 2 + state.best * 3);
}

function incomeRate() {
  return state.board.reduce((sum, lv) => sum + (lv ? Math.pow(2, lv - 1) * 0.15 : 0), 0);
}

function drawCrop(level) {
  const c = document.createElement('canvas');
  c.width = 48;
  c.height = 48;
  c.className = 'crop';
  const ctx = c.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  const color = colors[(level - 1) % colors.length];
  ctx.fillStyle = '#172016';
  ctx.fillRect(0, 0, 48, 48);
  ctx.fillStyle = '#244b2d';
  ctx.fillRect(20, 24, 8, 18);
  ctx.fillStyle = color;
  const size = Math.min(28, 12 + level * 3);
  ctx.fillRect(24 - size / 2, 22 - size / 2, size, size);
  ctx.fillStyle = 'rgba(255,255,255,0.28)';
  ctx.fillRect(24 - size / 2 + 4, 22 - size / 2 + 4, Math.max(4, size / 3), 4);
  return c;
}

function render() {
  const board = document.getElementById('board');
  board.innerHTML = '';
  state.board.forEach((lv, i) => {
    const cell = document.createElement('button');
    cell.className = `cell${state.selected === i ? ' selected' : ''}`;
    cell.onclick = () => clickCell(i);
    if (lv) {
      cell.appendChild(drawCrop(lv));
      const label = document.createElement('span');
      label.className = 'level';
      label.textContent = `L${lv}`;
      cell.appendChild(label);
    }
    board.appendChild(cell);
  });
  document.getElementById('coins').textContent = Math.floor(state.coins);
  document.getElementById('best').textContent = state.best;
  document.getElementById('income').textContent = `${incomeRate().toFixed(1)}/s`;
  document.getElementById('buy').textContent = `Buy Seed (${seedCost()})`;
  document.getElementById('buy').disabled = state.coins < seedCost() || !state.board.includes(0);
  document.getElementById('collect').textContent = `Collect (${Math.floor(state.bank)})`;
}

function clickCell(i) {
  if (!state.board[i]) {
    state.selected = null;
    render();
    return;
  }
  if (state.selected === null) {
    state.selected = i;
  } else if (state.selected === i) {
    state.selected = null;
  } else {
    const a = state.selected;
    if (state.board[a] === state.board[i]) {
      state.board[i]++;
      state.board[a] = 0;
      state.best = Math.max(state.best, state.board[i]);
      state.coins += state.board[i] * 2;
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
  state.board[empty[Math.floor(Math.random() * empty.length)]] = 1;
  render();
}

function collect() {
  state.coins += Math.floor(state.bank);
  state.bank = 0;
  render();
}

function reset() {
  state.coins = 20;
  state.board = Array(SIZE).fill(0);
  state.selected = null;
  state.bank = 0;
  state.best = 1;
  state.board[12] = 1;
  state.board[13] = 1;
  render();
}

document.getElementById('buy').onclick = buySeed;
document.getElementById('collect').onclick = collect;
document.getElementById('reset').onclick = reset;

setInterval(() => {
  state.bank += incomeRate();
  state.tick++;
  if (state.tick % 2 === 0) render();
}, 1000);

reset();
