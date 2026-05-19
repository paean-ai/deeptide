// Pixel Auto Arena - a compact auto-battler: draft a squad, merge duplicates,
// arrange the line, then watch them auto-fight escalating enemy squads.

const BEST_KEY = 'pixel-auto-arena-best';
const CW = 480, CH = 300;
const STEP = 0.6;          // seconds per combat clash

const canvas = document.getElementById('arena');
const ctx = canvas.getContext('2d');
canvas.width = CW;
canvas.height = CH;
ctx.imageSmoothingEnabled = false;

let game = null;
let lastT = performance.now();

const rand = n => Math.floor(Math.random() * n);

// ---- run lifecycle -----------------------------------------------------
function newGame() {
  game = {
    round: 1, gold: goldFor(1), lives: START_LIVES, streak: 0,
    team: [], shop: [], selected: -1, phase: 'shop',
    bA: [], bB: [], battleTimer: 0, popups: [], t: 0, lastResult: '',
  };
  rollShop();
}

// ---- shop --------------------------------------------------------------
function rollShop() {
  const maxTier = shopTier(game.round);
  const pool = [];
  for (const u of UNITS) {
    if (u.tier > maxTier) continue;
    const weight = u.tier === 1 ? 6 : u.tier === 2 ? 3 : 2;
    for (let i = 0; i < weight; i++) pool.push(u.id);
  }
  game.shop = [];
  for (let i = 0; i < SHOP_SLOTS; i++) game.shop.push({ id: pool[rand(pool.length)] });
}

function buyUnit(slot) {
  if (game.phase !== 'shop') return;
  const offer = game.shop[slot];
  if (!offer) return;
  const u = unitById(offer.id);
  if (game.gold < u.tier) { toast(t('needGold')); return; }
  // a buy is allowed when the squad is full only if it completes a merge
  const willMerge = game.team.filter(m => m.id === offer.id && m.star === 1).length >= 2;
  if (game.team.length >= teamCap(game.round) && !willMerge) { toast(t('teamFull')); return; }
  game.gold -= u.tier;
  game.team.push({ id: offer.id, star: 1 });
  game.shop[slot] = null;
  resolveMerges();
  renderShop();
  renderArena();
  updateHud();
}

function resolveMerges() {
  let merged = true;
  while (merged) {
    merged = false;
    for (let star = 1; star <= 2; star++) {
      for (const u of UNITS) {
        const same = game.team.filter(m => m.id === u.id && m.star === star);
        if (same.length >= 3) {
          game.team = game.team.filter(m => !(m.id === u.id && m.star === star));
          game.team.push({ id: u.id, star: star + 1 });
          toast(t('merged'));
          merged = true;
        }
      }
    }
  }
}

function sellSelected() {
  if (game.phase !== 'shop' || game.selected < 0) return;
  const m = game.team[game.selected];
  if (!m) return;
  game.gold += unitById(m.id).tier * m.star;
  game.team.splice(game.selected, 1);
  game.selected = -1;
  renderShop(); renderArena(); updateHud();
}

function reroll() {
  if (game.phase !== 'shop' || game.gold < REROLL_COST) return;
  game.gold -= REROLL_COST;
  rollShop();
  renderShop(); updateHud();
}

// ---- synergy / battle units -------------------------------------------
function classCounts(team) {
  const c = { beast: 0, mech: 0, mage: 0 };
  for (const m of team) c[unitById(m.id).cls]++;
  return c;
}

function buildBattleTeam(team) {
  const counts = classCounts(team);
  return team.map(m => {
    const u = unitById(m.id), cls = CLASSES[u.cls], mul = STAR_MUL[m.star];
    let hp = Math.round(u.hp * mul), atk = Math.round(u.atk * mul);
    const c = counts[u.cls];
    if (c >= 4) { hp += cls.t4.hp || 0; atk += cls.t4.atk || 0; }
    else if (c >= 2) { hp += cls.t2.hp || 0; atk += cls.t2.atk || 0; }
    return { id: u.id, glyph: u.glyph, cls: u.cls, color: cls.color, star: m.star,
      hp, maxHp: hp, atk: Math.max(1, atk), flash: 0 };
  });
}

