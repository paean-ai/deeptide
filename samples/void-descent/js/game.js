// === VOID DESCENT - Main Game Engine ===

const VISION_RADIUS = 8;
const POTION_HEAL = 35;
const BIG_POTION_HEAL = 65;
const SCROLL_ATK_BONUS = 4;
const SCROLL_DEF_BONUS = 3;
const REGEN_TURNS = 5;

let gs = null;    // game state
let dungeon = null;
let canvas, ctx;
let tileSize = 18;
let cameraX = 0, cameraY = 0;
let viewCols = 50, viewRows = 35;
let viewportMode = false;
let explored = null;
let messages = [];
let gameOver = false;
let firstAttackUsed = false;
let damagePopups = [];
let animFrame = null;

// Floor themes — colors shift as you descend
function getFloorTheme(floor) {
  const t = (floor - 1) % 6;
  const themes = [
    { wall: '#1a1a3e', floor: '#111122', stairs: '#331144', bg: '#08080f', fog: '#08081a', accent: '#00ff88', name: 'Void' },
    { wall: '#1a2a1a', floor: '#0f1a0f', stairs: '#1a3a1a', bg: '#050a05', fog: '#050a05', accent: '#44dd44', name: 'Depths' },
    { wall: '#2a1a1a', floor: '#1a1111', stairs: '#3a1a1a', bg: '#0f0808', fog: '#0f0808', accent: '#ff6644', name: 'Magma' },
    { wall: '#1a1a2a', floor: '#111122', stairs: '#1a2a3a', bg: '#080810', fog: '#080810', accent: '#44aaff', name: 'Frost' },
    { wall: '#2a1a2a', floor: '#1a1122', stairs: '#3a2a3a', bg: '#0f0810', fog: '#0f0810', accent: '#cc44ff', name: 'Deep Void' },
    { wall: '#2a2a1a', floor: '#1a1a0f', stairs: '#3a3a1a', bg: '#0f0f05', fog: '#0f0f05', accent: '#ddcc44', name: 'Halls' },
  ];
  return themes[t];
}

function initGame() {
  const saved = loadGame();
  if (saved && saved.player && saved.player.hp > 0) {
    gs = saved;
  } else {
    gs = newGameState();
    clearGame();
  }
  gs.player._bonusAtk = 0;
  gs.player._floorKills = 0;
  firstAttackUsed = false;
  damagePopups = [];

  dungeon = generateFloor(gs.floor);
  gs.player.x = dungeon.playerStart.x;
  gs.player.y = dungeon.playerStart.y;

  gs._theme = getFloorTheme(gs.floor);

  if (gs.player.shield > 0) {
    gs.player._shieldHP = gs.player.shield;
  } else {
    gs.player._shieldHP = 0;
  }

  if (!gs.player.phoenixCharges) gs.player._phoenixCharges = 0;
  else gs.player._phoenixCharges = gs.player.phoenixCharges || 0;

  gs.player._visionRadius = VISION_RADIUS + (gs.player.visionBonus || 0);

  explored = [];
  for (let y = 0; y < dungeon.height; y++) {
    explored[y] = new Array(dungeon.width).fill(false);
  }

  document.getElementById('floor-num').textContent = gs.floor;
  document.getElementById('theme-name').textContent = gs._theme.name;
  if (document.getElementById('total-cleared')) {
    document.getElementById('total-cleared').textContent = gs.totalFloorsCleared || 0;
  }
  updateStatsDisplay();

  canvas = document.getElementById('game-canvas');
  ctx = canvas.getContext('2d');

  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);

  document.addEventListener('keydown', handleInput);

  messages = [];
  addMessage(typeof t === 'function' ? t('enterFloor', gs.floor, gs._theme.name) : `Floor ${gs.floor} [${gs._theme.name}] — Find the stairs › and walk onto them`, 'info');

  updateExplored();

  gameOver = false;
  window.voidDescent = { gs, dungeon, get tileSize() { return tileSize; } };
  render();
  startAnimLoop();

  setupTouchControls();
}

