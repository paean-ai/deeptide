class IdleForge {
  constructor() {
    this.canvas = document.getElementById('forge');
    this.ctx = this.canvas.getContext('2d');
    this.tab = 'upgrades';
    this.floaters = [];
    this.last = performance.now();
    this.saveTimer = 0;
    this.state = this.load();
    this.applyOffline();
    this.bind();
    this.recalc();
    this.renderShop();
    setupLanguageToggle(() => {
      this.renderShop();
      this.updateHud();
    });
    this.loop(this.last);
  }

  defaultState() {
    return {
      dust: 0,
      alloy: 0,
      core: 0,
      prestige: 0,
      totalDust: 0,
      levels: {},
      lastSave: Date.now(),
    };
  }

  load() {
    try {
      const raw = localStorage.getItem(SAVE_KEY);
      if (!raw) return this.defaultState();
      return { ...this.defaultState(), ...JSON.parse(raw) };
    } catch {
      return this.defaultState();
    }
  }

  save() {
    this.state.lastSave = Date.now();
    localStorage.setItem(SAVE_KEY, JSON.stringify(this.state));
  }

  bind() {
    document.getElementById('mine').onclick = () => this.mine();
    document.getElementById('prestige-btn').onclick = () => this.prestigeReset();
    document.getElementById('reset-save').onclick = () => {
      if (!confirm(t('resetConfirm'))) return;
      localStorage.removeItem(SAVE_KEY);
      this.state = this.defaultState();
      this.recalc();
      this.renderShop();
      this.toast(t('resetDone'));
    };
    document.querySelectorAll('.tab').forEach(tab => {
      tab.onclick = () => {
        this.tab = tab.dataset.tab;
        document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t === tab));
        this.renderShop();
      };
    });
  }

  applyOffline() {
    const elapsed = Math.min(8 * 3600, Math.max(0, (Date.now() - (this.state.lastSave || Date.now())) / 1000));
    if (elapsed < 10) return;
    this.recalc();
    this.addResources(elapsed, true);
    this.toast(t('offline', Math.floor(elapsed / 60)));
  }

  recalc() {
    const s = this.state;
    s.clickPower = 1;
    s.clickMul = 1;
    s.globalMul = 1;
    s.autoMul = 1;
    s.alloyMul = 1;
    s.offlineMul = 1;
    s.prestigeMul = 1;
    s.dustPerSec = 0;
    s.alloyPerSec = 0;
    s.corePerSec = 0;
    for (const item of allItems()) item.effect(s);
    const prestigeBonus = 1 + s.prestige * 0.18 * s.prestigeMul;
    s.totalMul = s.globalMul * prestigeBonus;
    s.clickValue = s.clickPower * s.clickMul * s.totalMul;
    s.dustRate = s.dustPerSec * s.autoMul * s.totalMul;
    s.alloyRate = s.alloyPerSec * s.autoMul * s.totalMul * s.alloyMul;
    s.coreRate = s.corePerSec * s.autoMul * s.totalMul;
  }

  mine() {
    const gain = this.state.clickValue;
    this.state.dust += gain;
    this.state.totalDust += gain;
    this.floaters.push({ x: 160 + (Math.random() - 0.5) * 90, y: 176 + (Math.random() - 0.5) * 40, text: `+${format(gain)}`, life: 55, color: '#f4c656' });
    this.drawForge(true);
    this.updateHud();
  }

  addResources(seconds, offline) {
    const mul = offline ? this.state.offlineMul : 1;
    const dust = this.state.dustRate * seconds * mul;
    const alloy = this.state.alloyRate * seconds * mul;
    const core = this.state.coreRate * seconds * mul;
    this.state.dust += dust;
    this.state.alloy += alloy;
    this.state.core += core;
    this.state.totalDust += dust;
  }

  canBuy(item) {
    return this.state[item.resource] >= costFor(item, levelOf(this.state, item.id));
  }

  buy(item) {
    const lv = levelOf(this.state, item.id);
    const cost = costFor(item, lv);
    if (this.state[item.resource] < cost) return;
    this.state[item.resource] -= cost;
    this.state.levels[item.id] = lv + 1;
    this.recalc();
    this.renderShop();
    this.updateHud();
    this.toast(t('leveled', item.name[currentLang], lv + 1));
  }

  prestigeGain() {
    if (this.state.totalDust < 50000) return 0;
    return Math.floor(Math.sqrt(this.state.totalDust / 50000));
  }

  prestigeReset() {
    const gain = this.prestigeGain();
    if (gain <= 0) {
      this.toast(t('recastNeed'));
      return;
    }
    if (!confirm(t('recastConfirm', gain))) return;
    const keepRelics = {};
    for (const item of ITEMS.relics) keepRelics[item.id] = this.state.levels[item.id] || 0;
    this.state = {
      ...this.defaultState(),
      prestige: this.state.prestige + gain,
      levels: keepRelics,
    };
    this.recalc();
    this.renderShop();
    this.updateHud();
    this.save();
    this.toast(t('recastGain', gain));
  }

  renderShop() {
    const cards = document.getElementById('cards');
    cards.innerHTML = '';
    for (const item of ITEMS[this.tab]) {
      const lv = levelOf(this.state, item.id);
      const cost = costFor(item, lv);
      const card = document.createElement('article');
      card.className = 'card';
      const icon = document.createElement('canvas');
      icon.width = 54;
      icon.height = 54;
      icon.className = 'icon';
      this.drawIcon(icon.getContext('2d'), item.color, lv);
      const meta = document.createElement('div');
      meta.className = 'meta';
      meta.innerHTML = `<h2>${item.name[currentLang]}</h2><p>${item.desc[currentLang](lv)}</p><span class="level">Lv.${lv}</span>`;
      const buy = document.createElement('button');
      buy.className = 'buy';
      buy.innerHTML = `${format(cost)}<br>${this.labelFor(item.resource)}`;
      buy.disabled = !this.canBuy(item);
      buy.onclick = () => this.buy(item);
      card.append(icon, meta, buy);
      cards.appendChild(card);
    }
  }

  labelFor(resource) {
    return { dust: t('costDust'), alloy: t('costAlloy'), core: t('costCore') }[resource] || resource;
  }

  loop(now) {
    const dt = Math.min(0.2, (now - this.last) / 1000 || 0);
    this.last = now;
    this.addResources(dt, false);
    this.saveTimer += dt;
    if (this.saveTimer > 3) {
      this.saveTimer = 0;
      this.save();
    }
    for (const f of this.floaters) {
      f.y -= 28 * dt;
      f.life -= 60 * dt;
    }
    this.floaters = this.floaters.filter(f => f.life > 0);
    this.drawForge(false);
    this.updateHud();
    requestAnimationFrame(t => this.loop(t));
  }

  updateHud() {
    document.getElementById('dust').textContent = format(this.state.dust);
    document.getElementById('alloy').textContent = format(this.state.alloy);
    document.getElementById('core').textContent = format(this.state.core);
    document.getElementById('prestige').textContent = format(this.state.prestige);
    document.getElementById('multiplier').textContent = `x${format(this.state.totalMul)}`;
    document.getElementById('click-rate').textContent = `${t('click')} +${format(this.state.clickValue)}`;
    document.getElementById('auto-rate').textContent = `${t('auto')} ${format(this.state.dustRate)}/s`;
    const gain = this.prestigeGain();
    const btn = document.getElementById('prestige-btn');
    btn.disabled = gain <= 0;
    btn.textContent = gain > 0 ? `${t('recast')} +${gain}` : t('recast');
    document.getElementById('prestige-copy').textContent = gain > 0
      ? t('prestigeReady', gain)
      : t('prestigeLocked', format(this.state.totalDust));
    if (Math.floor(performance.now() / 500) % 2 === 0) this.refreshBuyStates();
  }

  refreshBuyStates() {
    const buttons = document.querySelectorAll('.buy');
    ITEMS[this.tab].forEach((item, i) => {
      if (buttons[i]) buttons[i].disabled = !this.canBuy(item);
    });
  }

  drawForge(active) {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, 320, 320);
    ctx.fillStyle = '#080b12';
    ctx.fillRect(0, 0, 320, 320);
    for (let y = 0; y < 320; y += 16) {
      ctx.fillStyle = y % 32 === 0 ? '#0f1724' : '#0b111b';
      ctx.fillRect(0, y, 320, 8);
    }
    const t = performance.now() / 1000;
    ctx.fillStyle = '#182337';
    ctx.fillRect(48, 210, 224, 58);
    ctx.fillStyle = '#38465c';
    ctx.fillRect(64, 194, 192, 26);
    ctx.fillStyle = '#101722';
    ctx.fillRect(92, 70, 136, 136);
    ctx.fillStyle = active ? '#f4c656' : '#aa7dff';
    ctx.fillRect(110, 88, 100, 100);
    ctx.fillStyle = '#67e7ff';
    const pulse = Math.floor(Math.sin(t * 5) * 6);
    ctx.fillRect(128 - pulse / 2, 106 - pulse / 2, 64 + pulse, 64 + pulse);
    ctx.fillStyle = '#071018';
    ctx.fillRect(140, 118, 40, 40);
    ctx.fillStyle = '#51d889';
    ctx.fillRect(148, 126, 24, 24);
    ctx.fillStyle = '#26364c';
    for (let i = 0; i < 8; i++) {
      const a = t * 0.8 + i * Math.PI / 4;
      const x = 160 + Math.cos(a) * 108;
      const y = 142 + Math.sin(a) * 78;
      ctx.fillRect(Math.round(x), Math.round(y), 10, 10);
    }
    for (const f of this.floaters) {
      ctx.globalAlpha = Math.max(0, f.life / 55);
      ctx.fillStyle = f.color;
      ctx.font = '14px monospace';
      ctx.textAlign = 'center';
      ctx.fillText(f.text, f.x, f.y);
      ctx.globalAlpha = 1;
    }
  }

  drawIcon(ctx, color, lv) {
    ctx.fillStyle = '#080b12';
    ctx.fillRect(0, 0, 54, 54);
    ctx.fillStyle = color;
    const offset = lv % 5;
    ctx.fillRect(12, 12, 30, 30);
    ctx.fillStyle = '#edf4ff';
    ctx.fillRect(18 + offset, 18, 6, 6);
    ctx.fillRect(30 - offset, 30, 6, 6);
    ctx.fillStyle = 'rgba(0,0,0,0.35)';
    ctx.fillRect(8, 42, 38, 4);
  }

  toast(text) {
    const el = document.getElementById('toast');
    el.textContent = text;
    el.classList.remove('hidden');
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => el.classList.add('hidden'), 1800);
  }
}

window.addEventListener('DOMContentLoaded', () => new IdleForge());