function makeEnemyTeam(round) {
  const n = Math.min(MAX_TEAM, 1 + Math.floor(round * 0.7));
  const maxTier = shopTier(round);
  const pool = UNITS.filter(u => u.tier <= maxTier);
  const arr = [];
  for (let i = 0; i < n; i++) {
    let star = 1;
    if (round > 4 && Math.random() < 0.16 + round * 0.02) star = 2;
    if (round > 9 && Math.random() < 0.14) star = 3;
    arr.push({ id: pool[rand(pool.length)].id, star });
  }
  return arr;
}

function startBattle() {
  if (game.phase !== 'shop') return;
  if (!game.team.length) { toast(t('teamFull')); return; }
  game.bA = buildBattleTeam(game.team);
  game.bB = buildBattleTeam(makeEnemyTeam(game.round));
  const m = 1 + game.round * 0.05;
  for (const b of game.bB) { b.hp = Math.round(b.hp * m); b.maxHp = b.hp; b.atk = Math.round(b.atk * m); }
  game.phase = 'battle';
  game.battleTimer = STEP * 0.7;
  game.selected = -1;
  renderShop();
  updateHud();
}

function clash() {
  const A = game.bA, B = game.bB;
  if (!A.length || !B.length) { finishBattle(); return; }
  const a = A[0], b = B[0];
  a.hp -= b.atk; b.hp -= a.atk;
  a.flash = 0.3; b.flash = 0.3;
  popup(unitSlotX('a', 0, A.length), 150, '-' + b.atk, '#ff6b6b');
  popup(unitSlotX('b', 0, B.length), 150, '-' + a.atk, '#ff6b6b');
  if (a.hp <= 0) A.shift();
  if (b.hp <= 0) B.shift();
}

function finishBattle() {
  const a = game.bA.length, b = game.bB.length;
  let result;
  if (a > 0 && b === 0) result = 'win';
  else if (b > 0 && a === 0) result = 'lose';
  else result = 'draw';
  game.lastResult = result;
  if (result === 'lose') { game.lives--; game.streak = 0; }
  else if (result === 'win') { game.streak++; }
  if (game.lives <= 0) {
    game.phase = 'gameover';
    const r = game.round;
    if (r > bestRound()) localStorage.setItem(BEST_KEY, r);
    document.getElementById('over-msg').textContent = t('reachedRound', r);
    document.getElementById('over-best').textContent = t('bestRound', bestRound());
    showOverlay('overlay-over');
    return;
  }
  game.phase = 'result';
  const title = document.getElementById('result-title');
  title.textContent = t(result);
  title.className = result === 'win' ? 'win' : result === 'lose' ? 'lose' : 'draw';
  document.getElementById('result-msg').textContent =
    result === 'win' ? `+${Math.min(3, game.streak)} ${t('gold')} streak bonus` : t('frontHint');
  showOverlay('overlay-result');
}

function nextRound() {
  hideAllOverlays();
  game.round++;
  game.gold = goldFor(game.round) + Math.min(3, game.streak);
  game.bA = []; game.bB = []; game.popups = [];
  game.phase = 'shop';
  rollShop();
  renderShop(); renderArena(); updateHud();
}

// ---- popups ------------------------------------------------------------
function popup(x, y, text, color) {
  game.popups.push({ x: x + rand(20) - 10, y, text, color, life: 1 });
}

// ---- update ------------------------------------------------------------
function update(dt) {
  game.t += dt;
  for (const b of [...game.bA, ...game.bB]) if (b.flash > 0) b.flash -= dt;
  for (const p of game.popups) { p.y -= 26 * dt; p.life -= dt * 1.4; }
  game.popups = game.popups.filter(p => p.life > 0);
  if (game.phase === 'battle') {
    game.battleTimer -= dt;
    if (game.battleTimer <= 0) {
      game.battleTimer = STEP;
      clash();
    }
  }
}

// ---- rendering ---------------------------------------------------------
function unitSlotX(side, i, count) {
  if (game.phase === 'shop') {
    // your squad only, centred, front (index 0) on the right
    return CW / 2 + ((count - 1) / 2 - i) * 60;
  }
  // battle: yours left of centre, enemy right
  return side === 'a' ? CW / 2 - 16 - i * 40 : CW / 2 + 16 + i * 40;
}

