// ==================== 游戏主逻辑 ====================
const META_KEY = 'pixel-roguelike-profile-v1';
const META_UPGRADES = [
  {
    id: 'vitality',
    name: { en: 'Account Vitality', zh: '账户生命' },
    desc: { en: lv => `Start Max HP +${lv * 10}`, zh: lv => `开局最大生命 +${lv * 10}` },
    max: 10,
    cost: lv => 35 + lv * 25,
    apply: (p, lv) => {
      const add = lv * 10;
      p.maxHP += add;
      p.hp += add;
    },
  },
  {
    id: 'power',
    name: { en: 'Account Power', zh: '账户威力' },
    desc: { en: lv => `Start damage +${lv * 2}`, zh: lv => `开局伤害 +${lv * 2}` },
    max: 10,
    cost: lv => 45 + lv * 30,
    apply: (p, lv) => {
      p.baseAttack += lv * 2;
      p.baseRangedAttack += lv * 2;
    },
  },
  {
    id: 'recovery',
    name: { en: 'Account Recovery', zh: '账户恢复' },
    desc: { en: lv => `Passive regen +${(lv * 0.5).toFixed(1)}/s`, zh: lv => `被动回血 +${(lv * 0.5).toFixed(1)}/秒` },
    max: 8,
    cost: lv => 50 + lv * 35,
    apply: (p, lv) => { p._metaRegen = lv * 0.5; },
  },
  {
    id: 'reach',
    name: { en: 'Account Reach', zh: '账户拾取' },
    desc: { en: lv => `Pickup range +${lv * 8}`, zh: lv => `拾取范围 +${lv * 8}` },
    max: 8,
    cost: lv => 30 + lv * 25,
    apply: (p, lv) => { p._metaMagnetBonus = lv * 8; },
  },
];

function defaultProfile() {
  return {
    essence: 0,
    totalKills: 0,
    bestWave: 1,
    bestLevel: 1,
    runs: 0,
    upgrades: Object.fromEntries(META_UPGRADES.map(u => [u.id, 0])),
  };
}

