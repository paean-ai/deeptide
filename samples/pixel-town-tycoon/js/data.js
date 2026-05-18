// Pixel Town Tycoon - game data: buildings, resources, ranks, quests

const TILE = 56;
const GRID_W = 11;
const GRID_H = 8;

// ---- Resources ---------------------------------------------------------
// raw -> intermediate -> goods (goods are sold at the market)
// raw resources (tier 0) feed processors; only processed goods sell at market
const RESOURCES = {
  wheat: { icon: '🌾', color: '#e8c44a', tier: 0 },
  water: { icon: '💧', color: '#5fb8e0', tier: 0 },
  wood:  { icon: '🪵', color: '#a8753a', tier: 0 },
  ore:   { icon: '⛏', color: '#9aa4b4', tier: 0 },
  flour: { icon: '🥖', color: '#e8d8a8', tier: 1, sell: 11 },
  plank: { icon: '🧱', color: '#caa14a', tier: 1, sell: 13 },
  bread: { icon: '🍞', color: '#d99a4a', tier: 2, sell: 26 },
  tools: { icon: '🔧', color: '#7fc8e0', tier: 2, sell: 60 },
};
const RES_IDS = Object.keys(RESOURCES);

// ---- Buildings ---------------------------------------------------------
// kind: home | producer | processor | market | storage
// out: resource produced;  in: {res: amount} consumed per cycle
// adj: neighbour type granting +15% adjacency bonus
const BUILDINGS = {
  cottage:  { kind: 'home', cost: 55,  pop: 4, color: '#c97f4a', icon: '🏠', rank: 0 },
  farm:     { kind: 'producer', cost: 80,  out: 'wheat', rate: 2.2, adj: 'well',   color: '#7fae3e', icon: '🌱', rank: 0, workers: 1 },
  well:     { kind: 'producer', cost: 70,  out: 'water', rate: 2.4, adj: 'farm',   color: '#5fa8d8', icon: '⛲', rank: 0, workers: 1 },
  lumber:   { kind: 'producer', cost: 85,  out: 'wood',  rate: 2.0, adj: 'lumber', color: '#8a6a3a', icon: '🌲', rank: 0, workers: 1 },
  market:   { kind: 'market',   cost: 130, rate: 3.5, adj: 'cottage', color: '#d8b24a', icon: '🏪', rank: 0, workers: 1 },
  mine:     { kind: 'producer', cost: 160, out: 'ore',   rate: 1.6, adj: 'mine',   color: '#8893a4', icon: '🪨', rank: 2, workers: 2 },
  mill:     { kind: 'processor', cost: 175, in: { wheat: 3 }, out: 'flour', rate: 2.0, adj: 'farm', color: '#c8b27a', icon: '🏯', rank: 0, workers: 1 },
  sawmill:  { kind: 'processor', cost: 185, in: { wood: 3 },  out: 'plank', rate: 2.0, adj: 'lumber', color: '#a8854a', icon: '🪚', rank: 1, workers: 1 },
  bakery:   { kind: 'processor', cost: 260, in: { flour: 2, water: 2 }, out: 'bread', rate: 1.6, adj: 'mill', color: '#e0a85a', icon: '🥐', rank: 2, workers: 2 },
  smithy:   { kind: 'processor', cost: 320, in: { ore: 2, plank: 2 }, out: 'tools', rate: 1.3, adj: 'mine', color: '#6fa8c8', icon: '⚒', rank: 3, workers: 2 },
  warehouse:{ kind: 'storage',  cost: 220, cap: 80, adj: 'warehouse', color: '#7a8294', icon: '📦', rank: 1, workers: 0 },
};
const BUILDING_IDS = Object.keys(BUILDINGS);

// effective production rate at a given level
function buildingRate(base, level) {
  return base * (1 + (level - 1) * 0.55);
}
// upgrade cost to reach `level+1`
function upgradeCost(typeId, level) {
  return Math.round(BUILDINGS[typeId].cost * (0.8 + level * 0.85));
}
function sellValue(typeId, level) {
  return Math.round(BUILDINGS[typeId].cost * 0.55 * level);
}

// ---- Town ranks --------------------------------------------------------
// reaching `coinsEarned` raises the rank and unlocks buildings
const RANKS = [
  { coins: 0,    name: ['Hamlet', '小村庄'] },
  { coins: 600,  name: ['Village', '村落'] },
  { coins: 2200, name: ['Township', '城镇'] },
  { coins: 6000, name: ['Town', '都市'] },
  { coins: 14000,name: ['City', '大都会'] },
];

// ---- Quests ------------------------------------------------------------
// type: build (count of a type) | pop | stock (resource amount) | earned (coins)
// type: build (count of a type) | pop | produced (total made) | earned (coins)
const QUESTS = [
  { type: 'build', target: 'cottage', n: 2, reward: 130,
    text: ['Build 2 Cottages for workers', '建造 2 座小屋安置工人'] },
  { type: 'build', target: 'farm', n: 1, reward: 100,
    text: ['Build a Farm', '建造一座农场'] },
  { type: 'build', target: 'mill', n: 1, reward: 170,
    text: ['Build a Mill to process wheat', '建造磨坊加工小麦'] },
  { type: 'build', target: 'market', n: 1, reward: 200,
    text: ['Build a Market to sell goods', '建造集市出售商品'] },
  { type: 'produced', target: 'flour', n: 30, reward: 240,
    text: ['Produce 30 Flour', '累计生产 30 单位面粉'] },
  { type: 'earned', n: 1500, reward: 320,
    text: ['Earn 1500 coins total', '累计赚取 1500 金币'] },
  { type: 'pop', n: 24, reward: 400,
    text: ['Reach 24 Population', '人口达到 24'] },
  { type: 'build', target: 'bakery', n: 1, reward: 450,
    text: ['Build a Bakery', '建造一座面包房'] },
  { type: 'produced', target: 'bread', n: 60, reward: 600,
    text: ['Produce 60 Bread', '累计生产 60 单位面包'] },
  { type: 'build', target: 'smithy', n: 1, reward: 750,
    text: ['Build a Smithy', '建造一座铁匠铺'] },
  { type: 'earned', n: 9000, reward: 1100,
    text: ['Earn 9000 coins total', '累计赚取 9000 金币'] },
  { type: 'earned', n: 24000, reward: 2600,
    text: ['Earn 24000 coins — a thriving city!', '累计赚取 24000 金币 — 繁荣都市！'] },
];

const BASE_STORAGE = 60;
const TICK_SECONDS = 1.0;          // production tick
const OFFLINE_CAP = 8 * 3600;      // max offline seconds credited
