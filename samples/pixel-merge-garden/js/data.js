// Pixel Merge Garden - content data: crops, mutations, greenhouse tiers, orders

const COLS = 5;
const ROWS = 5;
const SIZE = COLS * ROWS;

// Crop ladder. Each tier has a distinct silhouette (topper) and palette so the
// board reads clearly at a glance, the way top-tier merge games stage progress.
const CROPS = [
  { name: 'Sprout',     topper: 'sprout', leaf: '#7fe089', stem: '#3c8a4a', glow: '#cdf6d2' },
  { name: 'Cloverleaf', topper: 'leaf',   leaf: '#62d879', stem: '#2f6d3a', glow: '#aef0b6' },
  { name: 'Dewbud',     topper: 'bud',    leaf: '#5fd3c0', stem: '#2c7d72', glow: '#b6f3ea' },
  { name: 'Bluebell',   topper: 'bloom',  leaf: '#64c7ff', stem: '#315f80', glow: '#c4ecff' },
  { name: 'Glowlily',   topper: 'bloom',  leaf: '#aa7dff', stem: '#594b94', glow: '#ddccff' },
  { name: 'Sunpod',     topper: 'fruit',  leaf: '#f4c85a', stem: '#8c671e', glow: '#ffe6a8' },
  { name: 'Emberberry', topper: 'berry',  leaf: '#ff9266', stem: '#8a4a30', glow: '#ffd0bc' },
  { name: 'Roseheart',  topper: 'bloom',  leaf: '#f0647f', stem: '#8b3148', glow: '#ffc4d0' },
  { name: 'Starfruit',  topper: 'star',   leaf: '#fff4d6', stem: '#9aa0b4', glow: '#ffffff' },
];

// Mutations multiply a crop's value and sale price.
const MUTATIONS = {
  plain:   { label: '',  mult: 1,    color: '#dfe8f5' },
  silver:  { label: 'S', mult: 2.5,  color: '#c9d7e8' },
  gold:    { label: 'G', mult: 6,    color: '#f4c85a' },
  rainbow: { label: 'R', mult: 16,   color: '#ff9ce0' },
};
const MUTATION_RANK = ['plain', 'silver', 'gold', 'rainbow'];

// Greenhouse upgrade tiers - bought with coins, boost income + seed quality.
const GREENHOUSE = [
  { cost: 0,    income: 1.0, seedLuck: 0.00 },
  { cost: 280,  income: 1.6, seedLuck: 0.03 },
  { cost: 1100, income: 2.6, seedLuck: 0.06 },
  { cost: 4200, income: 4.4, seedLuck: 0.10 },
  { cost: 16000, income: 7.6, seedLuck: 0.15 },
  { cost: 60000, income: 13,  seedLuck: 0.21 },
  { cost: 220000, income: 23, seedLuck: 0.28 },
  { cost: 900000, income: 42, seedLuck: 0.36 },
];

const OFFLINE_CAP_SECONDS = 8 * 3600; // bank keeps filling up to 8h while away

function cropName(level) { return CROPS[(level - 1) % CROPS.length].name; }
function cropArt(level)  { return CROPS[(level - 1) % CROPS.length]; }
// Crops cycle palette every 9 tiers but gain a prestige ring each loop.
function cropPrestige(level) { return Math.floor((level - 1) / CROPS.length); }