function resizeCanvas() {
  const headerH = document.querySelector('.game-header')?.offsetHeight || 30;
  const footerH = document.querySelector('.game-footer')?.offsetHeight || 50;
  const hintH = document.querySelector('.controls-hint')?.offsetHeight || 18;
  const maxW = Math.min(window.innerWidth - 16, 900);
  const maxH = Math.max(260, window.innerHeight - headerH - footerH - hintH - 34);
  viewportMode = window.matchMedia('(max-width: 700px), (pointer: coarse)').matches;

  if (viewportMode) {
    tileSize = Math.floor(Math.min(maxW / 17, maxH / 18));
    tileSize = Math.max(18, Math.min(30, tileSize));
    viewCols = Math.min(dungeon.width, Math.max(13, Math.floor(maxW / tileSize)));
    viewRows = Math.min(dungeon.height, Math.max(13, Math.floor(maxH / tileSize)));
  } else {
    tileSize = Math.floor(Math.min(maxW / dungeon.width, maxH / dungeon.height));
    tileSize = Math.max(12, Math.min(28, tileSize));
    viewCols = dungeon.width;
    viewRows = dungeon.height;
  }

  canvas.width = tileSize * viewCols;
  canvas.height = tileSize * viewRows;
  updateCamera();

  render();
}

function updateCamera() {
  if (!dungeon || !gs) return;
  if (!viewportMode) {
    cameraX = 0;
    cameraY = 0;
    return;
  }
  cameraX = Math.max(0, Math.min(dungeon.width - viewCols, gs.player.x - Math.floor(viewCols / 2)));
  cameraY = Math.max(0, Math.min(dungeon.height - viewRows, gs.player.y - Math.floor(viewRows / 2)));
}

function updateStatsDisplay() {
  const p = gs.player;
  document.getElementById('stat-hp').textContent = `${p.hp}/${p.maxHp}`;
  document.getElementById('stat-atk').textContent = p.atk + (p._bonusAtk || 0);
  document.getElementById('stat-def').textContent = p.def;
  document.getElementById('stat-essence').textContent = gs.essence;
  document.getElementById('stat-shield').textContent = p._shieldHP || 0;
}

function addMessage(text, cls) {
  messages.unshift({ text, cls });
  if (messages.length > 50) messages.pop();
  const footer = document.getElementById('messages');
  if (footer) {
    footer.innerHTML = messages.slice(0, 10).map(m =>
      `<div class="msg msg-${m.cls}">${m.text}</div>`
    ).join('');
  }
}

// === Input ===

function handleInput(e) {
  if (gameOver) return;

  let dx = 0, dy = 0;
  let wait = false;

  switch (e.key) {
    case 'ArrowUp': case 'k': case 'w': dy = -1; break;
    case 'ArrowDown': case 'j': case 's': dy = 1; break;
    case 'ArrowLeft': case 'h': case 'a': dx = -1; break;
    case 'ArrowRight': case 'l': case 'd': dx = 1; break;
    case 'y': dx = -1; dy = -1; break;
    case 'u': dx = 1; dy = -1; break;
    case 'b': dx = -1; dy = 1; break;
    case 'n': dx = 1; dy = 1; break;
    case '.': wait = true; break;
    case '>': case 'Enter':
      tryDescend();
      e.preventDefault();
      return;
    default: return;
  }

  e.preventDefault();

  if (wait) {
    doPlayerTurn(0, 0, true);
  } else {
    doPlayerTurn(dx, dy, false);
  }
}

