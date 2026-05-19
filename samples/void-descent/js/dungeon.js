// Procedural dungeon generation

const TILE = { WALL: 0, FLOOR: 1, STAIRS: 2, DOOR: 3 };

function seededRandom(seed) {
  let s = seed;
  return function() {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

function generateFloor(floorNumber) {
  const seed = floorNumber * 1337 + 42;
  const rng = seededRandom(seed);
  const width = 50;
  const height = 35;
  const minRooms = 7 + Math.min(floorNumber, 8);
  const maxRooms = minRooms + 5;

  // Init grid with walls
  const grid = [];
  for (let y = 0; y < height; y++) {
    grid[y] = new Array(width).fill(TILE.WALL);
  }

  // Generate rooms
  const rooms = [];
  const targetRooms = minRooms + Math.floor(rng() * (maxRooms - minRooms));

  for (let attempt = 0; attempt < 200 && rooms.length < targetRooms; attempt++) {
    const rw = 3 + Math.floor(rng() * 8);
    const rh = 3 + Math.floor(rng() * 6);
    const rx = 1 + Math.floor(rng() * (width - rw - 2));
    const ry = 1 + Math.floor(rng() * (height - rh - 2));

    const room = { x: rx, y: ry, w: rw, h: rh, cx: rx + Math.floor(rw / 2), cy: ry + Math.floor(rh / 2) };

    let overlap = false;
    for (const other of rooms) {
      if (rx - 1 < other.x + other.w && rx + rw + 1 > other.x &&
          ry - 1 < other.y + other.h && ry + rh + 1 > other.y) {
        overlap = true;
        break;
      }
    }

    if (!overlap) {
      rooms.push(room);
      // Carve room
      for (let y = room.y; y < room.y + room.h; y++) {
        for (let x = room.x; x < room.x + room.w; x++) {
          grid[y][x] = TILE.FLOOR;
        }
      }
    }
  }

  // Connect rooms with corridors
  for (let i = 1; i < rooms.length; i++) {
    const a = rooms[i - 1];
    const b = rooms[i];
    carveCorridor(grid, a.cx, a.cy, b.cx, b.cy);
  }

  // Ensure connectivity: extra connections between random room pairs
  for (let i = 0; i < Math.floor(rooms.length / 2); i++) {
    const a = rooms[Math.floor(rng() * rooms.length)];
    const b = rooms[Math.floor(rng() * rooms.length)];
    if (a !== b) {
      carveCorridor(grid, a.cx, a.cy, b.cx, b.cy);
    }
  }

  // Player starts in first room
  const playerStart = { x: rooms[0].cx, y: rooms[0].cy };

  // Stairs in the room farthest from start
  let farthestRoom = rooms[0];
  let maxDist = 0;
  for (const room of rooms) {
    const dist = Math.abs(room.cx - playerStart.x) + Math.abs(room.cy - playerStart.y);
    if (dist > maxDist) {
      maxDist = dist;
      farthestRoom = room;
    }
  }
  const stairsPos = { x: farthestRoom.cx, y: farthestRoom.cy };
  grid[stairsPos.y][stairsPos.x] = TILE.STAIRS;

  // Place enemies and items
  const entities = [];
  const enemyCount = 4 + floorNumber * 2;
  const itemCount = 2 + Math.floor(floorNumber / 2);

  for (let i = 0; i < enemyCount; i++) {
    const pos = findEmptyTile(grid, rooms, entities, rng);
    if (pos) {
      const type = pickEnemyType(floorNumber, rng);
      entities.push(createEnemy(type, pos.x, pos.y, floorNumber, i));
    }
  }

  for (let i = 0; i < itemCount; i++) {
    const pos = findEmptyTile(grid, rooms, entities, rng);
    if (pos) {
      entities.push(createItem(pos.x, pos.y, rng));
    }
  }

  return { grid, rooms, playerStart, stairsPos, entities, width, height };
}

function carveCorridor(grid, x1, y1, x2, y2) {
  let x = x1, y = y1;
  // Random horizontal or vertical first
  if (Math.random() < 0.5) {
    while (x !== x2) { grid[y][x] = TILE.FLOOR; x += x < x2 ? 1 : -1; }
    while (y !== y2) { grid[y][x] = TILE.FLOOR; y += y < y2 ? 1 : -1; }
  } else {
    while (y !== y2) { grid[y][x] = TILE.FLOOR; y += y < y2 ? 1 : -1; }
    while (x !== x2) { grid[y][x] = TILE.FLOOR; x += x < x2 ? 1 : -1; }
  }
  grid[y2][x2] = TILE.FLOOR;
}

function findEmptyTile(grid, rooms, entities, rng) {
  for (let attempt = 0; attempt < 50; attempt++) {
    const room = rooms[Math.floor(rng() * rooms.length)];
    const x = room.x + 1 + Math.floor(rng() * (room.w - 2));
    const y = room.y + 1 + Math.floor(rng() * (room.h - 2));
    if (grid[y][x] === TILE.FLOOR && !entities.some(e => e.x === x && e.y === y)) {
      return { x, y };
    }
  }
  return null;
}

function pickEnemyType(floor, rng) {
  const roll = rng();
  if (floor >= 5 && roll < 0.08) return 'void_lord';
  if (floor >= 4 && roll < 0.20) return 'golem';
  if (floor >= 3 && roll < 0.32) return 'revenant';
  if (floor >= 2 && roll < 0.46) return 'shade';
  if (floor >= 1 && roll < 0.62) return 'wraith';
  if (floor >= 1 && roll < 0.80) return 'bat';
  return 'slime';
}

function createEnemy(type, x, y, floor, id) {
  const base = ENEMY_BLUEPRINTS[type];
  const scale = floor - 1;
  return {
    id: `enemy_${id}`,
    type,
    x, y,
    char: base.char,
    name: base.name,
    color: base.color,
    hp: base.hp + scale * 3,
    maxHp: base.hp + scale * 3,
    atk: base.atk + Math.floor(scale * 1.2),
    def: base.def + Math.floor(scale * 0.5),
    xp: base.xp + scale * 2,
    behavior: base.behavior,
    behaviorCounter: 0,
    poisonTurns: 0,
    poisonDmg: 0,
  };
}

function createItem(x, y, rng) {
  const roll = rng();
  let type, char, name, color;
  if (roll < 0.35) { type = 'potion'; char = '!'; name = 'Health Potion'; color = '#ff4444'; }
  else if (roll < 0.55) { type = 'big_potion'; char = '‼'; name = 'Greater Potion'; color = '#ff6666'; }
  else if (roll < 0.70) { type = 'atk_scroll'; char = '≫'; name = 'Attack Scroll'; color = '#ffaa00'; }
  else if (roll < 0.85) { type = 'def_scroll'; char = '≪'; name = 'Defense Scroll'; color = '#4488ff'; }
  else { type = 'essence'; char = '✦'; name = 'Void Essence'; color = '#cc66ff'; }

  return {
    id: `item_${x}_${y}`,
    type: 'item',
    itemType: type,
    x, y,
    char, name, color,
  };
}

const ENEMY_BLUEPRINTS = {
  slime: {
    char: 's', name: 'Slime', color: '#44dd44',
    hp: 14, atk: 3, def: 0, xp: 8,
    behavior: 'random',
  },
  wraith: {
    char: 'W', name: 'Wraith', color: '#aa66ff',
    hp: 22, atk: 6, def: 1, xp: 18,
    behavior: 'chase',
  },
  shade: {
    char: 'S', name: 'Shade', color: '#44cccc',
    hp: 16, atk: 8, def: 1, xp: 22,
    behavior: 'flank',
  },
  golem: {
    char: 'G', name: 'Golem', color: '#ff8844',
    hp: 45, atk: 12, def: 4, xp: 35,
    behavior: 'slow_chase',
  },
  bat: {
    char: 'b', name: 'Void Bat', color: '#88aaff',
    hp: 10, atk: 4, def: 0, xp: 12,
    behavior: 'chase',
  },
  revenant: {
    char: 'R', name: 'Revenant', color: '#ccd4e0',
    hp: 34, atk: 10, def: 2, xp: 30,
    behavior: 'flank',
  },
  void_lord: {
    char: 'V', name: 'Void Lord', color: '#ff2244',
    hp: 70, atk: 16, def: 6, xp: 60,
    behavior: 'boss',
  },
};
