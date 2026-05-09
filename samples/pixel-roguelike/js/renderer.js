// ==================== Pixel renderer ====================

function frameFor(asset, state, timer, speed) {
  const frames = asset.animations[state] || asset.animations.idle;
  const idx = Math.floor(timer / (speed || 16)) % frames.length;
  return frames[idx];
}

function makePalette(base, mode) {
  const colors = { ...base };
  if (mode === 'flash') {
    for (const key of Object.keys(colors)) colors[key] = '#ffffff';
  } else if (mode === 'frozen') {
    for (const key of Object.keys(colors)) {
      if (key !== 'K' && key !== 'W') colors[key] = '#83d8ff';
    }
  } else if (mode === 'burn') {
    for (const key of Object.keys(colors)) {
      if (key !== 'K' && key !== 'W') colors[key] = key === 'Y' ? '#ffe066' : '#ff6b35';
    }
  }
  return colors;
}

function drawPixelSprite(ctx, frame, colorMap, x, y, scale, options = {}) {
  const rows = frame.length;
  const cols = frame[0] ? frame[0].length : 0;
  const pixelSize = scale || PIXEL_SCALE;
  const flipX = options.flipX || false;
  const ox = Math.round(x - (cols * pixelSize) / 2);
  const oy = Math.round(y - (rows * pixelSize) / 2 + (options.anchorY || 0));

  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const srcCol = flipX ? cols - 1 - col : col;
      const ch = frame[row][srcCol];
      if (ch !== '.' && colorMap[ch]) {
        ctx.fillStyle = colorMap[ch];
        ctx.fillRect(ox + col * pixelSize, oy + row * pixelSize, pixelSize, pixelSize);
      }
    }
  }
}

function drawPixelCircle(ctx, x, y, radius, step, color, alpha) {
  ctx.save();
  ctx.globalAlpha = alpha == null ? 1 : alpha;
  ctx.fillStyle = color;
  const r2 = radius * radius;
  const inner = Math.max(0, radius - step * 1.5);
  const inner2 = inner * inner;
  for (let py = -radius; py <= radius; py += step) {
    for (let px = -radius; px <= radius; px += step) {
      const d2 = px * px + py * py;
      if (d2 <= r2 && d2 >= inner2) {
        ctx.fillRect(Math.round(x + px), Math.round(y + py), step, step);
      }
    }
  }
  ctx.restore();
}

function drawPixelShadow(ctx, x, y, width, height, alpha) {
  ctx.save();
  ctx.globalAlpha = alpha || 0.28;
  ctx.fillStyle = '#050608';
  const step = 3;
  for (let py = -height; py <= height; py += step) {
    for (let px = -width; px <= width; px += step) {
      if ((px * px) / (width * width) + (py * py) / (height * height) <= 1) {
        ctx.fillRect(Math.round(x + px), Math.round(y + py), step, step);
      }
    }
  }
  ctx.restore();
}

class Renderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.viewW = CANVAS_W;
    this.viewH = CANVAS_H;
    this.resize(CANVAS_W, CANVAS_H);
  }

  resize(width, height) {
    const nextW = Math.max(320, Math.floor(width));
    const nextH = Math.max(320, Math.floor(height));
    if (this.canvas.width !== nextW || this.canvas.height !== nextH) {
      this.canvas.width = nextW;
      this.canvas.height = nextH;
    }
    this.viewW = nextW;
    this.viewH = nextH;
    this.ctx.imageSmoothingEnabled = false;
  }

  renderActor(assetId, actor, state, opts = {}) {
    const asset = SPRITES[assetId] || SPRITES.slime;
    const timer = actor.animTimer || 0;
    const frame = frameFor(asset, state || 'idle', timer, opts.speed || 14);
    const mode = opts.mode || null;
    const palette = makePalette(asset.palette, mode);
    const bob = opts.bob ? Math.round(Math.sin(timer * 0.16) * opts.bob) : 0;
    if (!opts.noShadow) {
      const shadowW = opts.shadowW || (assetId === 'boss' ? 28 : 18);
      const shadowH = opts.shadowH || (assetId === 'bat' || assetId === 'ghost' ? 4 : 6);
      drawPixelShadow(this.ctx, actor.x, actor.y + (opts.shadowY || actor.size || 12), shadowW, shadowH, opts.shadowAlpha);
    }
    drawPixelSprite(this.ctx, frame, palette, actor.x, actor.y + bob, asset.scale, {
      flipX: actor.facing === 'left',
      anchorY: asset.anchorY || 0,
    });
  }

  renderPlayer(player) {
    if (player.invincible > 0 && Math.floor(player.invincible / 3) % 2 === 0) return;
    const moving = Math.abs(player.vx) + Math.abs(player.vy) > 0.25 || Math.abs(player.dx) + Math.abs(player.dy) > 0.1;
    const state = player.invincible > 0 ? 'hit' : (player.attackPulse > 0 ? 'attack' : (moving ? 'walk' : 'idle'));
    this.renderActor('player', player, state, { speed: state === 'attack' ? 5 : (moving ? 8 : 24), bob: moving ? 1 : 0 });

    if (player.attackPulse > 0) {
      const alpha = player.attackPulse / 10;
      drawPixelCircle(this.ctx, player.x, player.y, 36 + player._areaMul * 8, 4, '#f2c14e', alpha * 0.45);
    }
  }

  renderSkillSigils(player, owned, frameCount) {
    const ctx = this.ctx;
    const entries = Object.entries(owned || {});
    if (!entries.length) return;
    const active = entries.filter(([, lv]) => lv > 0).slice(0, 8);
    const radius = 23 + Math.min(active.length, 6) * 3;
    active.forEach(([id, lv], i) => {
      const angle = frameCount * 0.018 + i * Math.PI * 2 / active.length;
      const x = player.x + Math.cos(angle) * radius;
      const y = player.y + Math.sin(angle) * (radius * 0.55) - 4;
      const color = SKILL_VISUALS[id] || '#f3f7ff';
      const size = 3 + Math.min(lv, 3);
      ctx.globalAlpha = 0.55;
      ctx.fillStyle = '#050608';
      ctx.fillRect(Math.round(x - size), Math.round(y - size), size * 2, size * 2);
      ctx.globalAlpha = 0.92;
      ctx.fillStyle = color;
      ctx.fillRect(Math.round(x - size / 2), Math.round(y - size / 2), size, size);
      ctx.fillStyle = '#f3f7ff';
      ctx.fillRect(Math.round(x), Math.round(y), 1, 1);
      ctx.globalAlpha = 1;
    });
  }

  renderEnemy(enemy) {
    if (!enemy.alive) return;
    let mode = null;
    if (enemy.hitFlash > 0) mode = 'flash';
    else if (enemy.frozenTimer > 0) mode = 'frozen';
    else if (enemy.burnTimer > 0) mode = 'burn';

    const bob = enemy.type === 'ghost' ? 3 : enemy.type === 'bat' ? 2 : 1;
    this.renderActor(enemy.type, enemy, 'idle', { mode, speed: enemy.type === 'bat' ? 7 : 16, bob });

    if (enemy.type === 'boss' || enemy.type === 'elite') {
      const barW = enemy.type === 'boss' ? 46 : 34;
      const barH = 4;
      const bx = Math.round(enemy.x - barW / 2);
      const by = Math.round(enemy.y - enemy.size - 13);
      this.ctx.fillStyle = 'rgba(0,0,0,0.72)';
      this.ctx.fillRect(bx - 1, by - 1, barW + 2, barH + 2);
      this.ctx.fillStyle = enemy.type === 'boss' ? '#e05243' : '#ff8a3d';
      this.ctx.fillRect(bx, by, Math.max(0, barW * (enemy.hp / enemy.maxHp)), barH);
      this.ctx.fillStyle = '#f3f7ff';
      this.ctx.fillRect(bx, by, 2, barH);
    }
  }

  renderProjectile(proj) {
    const ctx = this.ctx;
    const s = proj.size;
    for (let i = 0; i < 3; i++) {
      const tx = proj.x - proj.vx * i * 0.7;
      const ty = proj.y - proj.vy * i * 0.7;
      ctx.globalAlpha = 0.35 - i * 0.09;
      ctx.fillStyle = proj.color;
      ctx.fillRect(Math.round(tx - s / 2), Math.round(ty - s / 2), s, s);
    }
    ctx.globalAlpha = 1;
    ctx.fillStyle = '#f3f7ff';
    ctx.fillRect(Math.round(proj.x - 1), Math.round(proj.y - 1), 2, 2);
  }

  renderEffect(effect) {
    if (effect.kind === 'slash') {
      const t = 1 - effect.life / effect.maxLife;
      const dir = effect.facing === 'left' ? -1 : 1;
      const color = effect.crit ? '#f3f7ff' : '#f2c14e';
      this.ctx.save();
      this.ctx.globalAlpha = Math.max(0, 0.85 - t * 0.75);
      this.ctx.fillStyle = color;
      for (let i = 0; i < 11; i++) {
        const u = i / 10;
        const x = effect.x + dir * (effect.range * (0.22 + u * 0.72));
        const y = effect.y - 28 + Math.sin(u * Math.PI) * 54 + t * 6;
        const s = Math.max(2, Math.round((1 - Math.abs(u - 0.5)) * 7));
        this.ctx.fillRect(Math.round(x), Math.round(y), s, s);
      }
      this.ctx.restore();
    } else if (effect.kind === 'ring') {
      const t = 1 - effect.life / effect.maxLife;
      drawPixelCircle(this.ctx, effect.x, effect.y, effect.radius * (0.45 + t), 4, effect.color, Math.max(0, effect.alpha * (1 - t)));
    } else if (effect.kind === 'beam') {
      this.ctx.save();
      this.ctx.globalAlpha = effect.life / effect.maxLife;
      this.ctx.fillStyle = effect.color;
      for (let i = 0; i < 5; i++) {
        const t = i / 4;
        this.ctx.fillRect(
          Math.round(effect.x + (effect.tx - effect.x) * t),
          Math.round(effect.y + (effect.ty - effect.y) * t),
          4,
          4
        );
      }
      this.ctx.restore();
    }
  }

  renderExpOrb(orb) {
    const ctx = this.ctx;
    const pulse = Math.sin(orb.life * 0.12);
    const s = Math.round(orb.size + pulse);
    ctx.fillStyle = '#12365f';
    ctx.fillRect(Math.round(orb.x - s / 2 - 1), Math.round(orb.y - s / 2 - 1), s + 2, s + 2);
    ctx.fillStyle = '#2f80ed';
    ctx.fillRect(Math.round(orb.x - s / 2), Math.round(orb.y - s / 2), s, s);
    ctx.fillStyle = '#a9e8ff';
    ctx.fillRect(Math.round(orb.x - 1), Math.round(orb.y - 1), 2, 2);
  }

  renderMinion(minion) {
    this.renderActor('minion', minion, 'idle', { speed: 12, bob: 1 });
  }

  renderParticles(particles) {
    particles.render(this.ctx, 0, 0);
  }

  renderDamageNumber(dn) {
    const ctx = this.ctx;
    ctx.save();
    ctx.globalAlpha = dn.alpha;
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'center';
    ctx.fillStyle = '#08090d';
    ctx.fillText(dn.text, dn.x + 1, dn.y + 1);
    ctx.fillStyle = dn.color;
    ctx.fillText(dn.text, dn.x, dn.y);
    ctx.restore();
  }

  renderWaveText(wt) {
    const ctx = this.ctx;
    ctx.save();
    ctx.globalAlpha = wt.alpha;
    ctx.font = `bold ${Math.max(28, Math.min(48, this.viewW * 0.06))}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const cx = this.viewW / 2;
    const cy = this.viewH / 3;
    ctx.fillStyle = '#08090d';
    ctx.fillText(wt.text, cx + 2, cy + 2);
    ctx.fillStyle = '#f2c14e';
    ctx.fillText(wt.text, cx, cy);
    ctx.restore();
  }

  renderBossText(bt) {
    const ctx = this.ctx;
    ctx.save();
    ctx.globalAlpha = bt.alpha;
    ctx.font = `bold ${Math.max(34, Math.min(56, this.viewW * 0.07))}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const pulse = Math.sin(bt.timer * 0.5) * 3;
    ctx.fillStyle = '#08090d';
    ctx.fillText(bt.text, this.viewW / 2 + 2 + pulse, this.viewH / 2 + 2);
    ctx.fillStyle = '#e05243';
    ctx.fillText(bt.text, this.viewW / 2 + pulse, this.viewH / 2);
    ctx.restore();
  }

  renderLevelUpText(lt) {
    const ctx = this.ctx;
    ctx.save();
    ctx.globalAlpha = lt.alpha;
    ctx.font = `bold ${Math.max(36, Math.min(64, this.viewW * 0.075))}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const pulse = Math.sin(lt.timer * 0.8) * 4;
    const cy = this.viewH * 0.35 + lt.y;
    ctx.fillStyle = '#08090d';
    ctx.fillText(lt.text, this.viewW / 2 + 2, cy + 2);
    ctx.fillStyle = '#f2c14e';
    ctx.fillText(lt.text, this.viewW / 2 + pulse, cy);
    ctx.restore();
  }

  renderAuras(player) {
    if (player._freezePower > 0) {
      drawPixelCircle(this.ctx, player.x, player.y, 70, 5, '#83d8ff', 0.20);
      drawPixelCircle(this.ctx, player.x, player.y, 48 + Math.sin(player.animTimer * 0.08) * 3, 4, '#f3f7ff', 0.10);
    }
    if (player._burnPower > 0) {
      drawPixelCircle(this.ctx, player.x, player.y, 50, 5, '#ff6b35', 0.22);
      drawPixelCircle(this.ctx, player.x, player.y, 34 + Math.cos(player.animTimer * 0.12) * 4, 4, '#f2c14e', 0.12);
    }
    if (player._shieldMul < 1) {
      drawPixelCircle(this.ctx, player.x, player.y, 30, 4, '#2f80ed', 0.18);
    }
    if (player._magnetBonus > 0) {
      drawPixelCircle(this.ctx, player.x, player.y, player.getMagnetRange(), 6, '#b66cff', 0.08);
    }
  }

  renderNova(x, y, damage, life, maxLife) {
    const t = 1 - life / maxLife;
    const radius = Math.min(20 + damage, 64) * (0.5 + t * 0.8);
    drawPixelCircle(this.ctx, x, y, radius, 5, '#ff6b35', Math.max(0, 0.65 - t * 0.55));
    drawPixelCircle(this.ctx, x, y, radius * 0.65, 5, '#f2c14e', Math.max(0, 0.35 - t * 0.3));
  }
}