function renderArena() {
  drawArena(ctx, CW, CH, game.t);
  const baseY = 196;
  if (game.phase === 'battle' || game.phase === 'result') {
    drawRow(game.bA, 'a', baseY, 1, true);
    drawRow(game.bB, 'b', baseY, -1, true);
  } else {
    const bt = buildBattleTeam(game.team);
    for (let i = 0; i < bt.length; i++) {
      const x = unitSlotX('a', i, bt.length);
      drawTeamUnit(bt[i], game.team[i], x, baseY, 1, i === game.selected, false);
    }
    if (!game.team.length) {
      ctx.fillStyle = '#6a6488';
      ctx.font = '13px ui-monospace, monospace';
      ctx.textAlign = 'center';
      ctx.fillText(t('howto'), CW / 2, baseY - 8);
      ctx.textAlign = 'left';
    }
  }
  for (const p of game.popups) {
    ctx.globalAlpha = Math.max(0, p.life);
    ctx.fillStyle = p.color;
    ctx.font = '900 14px ui-monospace, monospace';
    ctx.textAlign = 'center';
    ctx.fillText(p.text, p.x, p.y);
    ctx.textAlign = 'left';
  }
  ctx.globalAlpha = 1;
}

function drawRow(bt, side, baseY, facing, withHp) {
  for (let i = 0; i < bt.length; i++) {
    const x = unitSlotX(side, i, bt.length);
    const b = bt[i];
    drawUnit(ctx, x, baseY, 38, b, b.color, facing, b.flash, game.t);
    if (withHp) {
      const f = Math.max(0, b.hp / b.maxHp);
      ctx.fillStyle = '#10131c';
      ctx.fillRect(x - 17, baseY + 6, 34, 6);
      ctx.fillStyle = f > 0.4 ? '#62d879' : '#ff5d5d';
      ctx.fillRect(x - 16, baseY + 7, 32 * f, 4);
    }
    drawStars(x, baseY - 30, b.star);
  }
}

function drawTeamUnit(b, member, x, baseY, facing, selected, withHp) {
  if (selected) {
    ctx.fillStyle = '#f4c85a';
    ctx.fillRect(x - 24, baseY - 40, 48, 52);
    ctx.fillStyle = '#1a1430';
    ctx.fillRect(x - 22, baseY - 38, 44, 48);
  }
  drawUnit(ctx, x, baseY, 42, b, b.color, facing, 0, game.t);
  drawStars(x, baseY - 34, member.star);
  // class color tag
  ctx.fillStyle = b.color;
  ctx.fillRect(x - 9, baseY + 6, 18, 4);
}

function drawStars(x, y, star) {
  for (let i = 0; i < star; i++) {
    ctx.fillStyle = '#f4c85a';
    ctx.fillRect(x - star * 5 + i * 10, y, 7, 7);
    ctx.fillStyle = '#fff2b0';
    ctx.fillRect(x - star * 5 + i * 10 + 1, y + 1, 3, 3);
  }
}

// ---- DOM: shop + synergy + hud ----------------------------------------
function renderShop() {
  const wrap = document.getElementById('shop-cards');
  wrap.innerHTML = '';
  const inShop = game.phase === 'shop';
  game.shop.forEach((offer, i) => {
    const card = document.createElement('button');
    card.className = 'shop-card';
    if (!offer) { card.className += ' empty'; card.disabled = true; wrap.appendChild(card); return; }
    const u = unitById(offer.id);
    const c = CLASSES[u.cls];
    card.style.borderColor = c.color;
    const cv = document.createElement('canvas');
    cv.width = 48; cv.height = 48;
    cv.className = 'shop-sprite';
    drawUnit(cv.getContext('2d'), 24, 40, 40, u, c.color, 1, 0, 0);
    card.appendChild(cv);
    card.insertAdjacentHTML('beforeend',
      `<div class="sc-name">${tUnit(u.id)}</div>` +
      `<div class="sc-stat">⚔${u.atk} ♥${u.hp}</div>` +
      `<div class="sc-cost" style="color:${c.color}">◆${u.tier} · ${tCls(u.cls)}</div>`);
    card.disabled = !inShop || game.gold < u.tier;
    card.onclick = () => buyUnit(i);
    wrap.appendChild(card);
  });
  // synergy
  const syn = document.getElementById('synergy');
  const counts = classCounts(game.team);
  syn.innerHTML = '';
  for (const k of ['beast', 'mech', 'mage']) {
    const n = counts[k], active = n >= 4 ? 4 : n >= 2 ? 2 : 0;
    const chip = document.createElement('span');
    chip.className = 'syn-chip' + (active ? ' on' : '');
    chip.style.color = CLASSES[k].color;
    chip.textContent = `${tCls(k)} ${n}/${n >= 2 ? (n >= 4 ? 4 : 4) : 2}`;
    syn.appendChild(chip);
  }
  document.getElementById('btn-reroll').disabled = !inShop || game.gold < REROLL_COST;
  document.getElementById('btn-sell').disabled = !inShop || game.selected < 0;
  document.getElementById('btn-battle').disabled = !inShop || !game.team.length;
}

