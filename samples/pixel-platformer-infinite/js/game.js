class PlatformGame {
  constructor() {
    this.canvas = document.getElementById('game');
    this.ctx = this.canvas.getContext('2d');
    this.scale = 1;
    this.keys = {};
    this.hold = {};
    this.state = 'playing';
    this.stage = 1;
    this.level = 1;
    this.xp = 0;
    this.coins = 0;
    this.skillLevels = {};
    this.particles = [];
    this.popups = [];
    this.platforms = [];
    this.hazards = [];
    this.enemies = [];
    this.pickups = [];
    this.portal = null;
    this.cameraX = 0;
    this.last = 0;
    this.bind();
    this.resize();
    setupLanguageToggle(() => this.refreshLanguage());
    this.newRun();
    requestAnimationFrame(t => this.loop(t));
  }

  bind() {
    window.addEventListener('resize', () => this.resize());
    window.addEventListener('orientationchange', () => this.resize());
    document.addEventListener('keydown', e => {
      const k = e.key.toLowerCase();
      if (['arrowleft','arrowright','arrowup','arrowdown',' ','a','d','w','j','k','l','shift'].includes(k)) e.preventDefault();
      this.keys[k] = true;
      if (k === 'j' || k === ' ') this.jump();
      if (k === 'k') this.attack();
      if (k === 'l' || k === 'shift') this.dash();
    });
    document.addEventListener('keyup', e => { this.keys[e.key.toLowerCase()] = false; });
    document.getElementById('restart').onclick = () => this.newRun();
    document.getElementById('next-stage').onclick = () => this.nextStage();
    document.querySelectorAll('[data-hold]').forEach(btn => {
      const name = btn.dataset.hold;
      const on = e => { e.preventDefault(); this.hold[name] = true; btn.classList.add('active'); if (name === 'jump') this.jump(); };
      const off = e => { e.preventDefault(); this.hold[name] = false; btn.classList.remove('active'); };
      btn.addEventListener('pointerdown', on);
      btn.addEventListener('pointerup', off);
      btn.addEventListener('pointercancel', off);
      btn.addEventListener('pointerleave', off);
    });
    document.querySelectorAll('[data-tap]').forEach(btn => {
      btn.addEventListener('pointerdown', e => {
        e.preventDefault();
        btn.classList.add('active');
        if (btn.dataset.tap === 'attack') this.attack();
        if (btn.dataset.tap === 'dash') this.dash();
      });
      btn.addEventListener('pointerup', () => btn.classList.remove('active'));
      btn.addEventListener('pointercancel', () => btn.classList.remove('active'));
    });
  }

  resize() {
    const rect = this.canvas.getBoundingClientRect();
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    this.canvas.width = Math.max(1, Math.floor(rect.width * dpr));
    this.canvas.height = Math.max(1, Math.floor(rect.height * dpr));
    this.ctx.setTransform(this.canvas.width / VIEW_W, 0, 0, this.canvas.height / VIEW_H, 0, 0);
  }

  basePlayer() {
    return {
      x: 70, y: 462, prevX: 70, prevY: 462, vx: 0, vy: 0, w: 24, h: 38, facing: 1,
      hp: 100, maxHp: 100, attack: 16, crit: 0.04, guard: 0, magnet: 70,
      rewardMul: 1, grounded: false, jumps: 0, maxJumps: 1, jumpPower: 12.4,
      dashCooldown: 0, dashCooldownMax: 70, invuln: 0, attackCooldown: 0, attackCooldownMax: 28,
      attackPulse: 0,
    };
  }

  newRun() {
    this.state = 'playing';
    this.stage = 1;
    this.level = 1;
    this.xp = 0;
    this.coins = 0;
    this.skillLevels = {};
    this.player = this.basePlayer();
    this.generateStage();
    this.hidePanels();
    this.updateHud();
  }

  hidePanels() {
    document.getElementById('skill-panel').classList.add('hidden');
    document.getElementById('stage-panel').classList.add('hidden');
    document.getElementById('gameover-panel').classList.add('hidden');
  }

  generateStage() {
    const rand = mulberry32(9000 + this.stage * 131);
    const length = 1900 + this.stage * 170;
    this.stageLength = length;
    this.platforms = [
      { x: -160, y: 514, w: length + 520, h: 38, deck: true },
      { x: -120, y: 462, w: 520, h: 58, safe: true },
    ];
    this.hazards = [];
    this.enemies = [];
    this.pickups = [];
    let x = 390;
    let y = 430;
    while (x < length - 260) {
      const gap = 46 + rand() * Math.min(70, 42 + this.stage * 3);
      const w = 170 + rand() * 160;
      y += (rand() - 0.5) * 76;
      y = Math.max(260, Math.min(452, y));
      x += gap;
      this.platforms.push({ x, y, w, h: 24 });
      if (rand() < 0.45) this.hazards.push({ x: x + w * 0.35, y: y - 8, w: Math.min(70, w * 0.35), h: 10 });
      if (rand() < 0.64) this.spawnEnemy(x + w * (0.35 + rand() * 0.35), y, rand() < 0.38 ? 'drone' : 'crawler');
      for (let i = 0; i < 2 + Math.floor(rand() * 3); i++) {
        this.pickups.push({ x: x + 24 + rand() * (w - 48), y: y - 42 - rand() * 35, r: 7, kind: 'coin', taken: false });
      }
      x += w;
    }
    this.addSafetyDeck(length);
    this.platforms.push({ x: length - 220, y: 408, w: 260, h: 54 });
    this.portal = { x: length - 84, y: 350, w: 38, h: 58 };
    this.player.x = 70;
    this.player.y = this.platforms[0].y;
    this.player.prevX = this.player.x;
    this.player.prevY = this.player.y;
    this.player.vx = 0;
    this.player.vy = 0;
    this.player.grounded = true;
    this.player.jumps = 0;
    this.cameraX = 0;
    this.addPopup(85, 310, t('stagePopup', this.stage), COLORS.cyan);
  }

  addSafetyDeck(length) {
    for (let x = 360; x < length - 120; x += 238) {
      this.platforms.push({ x, y: 496, w: 252, h: 30, deck: true });
    }
  }

  spawnEnemy(x, floorY, kind) {
    const power = 1 + this.stage * 0.16;
    this.enemies.push({
      x, y: floorY, vx: kind === 'drone' ? 0.8 : 0.55, w: kind === 'drone' ? 26 : 30, h: kind === 'drone' ? 28 : 32,
      hp: Math.floor((kind === 'drone' ? 28 : 42) * power),
      maxHp: Math.floor((kind === 'drone' ? 28 : 42) * power),
      dmg: Math.floor((kind === 'drone' ? 10 : 14) * power),
      kind, alive: true, hitCd: 0,
    });
  }

  applySkills() {
    this.player.maxHp = 100;
    this.player.attack = 16;
    this.player.crit = 0.04;
    this.player.guard = 0;
    this.player.magnet = 70;
    this.player.maxJumps = 1;
    this.player.jumpPower = 12.4;
    this.player.dashCooldownMax = 70;
    this.player.attackCooldownMax = 28;
    this.player.rewardMul = 1;
    for (const skill of SKILLS) skill.apply(this.player, this.skillLevels[skill.id] || 0);
    this.player.hp = Math.min(this.player.hp, this.player.maxHp);
  }

  loop(t) {
    const dt = Math.min(2, (t - this.last) / 16.67 || 1);
    this.last = t;
    if (this.state === 'playing') this.update(dt);
    this.draw();
    requestAnimationFrame(n => this.loop(n));
  }

  update(dt) {
    const p = this.player;
    const left = this.keys.arrowleft || this.keys.a || this.hold.left;
    const right = this.keys.arrowright || this.keys.d || this.hold.right;
    const accel = p.grounded ? 0.95 : 0.58;
    if (left) { p.vx -= accel * dt; p.facing = -1; }
    if (right) { p.vx += accel * dt; p.facing = 1; }
    p.vx *= Math.pow(FRICTION, dt);
    p.vx = Math.max(-6.2, Math.min(6.2, p.vx));
    p.prevX = p.x;
    p.prevY = p.y;
    p.vy += GRAVITY * dt;
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    p.x = Math.max(18, Math.min(this.stageLength - 18, p.x));
    p.grounded = false;
    this.resolvePlatforms();
    if (p.y > VIEW_H + 120) this.damage(999);
    if (p.dashCooldown > 0) p.dashCooldown -= dt;
    if (p.attackCooldown > 0) p.attackCooldown -= dt;
    if (p.invuln > 0) p.invuln -= dt;
    if (p.attackPulse > 0) p.attackPulse -= dt;
    this.updateEnemies(dt);
    this.updatePickups(dt);
    this.updateFx(dt);
    if (Math.abs(p.x - this.portal.x) < 42 && Math.abs((p.y - p.h) - this.portal.y) < 72) this.completeStage();
    this.cameraX = Math.max(0, Math.min(this.stageLength - VIEW_W, p.x - VIEW_W * 0.38));
    this.updateHud();
  }

  resolvePlatforms() {
    const p = this.player;
    const previousBottom = Number.isFinite(p.prevY) ? p.prevY : p.y;
    const currentBottom = p.y;
    const left = p.x - p.w / 2;
    const right = p.x + p.w / 2;
    let bestLanding = null;

    for (const plat of this.platforms) {
      const overlapsX = right > plat.x && left < plat.x + plat.w;
      if (!overlapsX || p.vy < 0) continue;

      const crossedFromAbove = previousBottom <= plat.y + 6 && currentBottom >= plat.y;
      const closeCatch = currentBottom >= plat.y && currentBottom <= plat.y + Math.max(plat.h + 36, Math.abs(p.vy) + 18);
      if (crossedFromAbove || closeCatch) {
        if (!bestLanding || plat.y < bestLanding.y) bestLanding = plat;
      }
    }

    if (bestLanding) {
      p.y = bestLanding.y;
      p.vy = 0;
      p.grounded = true;
      p.jumps = 0;
    }
    for (const h of this.hazards) {
      if (p.x + p.w / 2 > h.x && p.x - p.w / 2 < h.x + h.w && p.y > h.y - 2 && p.y - p.h < h.y + h.h) {
        this.damage(18 + this.stage * 3);
        p.vy = -8;
      }
    }
  }

  updateEnemies(dt) {
    const p = this.player;
    for (const e of this.enemies) {
      if (!e.alive) continue;
      e.hitCd -= dt;
      if (e.kind === 'drone') {
        e.x += Math.sin((performance.now() / 600) + e.maxHp) * 0.45 * dt;
        e.y += Math.cos((performance.now() / 480) + e.maxHp) * 0.25 * dt;
      } else {
        e.x += e.vx * dt;
        const onPlat = this.platforms.find(pl => e.x > pl.x + 16 && e.x < pl.x + pl.w - 16 && Math.abs(e.y - pl.y) < 4);
        if (!onPlat || Math.random() < 0.01) e.vx *= -1;
      }
      const hit = Math.abs(p.x - e.x) < (p.w + e.w) / 2 && p.y > e.y - e.h && p.y - p.h < e.y;
      if (hit && e.hitCd <= 0) {
        this.damage(e.dmg);
        e.hitCd = 36;
      }
    }
  }

  updatePickups(dt) {
    const p = this.player;
    for (const item of this.pickups) {
      if (item.taken) continue;
      const dx = p.x - item.x;
      const dy = (p.y - p.h / 2) - item.y;
      const d = Math.hypot(dx, dy);
      if (d < p.magnet) {
        item.x += dx / Math.max(1, d) * 5.2 * dt;
        item.y += dy / Math.max(1, d) * 5.2 * dt;
      }
      if (d < 22) {
        item.taken = true;
        const gain = Math.floor(9 * p.rewardMul);
        this.coins += Math.floor(1 * p.rewardMul);
        this.gainXp(gain);
        this.burst(item.x, item.y, COLORS.gold, 6);
      }
    }
  }

  updateFx(dt) {
    for (const fx of this.particles) {
      fx.x += fx.vx * dt;
      fx.y += fx.vy * dt;
      fx.vy += 0.08 * dt;
      fx.life -= dt;
    }
    this.particles = this.particles.filter(fx => fx.life > 0);
    for (const p of this.popups) {
      p.y -= 0.45 * dt;
      p.life -= dt;
    }
    this.popups = this.popups.filter(p => p.life > 0);
  }

  jump() {
    if (this.state !== 'playing') return;
    const p = this.player;
    if (p.grounded || p.jumps < p.maxJumps) {
      p.vy = -p.jumpPower;
      p.jumps++;
      p.grounded = false;
      this.burst(p.x, p.y, COLORS.cyan, 5);
    }
  }

  dash() {
    if (this.state !== 'playing') return;
    const p = this.player;
    if (p.dashCooldown <= 0) {
      p.vx = p.facing * 14;
      p.invuln = 14;
      p.dashCooldown = p.dashCooldownMax;
      this.burst(p.x - p.facing * 16, p.y - 18, COLORS.violet, 12);
    }
  }

  attack() {
    if (this.state !== 'playing' || this.player.attackCooldown > 0) return;
    const p = this.player;
    p.attackCooldown = p.attackCooldownMax;
    p.attackPulse = 9;
    let hit = false;
    for (const e of this.enemies) {
      if (!e.alive) continue;
      const inFront = p.facing > 0 ? e.x > p.x - 8 && e.x < p.x + 72 : e.x < p.x + 8 && e.x > p.x - 72;
      const vertical = p.y > e.y - e.h - 18 && p.y - p.h < e.y + 10;
      if (inFront && vertical) {
        const crit = Math.random() < p.crit;
        const dmg = Math.floor(p.attack * (crit ? 2.2 : 1));
        e.hp -= dmg;
        hit = true;
        this.addPopup(e.x, e.y - e.h, String(dmg), crit ? COLORS.gold : COLORS.white);
        this.burst(e.x, e.y - e.h / 2, crit ? COLORS.gold : COLORS.red, crit ? 14 : 8);
        if (e.hp <= 0) this.killEnemy(e);
      }
    }
    if (!hit) this.burst(p.x + p.facing * 42, p.y - 18, COLORS.cyan, 3);
  }

  killEnemy(e) {
    e.alive = false;
    const reward = Math.floor((18 + this.stage * 2) * this.player.rewardMul);
    this.coins += Math.floor((2 + this.stage * 0.25) * this.player.rewardMul);
    this.gainXp(reward);
    this.addPopup(e.x, e.y - e.h, `+${reward}`, COLORS.green);
  }

  damage(raw) {
    const p = this.player;
    if (p.invuln > 0) return;
    const amount = Math.max(1, Math.floor(raw * (1 - p.guard)));
    p.hp -= amount;
    p.invuln = 42;
    this.addPopup(p.x, p.y - p.h, `-${amount}`, COLORS.red);
    this.burst(p.x, p.y - 18, COLORS.red, 12);
    if (p.hp <= 0) this.gameOver();
  }

  gainXp(amount) {
    this.xp += amount;
    while (this.xp >= xpForLevel(this.level)) {
      this.xp -= xpForLevel(this.level);
      this.level++;
      this.openSkillPanel();
    }
  }

  openSkillPanel() {
    this.state = 'skill';
    const box = document.getElementById('skill-choices');
    const picks = [...SKILLS].sort(() => Math.random() - 0.5).slice(0, 3);
    box.innerHTML = '';
    for (const skill of picks) {
      const lv = this.skillLevels[skill.id] || 0;
      const card = document.createElement('button');
      card.className = 'skill-card';
      const icon = document.createElement('canvas');
      icon.width = 54;
      icon.height = 54;
      icon.className = 'skill-icon';
      drawSkillIcon(icon.getContext('2d'), skill.color);
      card.appendChild(icon);
      card.insertAdjacentHTML('beforeend', `<div class="skill-name">${skill.name[currentLang]}</div><div class="skill-level">Lv.${lv} -> Lv.${lv + 1}</div><div class="skill-desc">${skill.desc[currentLang](lv + 1)}</div>`);
      card.onclick = () => {
        this.skillLevels[skill.id] = lv + 1;
        this.applySkills();
        this.state = 'playing';
        document.getElementById('skill-panel').classList.add('hidden');
        this.renderSkillbar();
      };
      box.appendChild(card);
    }
    document.getElementById('skill-panel').classList.remove('hidden');
  }

  completeStage() {
    this.state = 'stage';
    document.getElementById('stage-title').textContent = t('stageTitle', this.stage);
    document.getElementById('stage-copy').textContent = t('stageCopy', this.level);
    document.getElementById('stage-panel').classList.remove('hidden');
  }

  nextStage() {
    this.stage++;
    this.player.hp = Math.min(this.player.maxHp, this.player.hp + Math.floor(this.player.maxHp * 0.35));
    this.state = 'playing';
    document.getElementById('stage-panel').classList.add('hidden');
    this.generateStage();
  }

  gameOver() {
    this.state = 'over';
    document.getElementById('gameover-copy').textContent = t('gameoverCopy', this.stage, this.level, this.coins);
    document.getElementById('gameover-panel').classList.remove('hidden');
  }

  burst(x, y, color, count) {
    for (let i = 0; i < count; i++) {
      this.particles.push({ x, y, vx: (Math.random() - 0.5) * 4, vy: (Math.random() - 0.8) * 4, life: 22 + Math.random() * 18, color });
    }
  }

  addPopup(x, y, text, color) {
    this.popups.push({ x, y, text, color, life: 56 });
  }

  updateHud() {
    document.getElementById('hud-stage').textContent = this.stage;
    document.getElementById('hud-level').textContent = this.level;
    document.getElementById('hud-coins').textContent = this.coins;
    document.getElementById('hp-fill').style.width = `${Math.max(0, this.player.hp / this.player.maxHp * 100)}%`;
    document.getElementById('hp-text').textContent = `${Math.max(0, Math.ceil(this.player.hp))}/${this.player.maxHp}`;
    document.getElementById('xp-fill').style.width = `${Math.min(100, this.xp / xpForLevel(this.level) * 100)}%`;
    document.getElementById('xp-text').textContent = `${this.xp}/${xpForLevel(this.level)}`;
  }

  renderSkillbar() {
    const bar = document.getElementById('skillbar');
    bar.innerHTML = '';
    for (const skill of SKILLS) {
      const lv = this.skillLevels[skill.id] || 0;
      if (!lv) continue;
      const item = document.createElement('div');
      item.className = 'mini-skill';
      item.style.background = skill.color;
      item.textContent = lv;
      item.title = skill.name[currentLang];
      bar.appendChild(item);
    }
  }

  refreshLanguage() {
    if (this.state === 'skill') this.openSkillPanel();
    if (this.state === 'stage') {
      document.getElementById('stage-title').textContent = t('stageTitle', this.stage);
      document.getElementById('stage-copy').textContent = t('stageCopy', this.level);
    }
    if (this.state === 'over') {
      document.getElementById('gameover-copy').textContent = t('gameoverCopy', this.stage, this.level, this.coins);
    }
    this.renderSkillbar();
  }

  draw() {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, VIEW_W, VIEW_H);
    this.drawBackground(ctx);
    ctx.save();
    ctx.translate(-Math.round(this.cameraX), 0);
    for (const plat of this.platforms) this.drawPlatform(ctx, plat);
    for (const h of this.hazards) this.drawHazard(ctx, h);
    this.drawPortal(ctx);
    for (const item of this.pickups) if (!item.taken) this.drawPickup(ctx, item);
    for (const e of this.enemies) if (e.alive) drawEnemy(ctx, e, 0);
    if (this.player.invuln <= 0 || Math.floor(this.player.invuln / 4) % 2 === 0) drawPixelHero(ctx, this.player.x, this.player.y, this.player.facing < 0, Math.max(0, this.player.attackPulse));
    for (const fx of this.particles) {
      ctx.fillStyle = fx.color;
      ctx.globalAlpha = Math.max(0, fx.life / 34);
      ctx.fillRect(Math.round(fx.x), Math.round(fx.y), 4, 4);
      ctx.globalAlpha = 1;
    }
    for (const p of this.popups) {
      ctx.fillStyle = p.color;
      ctx.font = '14px monospace';
      ctx.textAlign = 'center';
      ctx.fillText(p.text, p.x, p.y);
    }
    ctx.restore();
  }

  drawBackground(ctx) {
    const g = ctx.createLinearGradient(0, 0, 0, VIEW_H);
    g.addColorStop(0, COLORS.sky0);
    g.addColorStop(1, COLORS.sky1);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, VIEW_W, VIEW_H);
    for (let layer = 0; layer < 3; layer++) {
      const speed = 0.08 + layer * 0.08;
      const baseY = 260 + layer * 48;
      ctx.fillStyle = layer === 0 ? '#151d2d' : layer === 1 ? '#111827' : '#0d1320';
      for (let x = -120; x < VIEW_W + 180; x += 120) {
        const sx = x - (this.cameraX * speed % 120);
        ctx.fillRect(Math.round(sx), baseY - (x % 3) * 16, 96, 260);
        ctx.fillStyle = '#26354a';
        ctx.fillRect(Math.round(sx + 18), baseY + 18, 10, 4);
        ctx.fillRect(Math.round(sx + 52), baseY + 42, 16, 4);
        ctx.fillStyle = layer === 0 ? '#151d2d' : layer === 1 ? '#111827' : '#0d1320';
      }
    }
  }

  drawPlatform(ctx, p) {
    ctx.fillStyle = COLORS.metalTop;
    ctx.fillRect(p.x, p.y, p.w, 5);
    ctx.fillStyle = COLORS.metal;
    ctx.fillRect(p.x, p.y + 5, p.w, p.h - 5);
    ctx.fillStyle = '#151c27';
    for (let x = p.x + 8; x < p.x + p.w; x += 32) ctx.fillRect(x, p.y + 10, 16, 3);
  }

  drawHazard(ctx, h) {
    ctx.fillStyle = COLORS.red;
    for (let x = h.x; x < h.x + h.w; x += 12) {
      ctx.beginPath();
      ctx.moveTo(x, h.y + h.h);
      ctx.lineTo(x + 6, h.y - 8);
      ctx.lineTo(x + 12, h.y + h.h);
      ctx.fill();
    }
  }

  drawPortal(ctx) {
    const p = this.portal;
    ctx.fillStyle = '#101722';
    ctx.fillRect(p.x - 8, p.y, p.w + 16, p.h + 10);
    ctx.fillStyle = COLORS.violet;
    ctx.fillRect(p.x, p.y + 8, p.w, p.h);
    ctx.fillStyle = COLORS.cyan;
    ctx.fillRect(p.x + 8, p.y + 18, p.w - 16, p.h - 16);
  }

  drawPickup(ctx, item) {
    ctx.fillStyle = COLORS.gold;
    ctx.fillRect(Math.round(item.x - 5), Math.round(item.y - 5), 10, 10);
    ctx.fillStyle = '#fff0a6';
    ctx.fillRect(Math.round(item.x - 2), Math.round(item.y - 5), 4, 10);
  }
}

window.addEventListener('DOMContentLoaded', () => new PlatformGame());
