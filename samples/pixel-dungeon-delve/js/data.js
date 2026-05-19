// Pixel Dungeon Delve - content data: enemies, gear, items, tuning

const MAP_W = 38;
const MAP_H = 38;
const VIEW = 13;          // visible tiles across (odd, player centred)
const TILE = 32;          // logical pixels per tile
const FOV_RADIUS = 6;

// Tile codes
const T_WALL = 0, T_FLOOR = 1, T_STAIRS = 2;

const VICTORY_DEPTH = 10; // first reach = victory overlay, then endless

// Enemy roster. `tier` gates the earliest depth a foe appears.
const ENEMIES = {
  rat:      { glyph: 'rat',   tier: 1, hp: 9,  atk: 4,  def: 0, xp: 4,  sight: 5, speed: 1 },
  bat:      { glyph: 'bat',   tier: 1, hp: 7,  atk: 5,  def: 0, xp: 5,  sight: 7, speed: 1, erratic: true },
  skeleton: { glyph: 'skel',  tier: 3, hp: 18, atk: 8,  def: 2, xp: 11, sight: 6, speed: 1 },
  orc:      { glyph: 'orc',   tier: 5, hp: 30, atk: 12, def: 3, xp: 18, sight: 6, speed: 1 },
  wraith:   { glyph: 'wraith',tier: 7, hp: 26, atk: 16, def: 5, xp: 26, sight: 8, speed: 1 },
  dragon:   { glyph: 'dragon',tier: VICTORY_DEPTH, hp: 120, atk: 22, def: 8, xp: 200, sight: 9, speed: 1, boss: true },
};

// Weapons & armour - found on the floor, auto-equipped when stronger.
const WEAPONS = [
  { id: 'dagger',  atk: 3 },
  { id: 'sword',   atk: 6 },
  { id: 'axe',     atk: 10 },
  { id: 'flail',   atk: 15 },
  { id: 'runeblade', atk: 22 },
];
const ARMORS = [
  { id: 'cloth',   def: 1 },
  { id: 'leather', def: 3 },
  { id: 'chain',   def: 6 },
  { id: 'plate',   def: 10 },
  { id: 'aegis',   def: 16 },
];

const PLAYER_BASE = { maxHp: 60, atk: 5, def: 0 };

// XP needed to reach the next level (index = current level).
function xpForLevel(level) { return Math.floor(20 * Math.pow(1.45, level - 1)); }

// Per-depth difficulty multiplier applied to enemy hp / atk.
function depthScale(depth) { return 1 + (depth - 1) * 0.16; }

// How many of each thing to scatter on a floor.
function floorBudget(depth) {
  return {
    rooms: 6 + Math.floor(depth / 2),
    enemies: 5 + Math.floor(depth * 1.3),
    potions: 2 + (depth % 3 === 0 ? 1 : 0),
    gold: 3 + Math.floor(depth / 2),
    gear: depth % 2 === 0 ? 2 : 1,
  };
}