function doPlayerTurn(dx, dy, waitTurn) {
  const p = gs.player;
  const newX = p.x + dx;
  const newY = p.y + dy;

  // Regen counter
  if (!waitTurn && (dx !== 0 || dy !== 0)) {
    gs.player.regenTurns = (gs.player.regenTurns || 0) + 1;
    if (gs.player.regenTurns >= REGEN_TURNS && gs.player.regen > 0) {
      const heal = Math.min(gs.player.regen, gs.player.maxHp - gs.player.hp);
      if (heal > 0) {
        gs.player.hp += heal;
        SFX.heal();
        addMessage(`Regeneration: +${heal} HP`, 'heal');
      }
      gs.player.regenTurns = 0;
    }
  }

  if (waitTurn) {
    updateExplored();
    render();
    doEnemyTurns();
    updateStatsDisplay();
    return;
  }

  if (dx === 0 && dy === 0) return;

  // Check bounds
  const grid = dungeon.grid;
  if (newY < 0 || newY >= dungeon.height || newX < 0 || newX >= dungeon.width) return;

  const tile = grid[newY][newX];

  if (tile === TILE.WALL) return;

  // Check for enemy at target
  const enemy = dungeon.entities.find(e => e.type !== 'item' && e.x === newX && e.y === newY && e.hp > 0);

  if (enemy) {
    attackEnemy(enemy);
    // Double strike
    if (enemy.hp > 0 && Math.random() < gs.player.doubleStrikeChance) {
      addMessage('⚡ Double Strike!', 'info');
      attackEnemy(enemy);
    }
    // Cleave adjacent enemies
    if (gs.player.cleaveRange && enemy.hp > 0) {
      cleaveAttack(newX, newY);
    }
  } else {
    // Move
    p.x = newX;
    p.y = newY;
    SFX.move();

    // Auto-descend if on stairs
    if (tile === TILE.STAIRS) {
      tryDescend();
      return;
    }

    // Pick up items
    const item = dungeon.entities.find(e => e.type === 'item' && e.x === newX && e.y === newY);
    if (item) {
      pickUpItem(item);
    }
  }

  updateExplored();
  render();
  updateStatsDisplay();
  doEnemyTurns();

  // Swift: chance for extra move
  if (Math.random() < gs.player.swiftChance) {
    addMessage('💨 Swift! Extra move.', 'info');
  }
}

function tryDescend() {
  const p = gs.player;
  if (dungeon.grid[p.y][p.x] === TILE.STAIRS) {
    gs.totalFloorsCleared = (gs.totalFloorsCleared || 0) + 1;
    gs.player._bonusAtk = 0;
    gs.player._floorKills = 0;
    gs.player._shieldHP = gs.player.shield || 0;
    gs.player._phoenixCharges = gs.player.phoenixCharges || 0;
    SFX.stairs();
    saveGame(gs);
    setTimeout(() => { window.location.href = 'upgrades.html'; }, 150);
  } else {
    addMessage('No stairs here. Find › to descend.', 'info');
  }
}

// === Combat ===

function attackEnemy(enemy) {
  const p = gs.player;
  let atk = p.atk + (p._bonusAtk || 0);

  // Berserker: if HP < 30%, +40% ATK
  if (p.berserk && p.hp < p.maxHp * 0.3) {
    atk = Math.floor(atk * 1.4);
  }

  // Crit
  let isCrit = false;
  if (!firstAttackUsed && p.assassin) {
    isCrit = true;
    firstAttackUsed = true;
  } else if (Math.random() < p.critChance) {
    isCrit = true;
  }

  if (isCrit) {
    atk *= 2;
  }

  let dmg = Math.max(1, atk - enemy.def);
  // Variance
  dmg = Math.floor(dmg * (0.85 + Math.random() * 0.3));

  enemy.hp -= dmg;

  const critText = isCrit ? ' CRIT! ' : '';
  addMessage(`You hit ${enemy.name} for ${critText}${dmg} damage`, 'damage');
  spawnDamagePopup(enemy.x, enemy.y - 0.6, (isCrit ? '⚡' : '') + dmg, isCrit ? '#ffdd00' : '#ff4444');
  if (isCrit) SFX.crit(); else SFX.attack();

  // Lifesteal
  if (gs.player.lifesteal > 0) {
    const heal = Math.floor(dmg * gs.player.lifesteal);
    if (heal > 0) {
      gs.player.hp = Math.min(gs.player.maxHp, gs.player.hp + heal);
      addMessage(`🩸 Lifesteal: +${heal} HP`, 'heal');
    }
  }

  // Poison
  if (gs.player.poisonDmg > 0) {
    enemy.poisonTurns = 3;
    enemy.poisonDmg = gs.player.poisonDmg;
  }

  if (enemy.hp <= 0) {
    killEnemy(enemy);
  }
}

function cleaveAttack(centerX, centerY) {
  const range = gs.player.cleaveRange || 1;
  const cleaved = [];
  for (const e of dungeon.entities) {
    if (e.type !== 'item' && e.hp > 0 && !(e.x === centerX && e.y === centerY)) {
      const dist = Math.max(Math.abs(e.x - centerX), Math.abs(e.y - centerY));
      if (dist <= range) {
        const dmg = Math.max(1, Math.floor(gs.player.atk * 0.5) - e.def);
        e.hp -= dmg;
        cleaved.push(`${e.name}(-${dmg})`);
        if (e.hp <= 0) killEnemy(e);
      }
    }
  }
  if (cleaved.length > 0) {
    SFX.attack();
    addMessage(`🌀 Cleave hit: ${cleaved.join(', ')}`, 'damage');
  }
}

