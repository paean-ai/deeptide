// Game state persistence via localStorage

const STORAGE_KEY = 'void_descent_save';

const DEFAULT_STATE = {
  floor: 1,
  player: {
    hp: 100,
    maxHp: 100,
    atk: 10,
    def: 2,
    critChance: 0.05,
    dodgeChance: 0,
    lifesteal: 0,
    thorns: 0,
    regen: 0,
    regenTurns: 0,
    swiftChance: 0,
    doubleStrikeChance: 0,
    shield: 0,
    bonusAtkPerKill: 0,
  },
  upgrades: {},
  essence: 0,
  kills: 0,
  totalFloorsCleared: 0,
};

function saveGame(state) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

function loadGame() {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function clearGame() {
  localStorage.removeItem(STORAGE_KEY);
}

function newGameState() {
  return JSON.parse(JSON.stringify(DEFAULT_STATE));
}
