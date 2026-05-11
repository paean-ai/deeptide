// ==================== 玩家 ====================
class Player {
  constructor(x, y) {
    this.x = x;
    this.y = y;
    this.size = PLAYER_INIT.size;

    // 基础属性
    this.maxHP = PLAYER_INIT.maxHP;
    this.hp = PLAYER_INIT.hp;
    this.baseSpeed = PLAYER_INIT.speed;
    this.baseAttack = PLAYER_INIT.attack;
    this.baseRangedAttack = PLAYER_INIT.rangedAttack;
    this.attackRange = PLAYER_INIT.attackRange;
    this.attackInterval = PLAYER_INIT.attackSpeed;
    this.rangedInterval = PLAYER_INIT.rangedSpeed;
    this.projectileSpeed = PLAYER_INIT.projectileSpeed;
    this.baseCritChance = PLAYER_INIT.critChance;
    this.critDamage = PLAYER_INIT.critDamage;
    this.baseDodge = PLAYER_INIT.dodge;
    this.baseLifeSteal = PLAYER_INIT.lifeSteal;
    this.areaRange = PLAYER_INIT.areaRange;

    // 等级 & 经验
    this.level = 1;
    this.exp = 0;
    this.expToNext = expForLevel(1);

    // 波次
    this.wave = 1;
    this.kills = 0;

    // 攻击计时
    this.lastMelee = 0;
    this.lastRanged = 0;

    // ===== 技能加成 (由技能系统赋值) =====
    this._meleeBonus = 0;
    this._rangedBonus = 0;
    this._attackSpeedMul = 1;
    this._critBonus = 0;
    this._areaMul = 1;
    this._shieldMul = 1;
    this._dodgeBonus = 0;
    this._regenBonus = 0;
    this._reviveMax = 0;
    this._revives = 0;
    this._speedBonus = 0;
    this._lifeStealBonus = 0;
    this._bounceCount = 0;
    this._freezePower = 0;
    this._burnPower = 0;
    this._minionCount = 0;
    this._thorns = 0;
    this._expMul = 1;
    this._magnetBonus = 0;
    this._novaDamage = 0;
    this._metaRegen = 0;
    this._metaMagnetBonus = 0;

    // 移动
    this.dx = 0;
    this.dy = 0;
    this.vx = 0;
    this.vy = 0;
    this.facing = 'right';
    this.animTimer = 0;
    this.attackPulse = 0;

    // 无敌帧
    this.invincible = 0;

    // 被击退
    this.knockbackX = 0;
    this.knockbackY = 0;

    // 玩家颜色（像素画用）
    this.bodyColor = '#3498db';
    this.armorColor = '#2c3e50';
    this.skinColor = '#f5cba7';
  }

  get speed() {
    return this.baseSpeed + this._speedBonus;
  }

  get attack() {
    return this.baseAttack + this._meleeBonus;
  }

  get rangedAttack() {
    return this.baseRangedAttack + this._rangedBonus;
  }

  get critChance() {
    return this.baseCritChance + this._critBonus;
  }

  get dodge() {
    return this.baseDodge + this._dodgeBonus;
  }

  get lifeSteal() {
    return this.baseLifeSteal + this._lifeStealBonus;
  }

  get regenPerSecond() {
    return this._regenBonus + this._metaRegen;
  }

  get attackCooldown() {
    return this.attackInterval * this._attackSpeedMul;
  }

  get rangedCooldown() {
    return this.rangedInterval * this._attackSpeedMul;
  }

  get expMul() {
    return this._expMul;
  }

  getMagnetRange() {
    return 60 + this._magnetBonus + this._metaMagnetBonus;
  }

  heal(amount) {
    if (amount <= 0 || this.hp <= 0) return 0;
    const before = this.hp;
    this.hp = Math.min(this.maxHP, this.hp + amount);
    return this.hp - before;
  }

  addExp(amount) {
    const actual = Math.floor(amount * this.expMul);
    this.exp += actual;
    if (this.exp >= this.expToNext) {
      this.exp -= this.expToNext;
      this.level++;
      this.expToNext = expForLevel(this.level);
      // 升级回血
      this.heal(20);
      return true; // 触发升级
    }
    return false;
  }

  takeDamage(rawDmg) {
    if (this.invincible > 0) return 0;

    // 闪避
    if (Math.random() < this.dodge) return -1;

    // 护盾减伤
    const dmg = Math.max(1, Math.floor(rawDmg * this._shieldMul));
    this.hp -= dmg;
    this.invincible = 15;
    return dmg;
  }

  update(worldW, worldH) {
    if (this.invincible > 0) this.invincible--;

    // 击退衰减
    this.knockbackX *= 0.85;
    this.knockbackY *= 0.85;

    // 移动
    const len = Math.sqrt(this.dx * this.dx + this.dy * this.dy);
    if (len > 0) {
      this.vx = (this.dx / len) * this.speed;
      this.vy = (this.dy / len) * this.speed;
      if (Math.abs(this.dx) > 0.1) this.facing = this.dx < 0 ? 'left' : 'right';
    } else {
      this.vx *= 0.8;
      this.vy *= 0.8;
    }
    this.animTimer++;
    this.attackPulse = Math.max(0, this.attackPulse - 1);

    this.x += this.vx + this.knockbackX;
    this.y += this.vy + this.knockbackY;

    // 边界限制
    this.x = Math.max(this.size, Math.min(worldW - this.size, this.x));
    this.y = Math.max(this.size, Math.min(worldH - this.size, this.y));
  }

  // 是否已死亡
  get dead() {
    return this.hp <= 0;
  }
}