function killEnemy(enemy) {
  dungeon.entities = dungeon.entities.filter(e => e !== enemy);
  gs.essence += Math.floor(enemy.xp / 3);
  gs.kills = (gs.kills || 0) + 1;
  gs.player._floorKills = (gs.player._floorKills || 0) + 1;

  SFX.kill();

  if (gs.player.bonusAtkPerKill) {
    gs.player._bonusAtk = (gs.player._bonusAtk || 0) + gs.player.bonusAtkPerKill;
  }

  spawnDamagePopup(enemy.x, enemy.y - 0.6, '💀', '#ffaa00');
  addMessage(`💀 ${enemy.name} destroyed! +${Math.floor(enemy.xp / 3)} essence`, 'kill');
}

// === Enemy AI ===

function doEnemyTurns() {
  const p = gs.player;

  for (const enemy of dungeon.entities) {
    if (enemy.type === 'item') continue;
    if (enemy.hp <= 0) continue;

    // Poison damage
    if (enemy.poisonTurns > 0) {
      enemy.hp -= enemy.poisonDmg;
      enemy.poisonTurns--;
      if (enemy.hp <= 0) {
        killEnemy(enemy);
        continue;
      }
    }

    // Enemy miss chance
    if (Math.random() < (gs.player.enemyMissChance || 0)) {
      addMessage(`${enemy.name} missed!`, 'info');
      continue;
    }

    const dist = Math.max(Math.abs(enemy.x - p.x), Math.abs(enemy.y - p.y));

    switch (enemy.behavior) {
      case 'random': {
        if (dist <= 5 && Math.random() < 0.6) {
          moveEnemyToward(enemy, p);
        } else {
          // Random step
          const dirs = [[0,-1],[0,1],[-1,0],[1,0]];
          const dir = dirs[Math.floor(Math.random() * dirs.length)];
          tryEnemyMove(enemy, enemy.x + dir[0], enemy.y + dir[1]);
        }
        break;
      }
      case 'chase': {
        if (dist <= 8) {
          moveEnemyToward(enemy, p);
        } else if (dist <= 12 && Math.random() < 0.3) {
          moveEnemyToward(enemy, p);
        }
        break;
      }
      case 'flank': {
        if (dist <= 10) {
          // Try to move to a position adjacent to player but not directly toward
          if (Math.random() < 0.5) {
            moveEnemyToward(enemy, p);
          } else {
            // Move perpendicular
            const dx = p.x - enemy.x;
            const dy = p.y - enemy.y;
            const perpX = -dy;
            const perpY = dx;
            tryEnemyMove(enemy, enemy.x + (perpX > 0 ? 1 : -1), enemy.y);
          }
        } else if (Math.random() < 0.2) {
          moveEnemyToward(enemy, p);
        }
        break;
      }
      case 'slow_chase': {
        enemy.behaviorCounter = (enemy.behaviorCounter || 0) + 1;
        if (enemy.behaviorCounter >= 2) {
          enemy.behaviorCounter = 0;
          if (dist <= 10) moveEnemyToward(enemy, p);
        }
        break;
      }
      case 'boss': {
        if (dist <= 15) {
          moveEnemyToward(enemy, p);
        }
        // Boss special: every 6 turns, heal
        enemy.behaviorCounter = (enemy.behaviorCounter || 0) + 1;
        if (enemy.behaviorCounter >= 6) {
          enemy.behaviorCounter = 0;
          const heal = Math.floor(enemy.maxHp * 0.1);
          enemy.hp = Math.min(enemy.maxHp, enemy.hp + heal);
          addMessage(`${enemy.name} pulses with void energy! +${heal} HP`, 'info');
        }
        break;
      }
    }
  }

  // Clean up dead enemies
  dungeon.entities = dungeon.entities.filter(e => e.type === 'item' || e.hp > 0);
  updateExplored();
  render();
  updateStatsDisplay();
}