function loadProfile() {
  try {
    const raw = localStorage.getItem(META_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    const base = defaultProfile();
    return {
      ...base,
      ...parsed,
      upgrades: { ...base.upgrades, ...(parsed.upgrades || {}) },
    };
  } catch (_) {
    return defaultProfile();
  }
}

function saveProfile(profile) {
  localStorage.setItem(META_KEY, JSON.stringify(profile));
}

class Game {
  constructor() {
    this.canvas = document.getElementById('gameCanvas');
    this.renderer = new Renderer(this.canvas);
    this.ctx = this.renderer.ctx;
    this.particles = new ParticleSystem();

    this.player = null;
    this.skills = null;
    this.enemies = [];
    this.orbs = [];
    this.projectiles = [];
    this.minions = [];
    this.novas = [];
    this.effects = [];

    this.keys = {};

    this.state = 'playing'; // playing | paused | levelup | gameover
    this.prevState = 'playing';
    this.profile = loadProfile();
    this.runRewardClaimed = false;
    this.lastTime = 0;
    this.spawnTimer = 0;
    this.spawnCount = 0;
    this.waveEnemyCount = 0;
    this.waveSpawned = 0;
    this.waveDone = false;
    this.bossSpawned = false;
    this.damageNumbers = [];
    this.waveText = null;
    this.bossText = null;
    this.levelUpText = null;
    this.moveTarget = null;
    this.shakeTimer = 0;
    this.shakeIntensity = 0;
    this.frameCount = 0;

    this.setupInput();
    this.setupResize();
    this.initGame();
    setupLanguageToggle(() => this.refreshLanguage());
    this.refreshLanguage();
    this.loop(0);
  }

  setupResize() {
    const resize = () => {
      const rect = this.canvas.getBoundingClientRect();
      this.renderer.resize(rect.width || CANVAS_W, rect.height || CANVAS_H);
      if (this.camera) {
        this.camera.screenWidth = this.renderer.viewW;
        this.camera.screenHeight = this.renderer.viewH;
        this.camera.follow(this.player);
      }
    };
    window.addEventListener('resize', resize);
    window.addEventListener('orientationchange', resize);
    this.resizeCanvas = resize;
  }

  initGame() {
    this.profile = loadProfile();
    this.resizeCanvas();
    this.camera = new Camera(WORLD_W, WORLD_H, this.renderer.viewW, this.renderer.viewH);
    this.tilemap = new Tilemap(Math.ceil(WORLD_W / TILE_SIZE), Math.ceil(WORLD_H / TILE_SIZE));
    this.player = new Player(WORLD_W / 2, WORLD_H / 2);
    this.applyMetaBonuses();
    this.skills = new SkillSystem(this.player);
    this.enemies = [];
    this.orbs = [];
    this.projectiles = [];
    this.minions = [];
    this.particles = new ParticleSystem();
    this.novas = [];
    this.effects = [];

    this.state = 'playing';
    this.prevState = 'playing';
    this.runRewardClaimed = false;
    this.spawnTimer = 0;
    this.spawnCount = 0;
    this.waveSpawned = 0;
    this.waveDone = false;
    this.bossSpawned = false;
    this.damageNumbers = [];
    this.waveText = null;
    this.bossText = null;
    this.levelUpText = null;
    this.moveTarget = null;
    this.shakeTimer = 0;
    this.shakeIntensity = 0;
    this.frameCount = 0;

    this.startWave();

    document.getElementById('levelup-panel').classList.add('hidden');
    document.getElementById('gameover-panel').classList.add('hidden');
    document.getElementById('pause-panel').classList.add('hidden');
    document.getElementById('skillbar-icons').innerHTML = '';
    this.updateHUD();
  }

  applyMetaBonuses() {
    for (const upgrade of META_UPGRADES) {
      const lv = this.profile.upgrades[upgrade.id] || 0;
      if (lv > 0) upgrade.apply(this.player, lv);
    }
    this.player.hp = this.player.maxHP;
  }

  startWave() {
    const cfg = waveConfig(this.player.wave);
    this.waveEnemyCount = cfg.count;
    this.waveSpawned = 0;
    this.waveDone = false;
    this.bossSpawned = false;
    this.spawnTimer = 0;

    // wave 过渡文字
    this.waveText = { text: t('waveText', this.player.wave), alpha: 1, timer: 120 };
  }

  spawnEnemy() {
    const cfg = waveConfig(this.player.wave);
    const type = cfg.types[Math.floor(Math.random() * cfg.types.length)];
    const waveMul = 1 + (this.player.wave - 1) * 0.2;

    // 从相机视口边缘生成（世界坐标）
    let x, y;
    const side = Math.floor(Math.random() * 4);
    const pad = 60;
    const margin = 80;
    switch (side) {
      case 0: x = this.camera.worldLeft + Math.random() * this.renderer.viewW; y = this.camera.worldTop - pad; break;
      case 1: x = this.camera.worldRight + pad; y = this.camera.worldTop + Math.random() * this.renderer.viewH; break;
      case 2: x = this.camera.worldLeft + Math.random() * this.renderer.viewW; y = this.camera.worldBottom + pad; break;
      case 3: x = this.camera.worldLeft - pad; y = this.camera.worldTop + Math.random() * this.renderer.viewH; break;
    }
    // 确保在世界范围内
    x = Math.max(margin, Math.min(WORLD_W - margin, x));
    y = Math.max(margin, Math.min(WORLD_H - margin, y));

    const enemy = new Enemy(x, y, type, waveMul);
    this.enemies.push(enemy);
  }

  spawnBoss() {
    const cfg = waveConfig(this.player.wave);
    const waveMul = 1 + (this.player.wave - 1) * 0.2;
    const x = this.camera.worldLeft + Math.random() * this.renderer.viewW;
    const y = this.camera.worldTop - 30;
    const boss = new Enemy(x, y, 'boss', waveMul);
    if (cfg.bossHp) {
      boss.hp = cfg.bossHp;
      boss.maxHp = cfg.bossHp;
    }
    this.enemies.push(boss);
  }

  addDamageNumber(x, y, value, color) {
    this.damageNumbers.push({
      x, y,
      text: String(Math.floor(value)),
      color: color || '#ffffff',
      alpha: 1,
      life: 40,
      vy: -1.8,
    });
  }

  addHealNumber(x, y, value) {
    this.damageNumbers.push({
      x, y,
      text: `+${Math.floor(value)}`,
      color: '#43d17a',
      alpha: 1,
      life: 34,
      vy: -1.4,
    });
  }

  applyLifeSteal(damage, x, y, scale = 1) {
    const rate = this.player.lifeSteal;
    if (rate <= 0 || damage <= 0) return 0;
    const amount = Math.max(1, Math.floor(damage * rate * scale));
    const healed = this.player.heal(amount);
    if (healed > 0) {
      this.addHealNumber(x, y, healed);
      if (this.frameCount % 3 === 0) {
        this.particles.emit(this.player.x, this.player.y, 3, '#e05243', 2, 10, 2, { drag: 0.9 });
      }
    }
    return healed;
  }

  addRunReward() {
    if (this.runRewardClaimed) return 0;
    this.runRewardClaimed = true;
    const reward = Math.floor(this.player.kills / 4) + Math.max(0, this.player.wave - 1) * 3 + Math.max(0, this.player.level - 1);
    this.profile.essence += reward;
    this.profile.totalKills += this.player.kills;
    this.profile.bestWave = Math.max(this.profile.bestWave || 1, this.player.wave);
    this.profile.bestLevel = Math.max(this.profile.bestLevel || 1, this.player.level);
    this.profile.runs += 1;
    saveProfile(this.profile);
    return reward;
  }

  metaName(upgrade) {
    return upgrade.name[currentLang] || upgrade.name.en;
  }

  metaDesc(upgrade, level) {
    const desc = upgrade.desc[currentLang] || upgrade.desc.en;
    return desc(level);
  }

  renderMetaUpgrades(containerId) {
    const container = document.getElementById(containerId);
    if (!container) return;
    container.innerHTML = '';
    for (const upgrade of META_UPGRADES) {
      const lv = this.profile.upgrades[upgrade.id] || 0;
      const maxed = lv >= upgrade.max;
      const cost = maxed ? 0 : upgrade.cost(lv);
      const card = document.createElement('button');
      card.className = 'meta-upgrade';
      card.type = 'button';
      card.disabled = maxed || this.profile.essence < cost;
      card.innerHTML = `
        <b>${this.metaName(upgrade)} Lv.${lv}/${upgrade.max}</b>
        <span>${this.metaDesc(upgrade, lv)}</span>
        <strong>${maxed ? t('maxed') : t('buyUpgrade', cost)}</strong>
      `;
      card.onclick = () => this.buyMetaUpgrade(upgrade.id);
      container.appendChild(card);
    }
  }

  buyMetaUpgrade(id) {
    const upgrade = META_UPGRADES.find(u => u.id === id);
    if (!upgrade) return;
    const lv = this.profile.upgrades[id] || 0;
    if (lv >= upgrade.max) return;
    const cost = upgrade.cost(lv);
    if (this.profile.essence < cost) return;
    this.profile.essence -= cost;
    this.profile.upgrades[id] = lv + 1;
    saveProfile(this.profile);
    this.refreshMetaPanels();
  }

  refreshMetaPanels() {
    const text = t('accountInfo', this.profile.essence, this.profile.bestWave || 1);
    const pauseInfo = document.getElementById('pause-info');
    if (pauseInfo) pauseInfo.textContent = text;
    const gameoverMeta = document.getElementById('gameover-meta');
    if (gameoverMeta && this.state === 'gameover') gameoverMeta.textContent = text;
    this.renderMetaUpgrades('pause-upgrades');
    this.renderMetaUpgrades('gameover-upgrades');
  }

  // 玩家近战攻击（范围伤害，击中所有范围内敌人）
  doMeleeAttack() {
    const p = this.player;
    const range = p.attackRange * p._areaMul + 10;
    const dmg = p.attack;
    const crit = Math.random() < p.critChance;
    const finalDmg = crit ? Math.floor(dmg * (1 + p.critDamage)) : dmg;
    let hitAny = false;

    for (const e of this.enemies) {
      if (!e.alive) continue;
      const dx = e.x - p.x;
      const dy = e.y - p.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < range + e.size) {
        hitAny = true;
        const killed = e.takeDamage(finalDmg, p);
        this.addDamageNumber(e.x, e.y - e.size - 5, finalDmg, crit ? '#f1c40f' : '#ffffff');
        this.particles.emitHit(e.x, e.y);
        if (crit) {
          this.particles.emit(e.x, e.y, 8, '#f1c40f', 4, 15, 4);
        }
        // 范围扩大系数越高，近战范围越大
        if (p._areaMul > 1 && dist > p.attackRange * 0.6) {
          this.particles.emit(e.x, e.y, 4, '#e67e22', 2, 10, 3);
        }
        this.applyLifeSteal(finalDmg, p.x, p.y - p.size - 8);
        // 击杀处理
        if (killed) {
          this.onEnemyKilled(e);
        }
      }
    }

    if (hitAny) {
      // 近战特效 — 圆弧斩
      p.attackPulse = 10;
      this.effects.push({ kind: 'slash', x: p.x, y: p.y, range, facing: p.facing, crit, life: 12, maxLife: 12 });
      this.particles.emitSlash(p.x, p.y, p.facing, range, crit);
      this.particles.emit(p.x + (p.facing === 'left' ? -range : range) * 0.65, p.y, 5, '#f2c14e', 3, 10, 3);
    }
  }

  // 远程射击
  doRangedAttack(tx, ty) {
    const p = this.player;
    const dmg = p.rangedAttack;
    const crit = Math.random() < p.critChance;
    const finalDmg = crit ? Math.floor(dmg * (1 + p.critDamage)) : dmg;

    const proj = new Projectile(
      p.x, p.y, tx, ty, finalDmg,
      p.projectileSpeed,
      crit ? '#f1c40f' : '#85c1e9',
      p._bounceCount
    );
    proj.crit = crit;
    this.projectiles.push(proj);
    this.effects.push({ kind: 'beam', x: p.x, y: p.y, tx, ty, color: crit ? '#f3f7ff' : '#a9e8ff', life: 6, maxLife: 6 });
  }

  checkProjectileHits() {
    for (const proj of this.projectiles) {
      if (!proj.alive) continue;
      for (const e of this.enemies) {
        if (!e.alive || proj.hitTargets.has(e)) continue;
        const dx = e.x - proj.x;
        const dy = e.y - proj.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < e.size + proj.size + 4) {
          proj.hitTargets.add(e);
          const killed = e.takeDamage(proj.damage, this.player);
          this.addDamageNumber(e.x, e.y - e.size - 5, proj.damage, '#85c1e9');
          this.applyLifeSteal(proj.damage, this.player.x, this.player.y - this.player.size - 8, 0.8);
          this.particles.emitHit(e.x, e.y);
          this.effects.push({ kind: 'ring', x: e.x, y: e.y, radius: 22, color: proj.crit ? '#f3f7ff' : '#a9e8ff', alpha: 0.5, life: 10, maxLife: 10 });

          if (killed) {
            this.onEnemyKilled(e);
            if (proj.bounces > 0) {
              const next = this.findNearestEnemy(e);
              if (next) proj.bounceTo(next);
              else proj.alive = false;
            } else {
              proj.alive = false;
            }
          } else {
            if (proj.bounces > 0) {
              const next = this.findNearestEnemy(e);
              if (next) proj.bounceTo(next);
              else proj.alive = false;
            } else {
              proj.alive = false;
            }
          }
          break;
        }
      }
    }
    this.projectiles = this.projectiles.filter(p => p.alive);
  }

  findNearestEnemy(exclude) {
    let nearest = null;
    let nearDist = Infinity;
    for (const e of this.enemies) {
      if (!e.alive || e === exclude) continue;
      const d = Math.sqrt(
        (e.x - exclude.x) ** 2 + (e.y - exclude.y) ** 2
      );
      if (d < nearDist) {
        nearDist = d;
        nearest = e;
      }
    }
    return nearest;
  }

  onEnemyKilled(enemy) {
    this.player.kills++;
    this.particles.emitDeath(enemy.x, enemy.y, enemy.color);

    // 掉经验球
    const orbCount = Math.min(1 + Math.floor(enemy.expValue / 8), 5);
    for (let i = 0; i < orbCount; i++) {
      this.orbs.push(new ExpOrb(
        enemy.x + (Math.random() - 0.5) * 10,
        enemy.y + (Math.random() - 0.5) * 10,
        Math.floor(enemy.expValue / orbCount)
      ));
    }

    // 死亡爆炸
    if (this.player._novaDamage > 0) {
      this.triggerNova(enemy.x, enemy.y);
    }
  }

  triggerNova(x, y) {
    this.novas.push({ x, y, life: 15, maxLife: 15, damage: this.player._novaDamage });
    this.effects.push({ kind: 'ring', x, y, radius: 76, color: '#ff6b35', alpha: 0.7, life: 18, maxLife: 18 });
    this.particles.emit(x, y, 24, '#e05243', 5, 22, 5, { drag: 0.92, twinkle: true });

    // 伤害范围内敌人
    for (const e of this.enemies) {
      if (!e.alive) continue;
      const dx = e.x - x;
      const dy = e.y - y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 60) {
        const killed = e.takeDamage(this.player._novaDamage, this.player);
        this.addDamageNumber(e.x, e.y - e.size - 5, this.player._novaDamage, '#e74c3c');
        this.applyLifeSteal(this.player._novaDamage, this.player.x, this.player.y - this.player.size - 8, 0.35);
        if (killed) this.onEnemyKilled(e);
      }
    }
  }

  // 检查敌人攻击玩家
  checkEnemyAttacks() {
    for (const e of this.enemies) {
      if (!e.alive) continue;
      const result = e.update(this.player, this.enemies);
      if (result && result.attack) {
        const dmg = this.player.takeDamage(result.dmg);
        if (dmg === -1) {
          // 闪避！
          this.effects.push({ kind: 'ring', x: this.player.x, y: this.player.y, radius: 30, color: '#83d8ff', alpha: 0.45, life: 10, maxLife: 10 });
          this.particles.emit(this.player.x, this.player.y, 7, '#83d8ff', 3, 10, 3, { drag: 0.88 });
        } else if (dmg > 0) {
          this.particles.emit(this.player.x, this.player.y, 5, '#e05243', 2, 10, 3, { drag: 0.9 });
          this.player.knockbackX = (this.player.x - e.x) / 10;
          this.player.knockbackY = (this.player.y - e.y) / 10;

          // 荆棘反弹
          if (this.player._thorns > 0) {
            const thornDmg = Math.floor(dmg * this.player._thorns);
            e.takeDamage(thornDmg, this.player);
            this.addDamageNumber(e.x, e.y - e.size - 5, thornDmg, '#27ae60');
            this.applyLifeSteal(thornDmg, this.player.x, this.player.y - this.player.size - 8, 0.3);
            this.effects.push({ kind: 'ring', x: e.x, y: e.y, radius: 24, color: '#43d17a', alpha: 0.5, life: 10, maxLife: 10 });
            if (!e.alive) this.onEnemyKilled(e);
          }

          if (this.player.hp <= 0) {
            this.onPlayerDeath();
          }
        }
      } else if (result && result.burn) {
        this.addDamageNumber(e.x, e.y - e.size - 5, result.dmg, '#e67e22');
        this.applyLifeSteal(result.dmg, this.player.x, this.player.y - this.player.size - 8, 0.35);
        this.onEnemyKilled(e);
      }
    }
  }

  onPlayerDeath() {
    // 重生检查
    if (this.player._revives > 0) {
      this.player._revives--;
      this.player.hp = this.player.maxHP;
      this.player.invincible = 60;
      this.particles.emitLevelUp(this.player.x, this.player.y);
      this.effects.push({ kind: 'ring', x: this.player.x, y: this.player.y, radius: 72, color: '#f2c14e', alpha: 0.8, life: 18, maxLife: 18 });
      this.addHealNumber(this.player.x, this.player.y - this.player.size - 14, this.player.maxHP);
      this.updateHUD();
      return;
    }

    const reward = this.addRunReward();
    this.state = 'gameover';
    const panel = document.getElementById('gameover-panel');
    panel.querySelector('#gameover-info').textContent =
      t('gameoverInfo', this.player.level, this.player.wave, this.player.kills);
    this.refreshMetaPanels();
    document.getElementById('gameover-meta').textContent = `${t('runEarned', reward)} | ${t('accountInfo', this.profile.essence, this.profile.bestWave || 1)}`;
    panel.classList.remove('hidden');
  }

  // 升级选择
  showLevelUp() {
    this.state = 'levelup';
    // 升级特效文字
    this.levelUpText = { text: 'LEVEL UP!', alpha: 1, timer: 30, y: -10 };

    const panel = document.getElementById('levelup-panel');
    const container = document.getElementById('skill-choices');
    container.innerHTML = '';

    const choices = this.skills.getRandomChoices(3);

    for (const skill of choices) {
      const currentLevel = this.skills.getLevel(skill.id);
      const newLevel = currentLevel + 1;
      const card = document.createElement('div');
      card.className = 'skill-card';
      card.tabIndex = 0;
      card.innerHTML = `
        <div class="skill-icon" style="background:${skill.color}20; border:2px solid ${skill.color}"></div>
        <div class="skill-name">${skillName(skill.id)}</div>
        <div class="skill-level">${currentLevel > 0 ? `Lv.${currentLevel} → Lv.${newLevel}` : `Lv.${newLevel}`}</div>
        <div class="skill-desc">${skillDesc(skill.id, newLevel)}</div>
      `;
      const icon = card.querySelector('.skill-icon');
      icon.appendChild(this.createSkillGlyph(skill.id, skill.color, 42));
      card.onclick = () => {
        this.skills.addSkill(skill.id);
        panel.classList.add('hidden');
        this.state = 'playing';
        this.updateSkillBar();
        this.updateHUD();
        // 升级特效
        this.particles.emitLevelUp(this.player.x, this.player.y);
      };
      card.onkeydown = e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          card.click();
        }
      };
      container.appendChild(card);
    }

    panel.classList.remove('hidden');
  }

  togglePause(force) {
    if (this.state === 'levelup' || this.state === 'gameover') return;
    const shouldPause = typeof force === 'boolean' ? force : this.state === 'playing';
    if (shouldPause) {
      this.prevState = this.state;
      this.state = 'paused';
      this.keys = {};
      this.moveTarget = null;
      this.touch.attack = false;
      this.touch.skill = false;
      document.getElementById('pause-panel').classList.remove('hidden');
      this.refreshMetaPanels();
    } else if (this.state === 'paused') {
      this.state = 'playing';
      document.getElementById('pause-panel').classList.add('hidden');
    }
  }

  updateSkillBar() {
    const container = document.getElementById('skillbar-icons');
    container.innerHTML = '';
    for (const [id, lv] of Object.entries(this.skills.owned)) {
      const skill = this.skills.getSkill(id);
      if (!skill) continue;
      const el = document.createElement('div');
      el.className = 'skillbar-icon';
      el.style.background = skill.color + '30';
      el.style.borderColor = skill.color;
      el.appendChild(this.createSkillGlyph(skill.id, skill.color, 22));
      const lvEl = document.createElement('span');
      lvEl.className = 'slv';
      lvEl.textContent = lv;
      el.appendChild(lvEl);
      el.title = `${skillName(skill.id)} Lv.${lv}`;
      container.appendChild(el);
    }
  }

  createSkillGlyph(id, color, size) {
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    canvas.className = 'pixel-glyph';
    const ctx = canvas.getContext('2d');
    ctx.imageSmoothingEnabled = false;
    const glyph = SKILL_GLYPHS[id] || SKILL_GLYPHS.default;
    const cell = Math.floor(size / 9);
    const ox = Math.floor((size - glyph[0].length * cell) / 2);
    const oy = Math.floor((size - glyph.length * cell) / 2);
    ctx.fillStyle = 'rgba(0,0,0,0.55)';
    for (let y = 0; y < glyph.length; y++) {
      for (let x = 0; x < glyph[y].length; x++) {
        if (glyph[y][x] === '1') ctx.fillRect(ox + x * cell + 1, oy + y * cell + 1, cell, cell);
      }
    }
    ctx.fillStyle = color;
    for (let y = 0; y < glyph.length; y++) {
      for (let x = 0; x < glyph[y].length; x++) {
        if (glyph[y][x] === '1') ctx.fillRect(ox + x * cell, oy + y * cell, cell, cell);
      }
    }
    ctx.fillStyle = '#f3f7ff';
    ctx.fillRect(Math.floor(size * 0.42), Math.floor(size * 0.22), Math.max(1, cell - 1), Math.max(1, cell - 1));
    return canvas;
  }

  updateHUD() {
    const p = this.player;
    document.getElementById('hud-level').textContent = `Lv.${p.level}`;
    document.getElementById('hud-hp-fill').style.width = `${(p.hp / p.maxHP) * 100}%`;
    const reviveText = p._reviveMax > 0 ? ` | R${p._revives}` : '';
    document.getElementById('hud-hp-text').textContent = `${Math.ceil(p.hp)}/${p.maxHP}${reviveText}`;
    document.getElementById('hud-exp-fill').style.width = `${(p.exp / p.expToNext) * 100}%`;
    document.getElementById('hud-exp-text').textContent = `${p.exp}/${p.expToNext}`;
    document.getElementById('hud-wave').textContent = `${t('wave')} ${p.wave}`;
    document.getElementById('pause-btn').textContent = t('pause');
  }

  refreshLanguage() {
    this.updateHUD();
    this.updateSkillBar();
    if (this.state === 'levelup') this.showLevelUp();
    if (this.state === 'gameover') {
      document.getElementById('gameover-info').textContent =
        t('gameoverInfo', this.player.level, this.player.wave, this.player.kills);
    }
    this.refreshMetaPanels();
  }

  setupInput() {
    // 键盘
    document.addEventListener('keydown', e => {
      if (e.key === 'Escape' || e.key === 'p' || e.key === 'P') {
        e.preventDefault();
        this.togglePause();
        return;
      }
      this.keys[e.key] = true;
      if (e.key === 'r' || e.key === 'R') {
        if (this.state === 'gameover') this.initGame();
      }
    });
    document.addEventListener('keyup', e => {
      this.keys[e.key] = false;
    });

    // 鼠标 — 点击移动
    this.canvas.addEventListener('mousedown', e => {
      if (this.state !== 'playing') return;
      const rect = this.canvas.getBoundingClientRect();
      const scaleX = this.canvas.width / rect.width;
      const scaleY = this.canvas.height / rect.height;
      const sx = (e.clientX - rect.left) * scaleX;
      const sy = (e.clientY - rect.top) * scaleY;
      const world = this.camera.screenToWorld(sx, sy);
      this.moveTarget = { x: world.x, y: world.y };
    });

    this.canvas.addEventListener('contextmenu', e => e.preventDefault());

    // 触摸支持 — 保留点击移动作为备选
    this.canvas.addEventListener('touchstart', e => {
      if (this.state !== 'playing' || !e.touches.length) return;
      const touch = e.touches[0];
      // 判断是否有触屏控制（触摸设备才显示 tc 元素）
      const tc = document.getElementById('touch-controls');
      if (tc && tc.style.display !== 'none') return; // 触屏控制接管，canvas touch 不处理
      e.preventDefault();
      const rect = this.canvas.getBoundingClientRect();
      const scaleX = this.canvas.width / rect.width;
      const scaleY = this.canvas.height / rect.height;
      const sx = (touch.clientX - rect.left) * scaleX;
      const sy = (touch.clientY - rect.top) * scaleY;
      const world = this.camera.screenToWorld(sx, sy);
      this.moveTarget = { x: world.x, y: world.y };
    }, { passive: false });

    // 触摸摇杆 & 按钮
    this.touch = { joystick: { active: false, dx: 0, dy: 0, id: -1 }, attack: false, skill: false };

    const joystickArea = document.getElementById('tc-joystick-area');
    const joystickEl = document.getElementById('tc-joystick');
    const knobEl = document.getElementById('tc-joystick-knob');
    const btnAttack = document.getElementById('tc-btn-attack');
    const btnSkill = document.getElementById('tc-btn-skill');

    const getJoystickCenter = (el) => {
      const r = el.getBoundingClientRect();
      return { cx: r.left + r.width / 2, cy: r.top + r.height / 2, r: r.width / 2 };
    };

    const updateJoystick = (touch, cx, cy, maxR) => {
      const dx = touch.clientX - cx;
      const dy = touch.clientY - cy;
      const d = Math.sqrt(dx * dx + dy * dy);
      const clamp = Math.min(d, maxR) / (maxR || 1);
      const angle = Math.atan2(dy, dx);
      const nx = Math.cos(angle) * clamp;
      const ny = Math.sin(angle) * clamp;
      knobEl.style.transform = `translate(-50%, -50%) translate(${nx * maxR}px, ${ny * maxR}px)`;
      this.touch.joystick.dx = nx;
      this.touch.joystick.dy = ny;
    };

    if (joystickArea) {
      joystickArea.addEventListener('touchstart', e => {
        if (this.state !== 'playing') return;
        e.preventDefault();
        const touch = e.changedTouches[0];
        if (!touch) return;
        joystickEl.classList.remove('hidden');
        const center = getJoystickCenter(joystickEl);
        joystickEl.style.left = Math.max(10, Math.min(joystickArea.clientWidth - center.r * 2 - 10, touch.clientX - joystickArea.getBoundingClientRect().left - center.r)) + 'px';
        joystickEl.style.bottom = 'auto';
        joystickEl.style.top = Math.max(10, Math.min(joystickArea.clientHeight - center.r * 2 - 10, touch.clientY - joystickArea.getBoundingClientRect().top - center.r)) + 'px';
        // 重新计算中心
        const newCenter = getJoystickCenter(joystickEl);
        updateJoystick(touch, newCenter.cx, newCenter.cy, newCenter.r);
        this.touch.joystick.active = true;
        this.touch.joystick.id = touch.identifier;
      }, { passive: false });

      joystickArea.addEventListener('touchmove', e => {
        e.preventDefault();
        let touch = null;
        for (const t of e.changedTouches) {
          if (t.identifier === this.touch.joystick.id) { touch = t; break; }
        }
        if (!touch) return;
        const center = getJoystickCenter(joystickEl);
        updateJoystick(touch, center.cx, center.cy, center.r);
      }, { passive: false });

      joystickArea.addEventListener('touchend', e => {
        e.preventDefault();
        let wasActive = false;
        for (const t of e.changedTouches) {
          if (t.identifier === this.touch.joystick.id) { wasActive = true; break; }
        }
        if (!wasActive) return;
        joystickEl.classList.add('hidden');
        knobEl.style.transform = 'translate(-50%, -50%) translate(0px, 0px)';
        this.touch.joystick.active = false;
        this.touch.joystick.dx = 0;
        this.touch.joystick.dy = 0;
        this.touch.joystick.id = -1;
      }, { passive: false });

      joystickArea.addEventListener('touchcancel', e => {
        joystickEl.classList.add('hidden');
        knobEl.style.transform = 'translate(-50%, -50%) translate(0px, 0px)';
        this.touch.joystick.active = false;
        this.touch.joystick.dx = 0;
        this.touch.joystick.dy = 0;
        this.touch.joystick.id = -1;
      }, { passive: false });
    }

    if (btnAttack) {
      btnAttack.addEventListener('touchstart', e => { e.preventDefault(); this.touch.attack = true; }, { passive: false });
      btnAttack.addEventListener('touchend', e => { e.preventDefault(); this.touch.attack = false; }, { passive: false });
      btnAttack.addEventListener('touchcancel', e => { this.touch.attack = false; }, { passive: false });
    }

    if (btnSkill) {
      btnSkill.addEventListener('touchstart', e => { e.preventDefault(); this.touch.skill = true; }, { passive: false });
      btnSkill.addEventListener('touchend', e => { e.preventDefault(); this.touch.skill = false; }, { passive: false });
      btnSkill.addEventListener('touchcancel', e => { this.touch.skill = false; }, { passive: false });
    }

    document.getElementById('pause-btn').addEventListener('click', () => this.togglePause(true));
    document.getElementById('resume-btn').addEventListener('click', () => this.togglePause(false));
    document.getElementById('pause-restart-btn').addEventListener('click', () => {
      this.addRunReward();
      this.initGame();
    });

    // 重启按钮
    document.getElementById('restart-btn').addEventListener('click', () => {
      this.initGame();
    });
  }

  updateInput() {
    const p = this.player;
    p.dx = 0;
    p.dy = 0;

    // 触屏摇杆 — 优先于键盘
    if (this.touch && this.touch.joystick.active &&
        (Math.abs(this.touch.joystick.dx) > 0.15 || Math.abs(this.touch.joystick.dy) > 0.15)) {
      this.moveTarget = null;
      const mag = Math.sqrt(this.touch.joystick.dx ** 2 + this.touch.joystick.dy ** 2);
      p.dx = this.touch.joystick.dx / mag;
      p.dy = this.touch.joystick.dy / mag;
      return;
    }

    const keyHeld = this.keys['w'] || this.keys['W'] || this.keys['ArrowUp'] ||
                    this.keys['s'] || this.keys['S'] || this.keys['ArrowDown'] ||
                    this.keys['a'] || this.keys['A'] || this.keys['ArrowLeft'] ||
                    this.keys['d'] || this.keys['D'] || this.keys['ArrowRight'];
    if (keyHeld) {
      this.moveTarget = null;
      if (this.keys['w'] || this.keys['W'] || this.keys['ArrowUp']) p.dy = -1;
      if (this.keys['s'] || this.keys['S'] || this.keys['ArrowDown']) p.dy = 1;
      if (this.keys['a'] || this.keys['A'] || this.keys['ArrowLeft']) p.dx = -1;
      if (this.keys['d'] || this.keys['D'] || this.keys['ArrowRight']) p.dx = 1;
    } else if (this.moveTarget) {
      const dx = this.moveTarget.x - p.x;
      const dy = this.moveTarget.y - p.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 8) {
        this.moveTarget = null;
      } else {
        p.dx = dx / dist;
        p.dy = dy / dist;
      }
    }
  }

  update(time) {
    if (this.state !== 'playing') return;

    this.camera.follow(this.player);

    const p = this.player;
    this.frameCount++;

    // 输入
    this.updateInput();

    // 玩家更新
    p.update(WORLD_W, WORLD_H);

    // 自动回血
    if (p.regenPerSecond > 0 && this.frameCount % 60 === 0) {
      const healed = p.heal(p.regenPerSecond);
      if (healed > 0) this.addHealNumber(p.x, p.y - p.size - 8, healed);
    }

    // 近战自动攻击
    if (time - p.lastMelee >= p.attackCooldown) {
      let attacked = false;
      for (const e of this.enemies) {
        if (!e.alive) continue;
        const dx = e.x - p.x;
        const dy = e.y - p.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < p.attackRange * p._areaMul + e.size + 10) {
          p.lastMelee = time;
          this.doMeleeAttack();
          attacked = true;
          break;
        }
      }
      if (!attacked) {
        // 没有敌人也能挥砍
        p.lastMelee = time;
      }
    }

    // 远程自动攻击
    if (time - p.lastRanged >= p.rangedCooldown) {
      let nearest = null;
      let nearDist = Infinity;
      for (const e of this.enemies) {
        if (!e.alive) continue;
        const dx = e.x - p.x;
        const dy = e.y - p.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 500 && dist < nearDist) {
          nearDist = dist;
          nearest = e;
        }
      }
      if (nearest) {
        p.lastRanged = time;
        this.doRangedAttack(nearest.x, nearest.y);
      }
    }

    // 检查弹射物
    this.checkProjectileHits();

    // 更新弹射物
    for (const proj of this.projectiles) {
      proj.update();
    }
    this.projectiles = this.projectiles.filter(p => p.alive);

    // 敌人攻击玩家
    this.checkEnemyAttacks();

    // 清理死亡敌人
    this.enemies = this.enemies.filter(e => e.alive);

    // 经验球
    for (const orb of this.orbs) {
      const picked = orb.update(this.player);
      if (picked) {
        this.particles.emitExpOrb(orb.x, orb.y);
        const leveled = this.player.addExp(orb.value);
        this.updateHUD();
        if (leveled) {
          this.showLevelUp();
          return;
        }
      }
    }
    this.orbs = this.orbs.filter(o => o.alive);

    // 冰冻光环
    if (p._freezePower > 0) {
      for (const e of this.enemies) {
        if (!e.alive) continue;
        const dx = e.x - p.x;
        const dy = e.y - p.y;
        if (Math.sqrt(dx * dx + dy * dy) < 70) {
          e.frozenTimer = Math.max(e.frozenTimer, 5);
          e.frozenPower = Math.max(e.frozenPower || 0, p._freezePower);
          if (this.frameCount % 20 === 0) this.particles.emitStatus(e.x, e.y - e.size, '#a9e8ff');
        }
      }
    }

    // 灼烧光环
    if (p._burnPower > 0 && this.frameCount % 30 === 0) {
      for (const e of this.enemies) {
        if (!e.alive) continue;
        const dx = e.x - p.x;
        const dy = e.y - p.y;
        if (Math.sqrt(dx * dx + dy * dy) < 50) {
          e.burnTimer = 30;
          e.burnDamage = Math.max(e.burnDamage, p._burnPower);
          e.takeDamage(p._burnPower, p);
          this.addDamageNumber(e.x, e.y - e.size - 5, p._burnPower, '#e67e22');
          this.applyLifeSteal(p._burnPower, p.x, p.y - p.size - 8, 0.35);
          this.particles.emitStatus(e.x, e.y - e.size, '#ff6b35');
          if (!e.alive) this.onEnemyKilled(e);
        }
      }
    }

    // 跟班
    if (p._minionCount > 0) {
      while (this.minions.length < p._minionCount) {
        this.minions.push(new Minion(p, Math.floor(5 + p.level * 0.5)));
      }
      while (this.minions.length > p._minionCount) {
        this.minions.pop();
      }
      for (const m of this.minions) {
        const result = m.update(this.enemies);
        if (result) {
          const killed = result.target.takeDamage(result.damage, p);
          this.addDamageNumber(result.target.x, result.target.y - result.target.size - 5, result.damage, '#9b59b6');
          this.applyLifeSteal(result.damage, p.x, p.y - p.size - 8, 0.5);
          this.particles.emitHit(result.target.x, result.target.y);
          if (killed) this.onEnemyKilled(result.target);
        }
      }
    } else {
      this.minions = [];
    }

    // 波次生成
    this.spawnTimer--;
    if (this.spawnTimer <= 0 && this.waveSpawned < this.waveEnemyCount) {
      this.spawnEnemy();
      this.waveSpawned++;
      this.spawnTimer = Math.max(5, 20 - this.player.wave * 0.3);
    }

    // 波次中有 Boss（在波次结束前检查）
    const cfg = waveConfig(this.player.wave);
    if (cfg.hasBoss && !this.bossSpawned && this.waveSpawned >= this.waveEnemyCount) {
      this.spawnBoss();
      this.bossSpawned = true;
      // Boss 生成时屏幕震动
      this.shakeTimer = 15;
      this.shakeIntensity = 6;
      // Boss 出现提示
      this.bossText = { text: '⚠ BOSS ⚠', alpha: 1, timer: 45 };
    }

    // 检查当前波次是否结束（所有敌人都已生成且全部消灭）
    const bossCheck = cfg.hasBoss ? this.bossSpawned : true;
    if (this.enemies.length === 0 && this.waveSpawned >= this.waveEnemyCount && bossCheck) {
      this.player.wave++;
      this.startWave();
      this.updateHUD();
    }

    // 更新特效
    this.particles.update();

    // 更新伤害数字
    for (const dn of this.damageNumbers) {
      dn.y += dn.vy;
      dn.vy *= 0.95;
      dn.life--;
      dn.alpha = Math.max(0, dn.life / 40);
    }
    this.damageNumbers = this.damageNumbers.filter(dn => dn.life > 0);

    // 更新波次文字
    if (this.waveText) {
      this.waveText.timer--;
      if (this.waveText.timer > 60) {
        this.waveText.alpha = Math.min(1, (120 - this.waveText.timer) / 15);
      } else {
        this.waveText.alpha = Math.max(0, this.waveText.timer / 60);
      }
      if (this.waveText.timer <= 0) this.waveText = null;
    }

    // 更新 Boss 文字
    if (this.bossText) {
      this.bossText.timer--;
      this.bossText.alpha = Math.max(0, this.bossText.timer / 45);
      if (this.bossText.timer <= 0) this.bossText = null;
    }

    // 更新升级文字
    if (this.levelUpText) {
      this.levelUpText.timer--;
      this.levelUpText.y -= 0.5;
      this.levelUpText.alpha = Math.max(0, this.levelUpText.timer / 30);
      if (this.levelUpText.timer <= 0) this.levelUpText = null;
    }

    // 屏幕震动衰减
    if (this.shakeTimer > 0) {
      this.shakeTimer--;
      this.shakeIntensity *= 0.9;
    }

    // 更新 nova 特效
    this.novas = this.novas.filter(n => n.life-- > 0);
    this.effects = this.effects.filter(e => e.life-- > 0);

    // 更新 HUD
    this.updateHUD();
  }

  render() {
    const ctx = this.ctx;
    const renderer = this.renderer;

    let sx = 0, sy = 0;
    if (this.shakeTimer > 0) {
      const intensity = this.shakeIntensity;
      sx = (Math.random() - 0.5) * intensity;
      sy = (Math.random() - 0.5) * intensity;
    }

    // 1. 清屏
    ctx.clearRect(0, 0, this.renderer.viewW, this.renderer.viewH);

    // 2. 世界空间渲染（相机 + 屏幕震动）
    ctx.save();
    ctx.translate(-this.camera.x + sx, -this.camera.y + sy);

    // Tile 地图背景
    this.tilemap.render(ctx, this.camera);

    // 光环
    renderer.renderAuras(this.player);

    // 技能符文
    renderer.renderSkillSigils(this.player, this.skills.owned, this.frameCount);

    // nova 特效
    for (const n of this.novas) {
      renderer.renderNova(n.x, n.y, n.damage, n.life, n.maxLife);
    }

    // 经验球
    for (const orb of this.orbs) {
      renderer.renderExpOrb(orb);
    }

    // 弹射物
    for (const proj of this.projectiles) {
      renderer.renderProjectile(proj);
    }

    // 技能/武器特效
    for (const effect of this.effects) {
      renderer.renderEffect(effect);
    }

    // 伤害数字
    for (const dn of this.damageNumbers) {
      renderer.renderDamageNumber(dn);
    }

    // 敌人
    for (const enemy of this.enemies) {
      renderer.renderEnemy(enemy);
    }

    // 跟班 (draw above enemies)
    for (const m of this.minions) {
      renderer.renderMinion(m);
    }

    // 玩家
    if (this.state !== 'gameover') {
      renderer.renderPlayer(this.player);
    }

    // 粒子
    renderer.renderParticles(this.particles);

    ctx.restore();

    // 3. 屏幕空间 UI 文字（不受相机影响）
    if (this.waveText) {
      renderer.renderWaveText(this.waveText);
    }
    if (this.bossText) {
      renderer.renderBossText(this.bossText);
    }
    if (this.levelUpText) {
      renderer.renderLevelUpText(this.levelUpText);
    }

    // 4. 小地图
    this.renderMinimap(ctx);
  }

  renderMinimap(ctx) {
    const mapSize = Math.max(92, Math.min(140, Math.floor(this.renderer.viewW * 0.16)));
    const margin = 10;
    const mx = this.renderer.viewW - mapSize - margin;
    const my = this.renderer.viewH - mapSize - margin;
    const scale = mapSize / WORLD_W;

    // 背景
    ctx.fillStyle = 'rgba(0,0,0,0.55)';
    ctx.fillRect(mx, my, mapSize, mapSize);

    // 绘制 tiles 采样（每 4 格一个色块）
    const tileStep = 4;
    const pixelStep = tileStep * TILE_SIZE * scale;
    for (let ty = 0; ty < this.tilemap.rows; ty += tileStep) {
      for (let tx = 0; tx < this.tilemap.cols; tx += tileStep) {
        const tile = this.tilemap.getTile(tx, ty);
        if (!tile) continue;
        const col = TILE_COLORS[tile.type];
        if (!col) continue;
        ctx.fillStyle = `rgb(${col.base[0]},${col.base[1]},${col.base[2]})`;
        ctx.fillRect(mx + tx * TILE_SIZE * scale, my + ty * TILE_SIZE * scale, pixelStep, pixelStep);
      }
    }

    // 相机视口
    ctx.strokeStyle = 'rgba(255,255,255,0.6)';
    ctx.lineWidth = 1;
    ctx.strokeRect(
      mx + this.camera.x * scale,
      my + this.camera.y * scale,
      this.renderer.viewW * scale,
      this.renderer.viewH * scale
    );

    // 敌人
    ctx.fillStyle = '#ff4444';
    for (const e of this.enemies) {
      ctx.fillRect(mx + e.x * scale - 1, my + e.y * scale - 1, 2, 2);
    }

    // 玩家
    ctx.fillStyle = '#44ff44';
    const px = mx + this.player.x * scale;
    const py = my + this.player.y * scale;
    ctx.beginPath();
    ctx.arc(px, py, 3, 0, Math.PI * 2);
    ctx.fill();

    // 边框
    ctx.strokeStyle = 'rgba(255,255,255,0.35)';
    ctx.lineWidth = 1;
    ctx.strokeRect(mx, my, mapSize, mapSize);
  }

  loop(time) {
    this.update(time);
    this.render();
    requestAnimationFrame(t => this.loop(t));
  }
}

// ==================== 启动 ====================
window.addEventListener('DOMContentLoaded', () => {
  window.game = new Game();
});
