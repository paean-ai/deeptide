// ==================== 粒子系统 ====================
class Particle {
  constructor(x, y, vx, vy, color, life, size, options = {}) {
    this.x = x;
    this.y = y;
    this.vx = vx;
    this.vy = vy;
    this.color = color;
    this.life = life;
    this.maxLife = life;
    this.size = size || 3;
    this.gravity = options.gravity || 0;
    this.drag = options.drag == null ? 1 : options.drag;
    this.kind = options.kind || 'square';
    this.twinkle = options.twinkle || false;
  }

  update() {
    this.vx *= this.drag;
    this.vy = this.vy * this.drag + this.gravity;
    this.x += this.vx;
    this.y += this.vy;
    this.life--;
    return this.life > 0;
  }
}

class ParticleSystem {
  constructor() {
    this.particles = [];
  }

  emit(x, y, count, color, speed, life, size, options = {}) {
    for (let i = 0; i < count; i++) {
      const angle = Math.random() * Math.PI * 2;
      const spd = (Math.random() * 0.5 + 0.5) * speed;
      this.particles.push(new Particle(
        x + (Math.random() - 0.5) * 4,
        y + (Math.random() - 0.5) * 4,
        Math.cos(angle) * spd,
        Math.sin(angle) * spd,
        color,
        Math.floor(Math.random() * life * 0.5 + life * 0.5),
        size || (Math.random() * 2 + 2),
        options
      ));
    }
  }

  emitHit(x, y) {
    this.emit(x, y, 10, '#f2c14e', 3.6, 14, 3, { drag: 0.9, twinkle: true });
    this.emit(x, y, 4, '#f3f7ff', 2.2, 9, 2, { drag: 0.88 });
  }

  emitDeath(x, y, color) {
    this.emit(x, y, 18, color, 4.4, 24, 4, { drag: 0.92, gravity: 0.02 });
    this.emit(x, y, 7, '#f3f7ff', 2.2, 12, 2, { drag: 0.86, twinkle: true });
    this.emit(x, y, 5, '#11131a', 2.6, 18, 3, { drag: 0.9 });
  }

  emitLevelUp(x, y) {
    this.emit(x, y, 34, '#f2c14e', 5, 34, 4, { drag: 0.94, twinkle: true });
    this.emit(x, y, 18, '#f3f7ff', 3, 24, 3, { drag: 0.91 });
    this.emit(x, y, 18, '#2f80ed', 4, 28, 3, { drag: 0.93 });
  }

  emitExpOrb(x, y) {
    this.emit(x, y, 4, '#2f80ed', 1.2, 16, 2, { drag: 0.88 });
    this.emit(x, y, 3, '#a9e8ff', 0.9, 12, 1, { drag: 0.86, twinkle: true });
  }

  emitSlash(x, y, facing, range, crit) {
    const dir = facing === 'left' ? -1 : 1;
    for (let i = 0; i < 14; i++) {
      const t = i / 13;
      const px = x + dir * (range * (0.25 + t * 0.65));
      const py = y - 20 + Math.sin(t * Math.PI) * 42;
      this.particles.push(new Particle(
        px, py,
        dir * (1.4 + t * 1.8),
        -0.7 + t * 1.3,
        crit ? '#f3f7ff' : '#f2c14e',
        10 + Math.floor(t * 8),
        crit ? 4 : 3,
        { drag: 0.86, twinkle: crit }
      ));
    }
  }

  emitStatus(x, y, color) {
    this.emit(x, y, 5, color, 1.5, 16, 2, { drag: 0.9, twinkle: true });
  }

  update() {
    this.particles = this.particles.filter(p => p.update());
  }

  render(ctx, camX, camY) {
    for (const p of this.particles) {
      const alpha = p.life / p.maxLife;
      ctx.globalAlpha = alpha;
      ctx.fillStyle = p.color;
      const pulse = p.twinkle ? 0.65 + Math.sin(p.life * 1.7) * 0.25 : 1;
      const s = Math.max(1, Math.round(p.size * alpha * pulse));
      const x = Math.round(p.x - camX - s / 2);
      const y = Math.round(p.y - camY - s / 2);
      if (p.kind === 'spark') {
        ctx.fillRect(x, y + Math.floor(s / 2), s, 1);
        ctx.fillRect(x + Math.floor(s / 2), y, 1, s);
      } else {
        ctx.fillRect(x, y, s, s);
      }
    }
    ctx.globalAlpha = 1;
  }
}