function moveEnemyToward(enemy, target) {
  const dx = Math.sign(target.x - enemy.x);
  const dy = Math.sign(target.y - enemy.y);

  // Try diagonal first, then cardinal
  if (dx !== 0 && dy !== 0) {
    if (tryEnemyMove(enemy, enemy.x + dx, enemy.y + dy)) return;
  }
  if (dx !== 0) {
    if (tryEnemyMove(enemy, enemy.x + dx, enemy.y)) return;
  }
  if (dy !== 0) {
    if (tryEnemyMove(enemy, enemy.x, enemy.y + dy)) return;
  }
}

function tryEnemyMove(enemy, newX, newY) {
  const p = gs.player;

  // Bounds & wall check
  if (newY < 0 || newY >= dungeon.height || newX < 0 || newX >= dungeon.width) return false;
  if (dungeon.grid[newY][newX] === TILE.WALL) return false;

  // Player check -> attack
  if (newX === p.x && newY === p.y) {
    enemyAttackPlayer(enemy);
    return true;
  }

  // Other enemy check
  if (dungeon.entities.some(e => e !== enemy && e.type !== 'item' && e.x === newX && e.y === newY && e.hp > 0)) {
    return false;
  }

  enemy.x = newX;
  enemy.y = newY;
  return true;
}

function enemyAttackPlayer(enemy) {
  const p = gs.player;

  // Dodge
  if (Math.random() < p.dodgeChance) {
    addMessage(`💫 You dodged ${enemy.name}!`, 'info');
    SFX.dodge();
    spawnDamagePopup(p.x, p.y - 0.6, 'MISS', '#44cccc');
    return;
  }

  let dmg = Math.max(1, enemy.atk - p.def);
  dmg = Math.floor(dmg * (0.85 + Math.random() * 0.3));

  // Absorb with shield first
  if (p._shieldHP > 0) {
    const absorbed = Math.min(p._shieldHP, dmg);
    p._shieldHP -= absorbed;
    dmg -= absorbed;
    SFX.shield();
    if (dmg <= 0) {
      addMessage(`🔮 Shield absorbed ${absorbed} damage from ${enemy.name}`, 'info');
      return;
    }
    addMessage(`🔮 Shield absorbed ${absorbed}, ${enemy.name} deals ${dmg}`, 'damage');
  } else {
    SFX.hit();
    addMessage(`${enemy.name} hits you for ${dmg} damage`, 'damage');
  }

  p.hp -= dmg;
  spawnDamagePopup(p.x, p.y - 0.6, '-' + dmg, '#ff4444');

  // Thorns
  if (p.thorns > 0) {
    const reflect = Math.floor(dmg * p.thorns);
    if (reflect > 0) {
      enemy.hp -= reflect;
      spawnDamagePopup(enemy.x, enemy.y - 0.6, '🌵' + reflect, '#44dd44');
      addMessage(`🌵 Thorns reflect ${reflect} to ${enemy.name}`, 'damage');
      if (enemy.hp <= 0) killEnemy(enemy);
    }
  }

  // Check death
  if (p.hp <= 0) {
    if (p._phoenixCharges > 0) {
      p._phoenixCharges--;
      p.hp = Math.floor(p.maxHp * 0.5);
      SFX.phoenix();
      addMessage('🐦 Phoenix revived you!', 'heal');
      updateStatsDisplay();
      return;
    }
    playerDeath();
  }
}

// === Items ===

function pickUpItem(item) {
  const p = gs.player;
  dungeon.entities = dungeon.entities.filter(e => e !== item);

  SFX.pickup();

  switch (item.itemType) {
    case 'potion': {
      const bonus = 1 + (p.potionBonus || 0);
      const heal = Math.floor(POTION_HEAL * bonus);
      const actual = Math.min(heal, p.maxHp - p.hp);
      p.hp += actual;
      SFX.heal();
      spawnDamagePopup(p.x, p.y - 0.6, '+' + actual, '#44dd44');
      addMessage(`🧪 +${actual} HP from potion`, 'heal');
      break;
    }
    case 'big_potion': {
      const bonus = 1 + (p.potionBonus || 0);
      const heal = Math.floor(BIG_POTION_HEAL * bonus);
      const actual = Math.min(heal, p.maxHp - p.hp);
      p.hp += actual;
      SFX.heal();
      spawnDamagePopup(p.x, p.y - 0.6, '+' + actual, '#44dd44');
      addMessage(`🧪 Greater Potion: +${actual} HP`, 'heal');
      break;
    }
    case 'atk_scroll':
      p._bonusAtk = (p._bonusAtk || 0) + SCROLL_ATK_BONUS;
      addMessage('≫ Attack Scroll: +4 ATK this floor', 'loot');
      break;
    case 'def_scroll':
      p.def += SCROLL_DEF_BONUS;
      addMessage('≪ Defense Scroll: +3 DEF this floor', 'loot');
      break;
    case 'essence': {
      const amount = 10 + Math.floor(Math.random() * 21);
      gs.essence += amount;
      spawnDamagePopup(p.x, p.y - 0.6, '+' + amount, '#cc66ff');
      addMessage(`✦ +${amount} Void Essence`, 'loot');
      break;
    }
  }
}