function updateHud() {
  document.getElementById('hud-round').textContent = `${t('round')} ${game.round}`;
  document.getElementById('hud-gold').textContent = `◆ ${game.gold}`;
  document.getElementById('hud-lives').textContent = `♥ ${game.lives}`;
  document.getElementById('hud-cap').textContent = `${game.team.length}/${teamCap(game.round)}`;
}

let toastTimer = 0;
function toast(msg) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add('hidden'), 1500);
}

// ---- win / lose --------------------------------------------------------
function bestRound() { return +(localStorage.getItem(BEST_KEY) || 0); }

// ---- screens / overlays -----------------------------------------------
function showScreen(id) {
  document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
  document.getElementById(id).classList.remove('hidden');
}
function showOverlay(id) { document.getElementById(id).classList.remove('hidden'); }
function hideAllOverlays() { document.querySelectorAll('.overlay').forEach(o => o.classList.add('hidden')); }
function overlaysClosed() { return document.querySelectorAll('.overlay:not(.hidden)').length === 0; }

function startGame() {
  newGame();
  hideAllOverlays();
  showScreen('screen-game');
  renderShop(); renderArena(); updateHud();
}
function gotoTitle() {
  hideAllOverlays();
  document.getElementById('title-best').textContent = t('bestRound', bestRound());
  showScreen('screen-title');
}
function togglePause() {
  const o = document.getElementById('overlay-pause');
  if (o.classList.contains('hidden')) showOverlay('overlay-pause');
  else hideAllOverlays();
}

// ---- input -------------------------------------------------------------
canvas.addEventListener('pointerdown', e => {
  if (game.phase !== 'shop' || !overlaysClosed()) return;
  const r = canvas.getBoundingClientRect();
  const x = (e.clientX - r.left) / r.width * CW;
  const count = game.team.length;
  let hit = -1, bestD = 34;
  for (let i = 0; i < count; i++) {
    const d = Math.abs(unitSlotX('a', i, count) - x);
    if (d < bestD) { bestD = d; hit = i; }
  }
  if (hit < 0) { game.selected = -1; }
  else if (game.selected < 0) { game.selected = hit; }
  else if (game.selected === hit) { game.selected = -1; }
  else {
    const tmp = game.team[game.selected];
    game.team[game.selected] = game.team[hit];
    game.team[hit] = tmp;
    game.selected = -1;
  }
  renderShop(); renderArena();
});

addEventListener('keydown', e => {
  if (document.getElementById('screen-game').classList.contains('hidden')) return;
  if (e.key === 'Escape') togglePause();
});

document.getElementById('btn-play').onclick = startGame;
document.getElementById('btn-reroll').onclick = reroll;
document.getElementById('btn-sell').onclick = sellSelected;
document.getElementById('btn-battle').onclick = startBattle;
document.getElementById('btn-pause').onclick = togglePause;
document.getElementById('btn-pause-resume').onclick = togglePause;
document.getElementById('btn-pause-restart').onclick = startGame;
document.getElementById('btn-pause-menu').onclick = gotoTitle;
document.getElementById('btn-next').onclick = nextRound;
document.getElementById('btn-over-again').onclick = startGame;
document.getElementById('btn-over-menu').onclick = gotoTitle;
setupLanguageToggle(() => {
  document.getElementById('title-best').textContent = t('bestRound', bestRound());
  if (game) { renderShop(); updateHud(); }
});

// ---- loop --------------------------------------------------------------
function loop(now) {
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  if (game && !document.getElementById('screen-game').classList.contains('hidden')) {
    if (overlaysClosed()) update(dt);
    renderArena();
  }
  requestAnimationFrame(loop);
}

gotoTitle();
requestAnimationFrame(loop);
