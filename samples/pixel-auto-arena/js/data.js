// Pixel Auto Arena - content data: classes, unit roster, economy tuning

const VW = 480, VH = 540;

// Three unit classes, each with a 2 / 4 synergy threshold.
const CLASSES = {
  beast: { color: '#e0843f', t2: { atk: 4 }, t4: { atk: 10 } },
  mech:  { color: '#5fa8e0', t2: { hp: 14 },  t4: { hp: 36 } },
  mage:  { color: '#b87ae0', t2: { atk: 6 },  t4: { atk: 15 } },
};

// Unit roster: 3 per class across tiers 1-3. cost = tier.
const UNITS = [
  { id: 'wolf',   cls: 'beast', tier: 1, hp: 20, atk: 6,  glyph: 'wolf' },
  { id: 'boar',   cls: 'beast', tier: 2, hp: 38, atk: 9,  glyph: 'boar' },
  { id: 'bear',   cls: 'beast', tier: 3, hp: 64, atk: 14, glyph: 'bear' },
  { id: 'bolt',   cls: 'mech',  tier: 1, hp: 26, atk: 4,  glyph: 'bolt' },
  { id: 'turret', cls: 'mech',  tier: 2, hp: 34, atk: 11, glyph: 'turret' },
  { id: 'titan',  cls: 'mech',  tier: 3, hp: 86, atk: 12, glyph: 'titan' },
  { id: 'imp',    cls: 'mage',  tier: 1, hp: 15, atk: 9,  glyph: 'imp' },
  { id: 'sage',   cls: 'mage',  tier: 2, hp: 28, atk: 13, glyph: 'sage' },
  { id: 'archon', cls: 'mage',  tier: 3, hp: 46, atk: 21, glyph: 'archon' },
];

const SHOP_SLOTS = 5;
const START_LIVES = 5;
const REROLL_COST = 1;
const MAX_TEAM = 6;

// star scaling: 1-star base, then x2, x3.4
const STAR_MUL = [0, 1, 2, 3.4];

function unitById(id) { return UNITS.find(u => u.id === id); }
function teamCap(round) { return Math.min(MAX_TEAM, 3 + Math.floor(round / 3)); }
function goldFor(round) { return Math.min(13, 5 + Math.floor(round * 0.7)); }
// highest unit tier that can appear in the shop this round
function shopTier(round) { return round < 3 ? 1 : round < 7 ? 2 : 3; }
