// ==================== 敌人 ====================
class Enemy {
  constructor(x, y, type, waveMul) {
    const cfg = ENEMY_TYPES[type];
    this.x = x;
    this.y = y;
    this.type = type;
    this.baseHp = cfg.hp;
    this.hp = Math.floor(cfg.hp * waveMul);
    this.maxHp = this.hp;
    this.attack = Math.floor(cfg.attack * waveMul);
    this.speed = cfg.speed;
    this.size = cfg.size;
    this.expValue = Math.floor(cfg.exp * waveMul);
    this.scoreValue = cfg.score;
    this.color = cfg.color;
    this.name = cfg.name;

    // AI
    this.vx = 0;
    this.vy = 0;
    this.facing = 'right';
    this.animTimer = Math.floor(Math.random() * 60);
    this.attackCooldown = 0;
    this.attackInterval = 60; // frames

    // 状态
    this.frozenTimer = 0;
    this.burnTimer = 0;
    this.burnDamage = 0;
    this.alive = true;
    this.hitFlash = 0;

    // 随机偏移让行进不完美
    this.wobble = Math.random() * 0.3;
  }

  get speedMul() {
    if (this.frozenTimer > 0) return 0.4;
    return 1;
  }

  takeDamage(dmg, player) {
    this.hp -= dmg;
    this.hitFlash = 6;

    if (this.hp <= 0) {
      this.alive = false;
      return true; // 击杀
    }
    return false;
  }

  update(player, enemies) {
    this.hitFlash = Math.max(0, this.hitFlash - 1);
    if (this.frozenTimer > 0) this.frozenTimer--;
    if (this.burnTimer > 0) {
      this.burnTimer--;
      this.hp -= this.burnDamage;
      if (this.hp <= 0) {
        this.alive = false;
        return 'burn';
      }
    }

    // 向玩家移动
    const dx = player.x - this.x;
    const dy = player.y - this.y;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist > 5) {
      const spd = this.speed * this.speedMul;
      this.vx = (dx / dist) * spd + (Math.random() - 0.5) * this.wobble;
      this.vy = (dy / dist) * spd + (Math.random() - 0.5) * this.wobble;
      if (Math.abs(this.vx) > 0.05) this.facing = this.vx < 0 ? 'left' : 'right';
    } else {
      this.vx *= 0.9;
      this.vy *= 0.9;
    }
    this.animTimer++;

    this.x += this.vx;
    this.y += this.vy;

    // 攻击冷却
    if (this.attackCooldown > 0) this.attackCooldown--;

    // 近战攻击玩家
    if (dist < this.size + player.size + 5 && this.attackCooldown <= 0) {
      this.attackCooldown = this.attackInterval;
      return { attack: true, dmg: this.attack };
    }

    return null;
  }
}

// ==================== 经验球 ====================
class ExpOrb {
  constructor(x, y, value) {
    this.x = x;
    this.y = y;
    this.value = value;
    this.size = 4 + Math.min(value / 5, 4);
    this.alive = true;
    this.vx = (Math.random() - 0.5) * 2;
    this.vy = (Math.random() - 0.5) * 2 - 1;
    this.life = 0;
  }

  update(player) {
    this.life++;

    // 初速度衰减
    if (this.life < 10) {
      // 初始飘散
    } else {
      // 磁铁吸引
      const dx = player.x - this.x;
      const dy = player.y - this.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const magnetRange = player.getMagnetRange();
      if (dist < magnetRange) {
        const pull = 3.5;
        this.vx += (dx / dist) * pull * 0.15;
        this.vy += (dy / dist) * pull * 0.15;
      }
    }

    this.vx *= 0.95;
    this.vy *= 0.95;
    this.x += this.vx;
    this.y += this.vy;

    // 检查拾取
    const dx = player.x - this.x;
    const dy = player.y - this.y;
    if (Math.sqrt(dx * dx + dy * dy) < player.size + this.size + 8) {
      this.alive = false;
      return true; // 拾取
    }
    return false;
  }
}

// ==================== 弹射物 ====================
class Projectile {
  constructor(x, y, tx, ty, damage, speed, color, bounceCount) {
    this.x = x;
    this.y = y;
    this.damage = damage;
    this.speed = speed;
    this.color = color || '#f1c40f';
    this.size = 5;
    this.alive = true;
    this.bounces = bounceCount || 0;
    this.hitTargets = new Set();

    const dx = tx - x;
    const dy = ty - y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    this.vx = (dx / dist) * speed;
    this.vy = (dy / dist) * speed;
  }

  update() {
    this.x += this.vx;
    this.y += this.vy;

    if (this.x < 0 || this.x > WORLD_W || this.y < 0 || this.y > WORLD_H) {
      this.alive = false;
    }
  }

  // 弹射到下一个目标
  bounceTo(enemy) {
    if (this.bounces <= 0) {
      this.alive = false;
      return;
    }
    this.bounces--;
    const dx = enemy.x - this.x;
    const dy = enemy.y - this.y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    this.vx = (dx / dist) * this.speed;
    this.vy = (dy / dist) * this.speed;
  }
}

// ==================== 跟班 ====================
class Minion {
  constructor(player, attack) {
    this.player = player;
    this.size = 8;
    this.speed = 2.5;
    this.attack = attack;
    this.attackCooldown = 0;
    this.attackInterval = 40;
    this.target = null;
    this.wanderAngle = Math.random() * Math.PI * 2;
    this.facing = 'right';
    this.animTimer = Math.floor(Math.random() * 60);

    // 初始位置在玩家附近
    this.x = player.x + (Math.random() - 0.5) * 60;
    this.y = player.y + (Math.random() - 0.5) * 60;
  }

  update(enemies) {
    this.attackCooldown--;

    // 找最近敌人
    let nearest = null;
    let nearDist = 200;
    for (const e of enemies) {
      if (!e.alive) continue;
      const dx = e.x - this.x;
      const dy = e.y - this.y;
      const d = Math.sqrt(dx * dx + dy * dy);
      if (d < nearDist) {
        nearDist = d;
        nearest = e;
      }
    }

    if (nearest && nearDist < 180) {
      // 追击
      const dx = nearest.x - this.x;
      const dy = nearest.y - this.y;
      const d = Math.sqrt(dx * dx + dy * dy) || 1;

      if (d > this.size + nearest.size + 5) {
        this.x += (dx / d) * this.speed;
        this.y += (dy / d) * this.speed;
        if (Math.abs(dx) > 1) this.facing = dx < 0 ? 'left' : 'right';
      }

      // 攻击
      if (d < this.size + nearest.size + 10 && this.attackCooldown <= 0) {
        this.attackCooldown = this.attackInterval;
        return { target: nearest, damage: this.attack };
      }
    } else {
      // 跟随玩家
      const dx = this.player.x - this.x;
      const dy = this.player.y - this.y;
      const d = Math.sqrt(dx * dx + dy * dy) || 1;
      if (d > 50) {
        this.x += (dx / d) * this.speed;
        this.y += (dy / d) * this.speed;
        if (Math.abs(dx) > 1) this.facing = dx < 0 ? 'left' : 'right';
      } else if (d < 30) {
        this.wanderAngle += 0.05;
        this.x += Math.cos(this.wanderAngle) * this.speed * 0.3;
        this.y += Math.sin(this.wanderAngle) * this.speed * 0.3;
      }
    }

    // 边界
    this.animTimer++;
    this.x = Math.max(this.size, Math.min(WORLD_W - this.size, this.x));
    this.y = Math.max(this.size, Math.min(WORLD_H - this.size, this.y));

    return null;
  }
}