// === Vision ===

function updateExplored() {
  const p = gs.player;
  const r = p._visionRadius;

  for (let y = Math.max(0, p.y - r); y < Math.min(dungeon.height, p.y + r + 1); y++) {
    for (let x = Math.max(0, p.x - r); x < Math.min(dungeon.width, p.x + r + 1); x++) {
      const dist = Math.max(Math.abs(x - p.x), Math.abs(y - p.y));
      if (dist <= r) {
        // Simple LOS: check if there's a wall blocking
        if (hasLineOfSight(p.x, p.y, x, y)) {
          explored[y][x] = true;
        }
      }
    }
  }
}

function hasLineOfSight(x0, y0, x1, y1) {
  const dx = Math.abs(x1 - x0);
  const dy = Math.abs(y1 - y0);
  let x = x0, y = y0;
  const sx = x0 < x1 ? 1 : -1;
  const sy = y0 < y1 ? 1 : -1;
  let err = dx - dy;

  while (x !== x1 || y !== y1) {
    if (dungeon.grid[y][x] === TILE.WALL && (x !== x0 || y !== y0)) return false;
    const e2 = 2 * err;
    if (e2 > -dy) { err -= dy; x += sx; }
    if (e2 < dx) { err += dx; y += sy; }
  }
  return true;
}

// === Rendering ===

function getThemeColors() {
  return gs._theme || getFloorTheme(1);
}

function spawnDamagePopup(gx, gy, text, color) {
  damagePopups.push({
    gx,
    gy,
    text: String(text),
    color: color || '#ff4444',
    life: 1.0,
    vy: -0.6,
  });
}

function startAnimLoop() {
  if (animFrame) cancelAnimationFrame(animFrame);
  function loop() {
    if (gameOver) { animFrame = null; return; }
    render();
    animFrame = requestAnimationFrame(loop);
  }
  animFrame = requestAnimationFrame(loop);
}

