// Pixel Deep Miner - content data: world, blocks, ores, upgrades, tuning

const TILE = 34;
const COLS = 16;
const SKY = 4;             // sky rows above the grass line
const ROWS = 220;          // total world height in tiles
const GRASS_ROW = SKY;     // solid grass surface
const BEDROCK_ROW = ROWS - 2;

// Block ids. 0 = empty/air.
const B_EMPTY = 0, B_GRASS = 1, B_DIRT = 2, B_STONE = 3, B_HARD = 4,
      B_LAVA = 5, B_BEDROCK = 6,
      B_COPPER = 7, B_IRON = 8, B_SILVER = 9, B_GOLD = 10, B_GEM = 11;

const BLOCKS = {
  [B_GRASS]:   { solid: 1, hardness: 1.0,      color: '#5a8f3c', top: '#7dc05a' },
  [B_DIRT]:    { solid: 1, hardness: 1.0,      color: '#7a5230', top: '#8c5f38' },
  [B_STONE]:   { solid: 1, hardness: 2.4,      color: '#5c5f6b', top: '#6c6f7d' },
  [B_HARD]:    { solid: 1, hardness: 4.6,      color: '#41434f', top: '#52545f' },
  [B_LAVA]:    { solid: 1, hardness: 2.0,      color: '#d8531f', top: '#ff8a3c', danger: 1 },
  [B_BEDROCK]: { solid: 1, hardness: Infinity, color: '#1f2027', top: '#2a2b33' },
  [B_COPPER]:  { solid: 1, hardness: 1.8, color: '#5c5f6b', ore: 'copper' },
  [B_IRON]:    { solid: 1, hardness: 2.8, color: '#5c5f6b', ore: 'iron' },
  [B_SILVER]:  { solid: 1, hardness: 3.6, color: '#41434f', ore: 'silver' },
  [B_GOLD]:    { solid: 1, hardness: 4.4, color: '#41434f', ore: 'gold' },
  [B_GEM]:     { solid: 1, hardness: 5.6, color: '#41434f', ore: 'gem' },
};

const ORES = {
  copper: { value: 16,  weight: 1, color: '#e08a4a', glow: '#ffb27a', minDepth: 2 },
  iron:   { value: 44,  weight: 1, color: '#d9dde6', glow: '#ffffff', minDepth: 14 },
  silver: { value: 120, weight: 1, color: '#b6e3f0', glow: '#e6ffff', minDepth: 38 },
  gold:   { value: 320, weight: 2, color: '#f4c85a', glow: '#ffe9a0', minDepth: 70 },
  gem:    { value: 950, weight: 2, color: '#ff7ad0', glow: '#ffc4ec', minDepth: 110 },
};

// Upgrade tracks. index 0 = starting level (already owned).
const UPGRADES = {
  drill:    { name: 'Drill',    levels: [{ power: 1.0 }, { power: 1.7 }, { power: 2.6 }, { power: 3.8 }, { power: 5.4 }, { power: 7.5 }],
              cost: [0, 130, 420, 1300, 4200, 13000] },
  fuel:     { name: 'Fuel Tank', levels: [{ cap: 100 }, { cap: 150 }, { cap: 220 }, { cap: 320 }, { cap: 460 }, { cap: 650 }],
              cost: [0, 110, 360, 1100, 3400, 10000] },
  cargo:    { name: 'Cargo Hold', levels: [{ cap: 12 }, { cap: 20 }, { cap: 32 }, { cap: 50 }, { cap: 78 }, { cap: 120 }],
              cost: [0, 120, 400, 1250, 3900, 11500] },
  hull:     { name: 'Hull',     levels: [{ hp: 100, lavaResist: 0.0 }, { hp: 150, lavaResist: 0.25 }, { hp: 220, lavaResist: 0.45 }, { hp: 320, lavaResist: 0.6 }, { hp: 460, lavaResist: 0.75 }, { hp: 650, lavaResist: 0.9 }],
              cost: [0, 140, 460, 1400, 4400, 13500] },
  thruster: { name: 'Thruster', levels: [{ fuelPerTile: 1.6 }, { fuelPerTile: 1.2 }, { fuelPerTile: 0.9 }, { fuelPerTile: 0.65 }, { fuelPerTile: 0.45 }, { fuelPerTile: 0.3 }],
              cost: [0, 100, 320, 950, 2900, 8800] },
};

const FUEL_IDLE = 0.55;      // fuel/sec just being underground
const FUEL_PER_DIG = 0.7;    // fuel per block drilled
const FUEL_PER_MOVE = 0.35;  // fuel per horizontal move
const FALL_SAFE = 4;         // tiles you can fall without damage
const FALL_DMG = 9;          // damage per tile past the safe distance

function idx(x, y) { return y * COLS + x; }

// depth in metres for a given row (surface = 0)
function depthOf(row) { return Math.max(0, row - GRASS_ROW); }
