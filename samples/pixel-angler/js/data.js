// Pixel Angler - fish species, waters and upgrade economy.

const VW = 360, VH = 480;

// Fish: size scales the sprite & value; diff drives how wildly it darts in the
// reeling minigame; zone is the waters tier it belongs to.
const FISH = [
  { id: 'minnow',  name: ['Minnow', '米诺鱼'],   color: '#9fd0e0', size: 0.62, value: 9,   diff: 0.6, zone: 0 },
  { id: 'perch',   name: ['Perch', '河鲈'],      color: '#79b35a', size: 0.85, value: 16,  diff: 0.95, zone: 0 },
  { id: 'sunfish', name: ['Sunfish', '太阳鱼'],  color: '#f2cf3f', size: 0.80, value: 24,  diff: 1.15, zone: 0 },
  { id: 'bass',    name: ['Bass', '黑鲈'],       color: '#5f7f4a', size: 1.05, value: 38,  diff: 1.35, zone: 1 },
  { id: 'trout',   name: ['Trout', '鳟鱼'],      color: '#d98a6a', size: 1.05, value: 48,  diff: 1.55, zone: 1 },
  { id: 'pike',    name: ['Pike', '梭子鱼'],     color: '#85925a', size: 1.30, value: 70,  diff: 1.8,  zone: 1 },
  { id: 'tuna',    name: ['Tuna', '金枪鱼'],     color: '#4f86b8', size: 1.50, value: 110, diff: 2.05, zone: 2 },
  { id: 'marlin',  name: ['Marlin', '马林鱼'],   color: '#6f5fd0', size: 1.72, value: 165, diff: 2.35, zone: 2 },
  { id: 'shark',   name: ['Shark', '鲨鱼'],      color: '#8893a4', size: 1.95, value: 250, diff: 2.65, zone: 2 },
];

const WATERS = [
  { name: ['Calm Shallows', '宁静浅滩'], unlockCost: 0 },
  { name: ['Open Lake', '开阔湖面'],     unlockCost: 320 },
  { name: ['Deep Trench', '深海沟壑'],   unlockCost: 1200 },
];

const MAX_UPGRADE = 6;
function rodCost(level) { return 70 + level * 85; }   // wider catch bar
function reelCost(level) { return 80 + level * 95; }  // faster reel-in

// Pick a fish for the given waters tier — the current tier's fish are common,
// shallower fish turn up occasionally.
function pickFish(waters, rng) {
  const pool = [];
  for (const f of FISH) {
    if (f.zone > waters) continue;
    const weight = f.zone === waters ? 3 : 1;
    for (let i = 0; i < weight; i++) pool.push(f);
  }
  return pool[(rng() * pool.length) | 0];
}