function render() {
  const theme = getThemeColors();
  const pixelTheme = vdTheme(theme.name);
  updateCamera();
  ctx.fillStyle = pixelTheme.fog;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  const now = Date.now();

  const endX = Math.min(dungeon.width, cameraX + viewCols);
  const endY = Math.min(dungeon.height, cameraY + viewRows);

  for (let y = cameraY; y < endY; y++) {
    for (let x = cameraX; x < endX; x++) {
      const px = (x - cameraX) * tileSize;
      const py = (y - cameraY) * tileSize;

      if (!explored[y][x]) {
        vdDrawTile(ctx, dungeon.grid[y][x], px, py, tileSize, pixelTheme, x, y, false);
        continue;
      }

      const tile = dungeon.grid[y][x];
      vdDrawTile(ctx, tile, px, py, tileSize, pixelTheme, x, y, true);

      if (tile === TILE.STAIRS) {
        const glow = 0.3 + Math.sin(now / 500) * 0.2;
        ctx.fillStyle = `rgba(182, 108, 255, ${glow})`;
        ctx.fillRect(px + 2, py + 2, tileSize - 4, tileSize - 4);
        vdDrawSprite(ctx, VD_SPRITES.stairs, px, py, tileSize);
      }
    }
  }

  // Entities
  const p = gs.player;
  for (const e of dungeon.entities) {
    if (!explored[e.y] || !explored[e.y][e.x]) continue;
    const dist = Math.max(Math.abs(e.x - p.x), Math.abs(e.y - p.y));
    if (dist > p._visionRadius) continue;
    if (e.x < cameraX || e.x >= endX || e.y < cameraY || e.y >= endY) continue;

    const px = (e.x - cameraX) * tileSize;
    const py = (e.y - cameraY) * tileSize;
    const sz = tileSize;

    if (e.type === 'item') {
      const bob = Math.sin(now / 400 + e.x + e.y) * 1;
      vdDrawShadow(ctx, px, py + bob, sz, 0.18);
      vdDrawSprite(ctx, VD_SPRITES[e.itemType] || VD_SPRITES.essence, px, py + bob, sz, 0.96);
    } else {
      const bob = Math.sin(now / 360 + e.x * 0.8 + e.y) * (e.type === 'wraith' || e.type === 'shade' ? 1.5 : 0.6);
      vdDrawShadow(ctx, px, py, sz, 0.26);
      vdDrawSprite(ctx, VD_SPRITES[e.type] || VD_SPRITES.slime, px, py + bob, sz, 0.98);

      if (e.hp < e.maxHp) {
        const barH = 3;
        const barW = sz - 2;
        const hpPct = Math.max(0, e.hp / e.maxHp);
        ctx.fillStyle = '#090b11';
        ctx.fillRect(px + 1, py - barH - 1, barW, barH);
        ctx.fillStyle = hpPct > 0.5 ? VD_PALETTE.green : hpPct > 0.25 ? VD_PALETTE.gold : VD_PALETTE.red;
        ctx.fillRect(px + 1, py - barH - 1, barW * hpPct, barH);
      }

      if (e.poisonTurns > 0) {
        ctx.fillStyle = 'rgba(67,209,122,0.32)';
        ctx.fillRect(px + 2, py + 2, sz - 4, 2);
        ctx.fillRect(px + 2, py + sz - 4, sz - 4, 2);
      }
    }
  }

  // Player
  const ppx = (p.x - cameraX) * tileSize;
  const ppy = (p.y - cameraY) * tileSize;
  const psz = tileSize;

  ctx.fillStyle = 'rgba(67,209,122,0.14)';
  ctx.fillRect(ppx - tileSize, ppy - tileSize, tileSize * 3, tileSize * 3);
  vdDrawShadow(ctx, ppx, ppy, psz, 0.32);
  vdDrawSprite(ctx, VD_SPRITES.player, ppx, ppy + Math.sin(now / 260) * 0.8, psz, 1);

  // Shield indicator
  if (p._shieldHP > 0) {
    ctx.fillStyle = 'rgba(169,232,255,0.24)';
    ctx.fillRect(ppx - 2, ppy - 2, psz + 4, 3);
    ctx.fillRect(ppx - 2, ppy + psz - 1, psz + 4, 3);
    ctx.fillRect(ppx - 2, ppy - 2, 3, psz + 4);
    ctx.fillRect(ppx + psz - 1, ppy - 2, 3, psz + 4);
  }

  // Damage popups (in pixel space)
  for (let i = damagePopups.length - 1; i >= 0; i--) {
    const dp = damagePopups[i];
    dp.life -= 0.015;
    dp.gy += dp.vy / tileSize;
    if (dp.life <= 0) { damagePopups.splice(i, 1); continue; }
    if (dp.gx < cameraX || dp.gx >= endX || dp.gy < cameraY - 1 || dp.gy >= endY) continue;

    ctx.globalAlpha = dp.life;
    ctx.fillStyle = dp.color;
    ctx.font = `bold ${Math.floor(tileSize * 0.7)}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(
      dp.text,
      (dp.gx - cameraX) * tileSize + tileSize / 2,
      (dp.gy - cameraY) * tileSize
    );
    ctx.globalAlpha = 1;
  }

  // Minimap (bottom-right corner)
  drawMinimap(now);
}

// === Minimap ===

function drawMinimap(now) {
  const mmW = Math.floor(canvas.width * 0.18);
  const mmH = Math.floor(canvas.height * 0.22);
  const mmX = canvas.width - mmW - 6;
  const mmY = canvas.height - mmH - 6;
  const cellW = mmW / dungeon.width;
  const cellH = mmH / dungeon.height;

  // Background
  ctx.fillStyle = 'rgba(0,0,0,0.55)';
  ctx.fillRect(mmX, mmY, mmW, mmH);
  ctx.strokeStyle = 'rgba(255,255,255,0.15)';
  ctx.lineWidth = 1;
  ctx.strokeRect(mmX, mmY, mmW, mmH);

  const p = gs.player;
  for (let y = 0; y < dungeon.height; y++) {
    for (let x = 0; x < dungeon.width; x++) {
      if (!explored[y][x]) continue;
      const sx = mmX + x * cellW;
      const sy = mmY + y * cellH;
      const tile = dungeon.grid[y][x];
      if (tile === TILE.WALL) {
        ctx.fillStyle = 'rgba(60,60,100,0.4)';
      } else if (tile === TILE.STAIRS) {
        const g = 0.4 + Math.sin(now / 400) * 0.3;
        ctx.fillStyle = `rgba(255,102,255,${g})`;
      } else {
        ctx.fillStyle = 'rgba(30,30,50,0.3)';
      }
      ctx.fillRect(sx, sy, Math.max(1, cellW), Math.max(1, cellH));
    }
  }

  // Enemies on minimap
  for (const e of dungeon.entities) {
    if (e.type === 'item') continue;
    if (!explored[e.y] || !explored[e.y][e.x]) continue;
    const dist = Math.max(Math.abs(e.x - p.x), Math.abs(e.y - p.y));
    if (dist > p._visionRadius) continue;
    ctx.fillStyle = e.color;
    ctx.fillRect(mmX + e.x * cellW, mmY + e.y * cellH, Math.max(2, cellW), Math.max(2, cellH));
  }

  // Player on minimap (pulsing)
  const pS = 1 + Math.sin(now / 200) * 0.5;
  ctx.fillStyle = '#00ff88';
  ctx.fillRect(
    mmX + p.x * cellW - pS, mmY + p.y * cellH - pS,
    Math.max(3, cellW + pS * 2), Math.max(3, cellH + pS * 2)
  );
}

// === Death ===

function playerDeath() {
  gameOver = true;
  if (animFrame) cancelAnimationFrame(animFrame);
  document.removeEventListener('keydown', handleInput);
  SFX.death();
  clearGame();

  const overlay = document.createElement('div');
  overlay.className = 'gameover-overlay';
  overlay.innerHTML = `
    <div class="gameover-box">
      <div class="gameover-title">${typeof t === 'function' ? t('youDied') : 'YOU DIED'}</div>
      <div class="gameover-stats">
        ${typeof t === 'function' ? t('floorReached') : 'Floor reached'}: <span style="color:#ff66ff">${gs.floor}</span><br>
        ${typeof t === 'function' ? t('floorsCleared') : 'Floors cleared'}: <span style="color:#ff66ff">${gs.totalFloorsCleared || 0}</span><br>
        ${typeof t === 'function' ? t('totalKills') : 'Total kills'}: <span style="color:#ffaa00">${gs.kills || 0}</span><br>
        ${typeof t === 'function' ? t('upgradesCollected') : 'Upgrades collected'}: <span style="color:#00ff88">${Object.values(gs.upgrades).reduce((a,b)=>a+b,0)}</span>
      </div>
      <button class="btn" onclick="location.reload()">${typeof t === 'function' ? t('descendAgain') : 'DESCEND AGAIN'}</button>
      <br><br>
      <a href="index.html" style="color:var(--dim);font-size:0.65rem;">${typeof t === 'function' ? t('returnTitle') : 'Return to Title'}</a>
    </div>
  `;
  document.body.appendChild(overlay);
}

// === Mobile Touch Controls ===

function setupTouchControls() {
  const canvas = document.getElementById('game-canvas');
  if (!canvas) return;

  let touchStart = null;

  canvas.addEventListener('touchstart', e => {
    e.preventDefault();
    const t = e.touches[0];
    touchStart = { x: t.clientX, y: t.clientY };
  });

  canvas.addEventListener('touchend', e => {
    e.preventDefault();
    if (!touchStart) return;
    const t = e.changedTouches[0];
    const dx = t.clientX - touchStart.x;
    const dy = t.clientY - touchStart.y;
    const threshold = 20;

    if (Math.abs(dx) < threshold && Math.abs(dy) < threshold) {
      tryDescend();
      return;
    }

    let mx = 0, my = 0;
    if (Math.abs(dx) > Math.abs(dy)) {
      mx = dx > 0 ? 1 : -1;
    } else {
      my = dy > 0 ? 1 : -1;
    }
    doPlayerTurn(mx, my, false);
    touchStart = null;
  });
}
