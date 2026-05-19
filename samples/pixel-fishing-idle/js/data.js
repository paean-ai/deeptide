// Pixel Fishing Idle - content data: zones, rarities, fish, tuning

const W = 900, H = 620;

const ZONES = [
  { id: 'cove',   unlock: 1, sky: ['#17304f', '#295b79'], sea: ['#14506e', '#082b47'], accent: '#65d9ff' },
  { id: 'kelp',   unlock: 2, sky: ['#203d47', '#407566'], sea: ['#0f604f', '#07392e'], accent: '#68da86' },
  { id: 'reef',   unlock: 4, sky: ['#241f4d', '#544587'], sea: ['#29315e', '#111936'], accent: '#b7a7ff' },
  { id: 'crown',  unlock: 7, sky: ['#40294d', '#7d4f62'], sea: ['#263253', '#121827'], accent: '#f4c85a' },
];

const RARITY = {
  common:   { mult: 1,   color: '#9ee8ff', rank: 0 },
  uncommon: { mult: 2.1, color: '#72df89', rank: 1 },
  rare:     { mult: 5,   color: '#f4c85a', rank: 2 },
  epic:     { mult: 12,  color: '#b7a7ff', rank: 3 },
  mythic:   { mult: 30,  color: '#ffffff', rank: 4 },
};

// shape archetypes drive the pixel sprite in art.js: small / round / long /
// flat / jelly / crab / serpent.
const FISH = [
  // Cove
  { id: 'minnow', name: 'Glass Minnow', zone: 0, rarity: 'common',   value: 5,   weight: 48, color: '#65d9ff', shape: 'small' },
  { id: 'carp',   name: 'Copper Carp',  zone: 0, rarity: 'uncommon', value: 15,  weight: 24, color: '#d68a4a', shape: 'round' },
  { id: 'crab',   name: 'Pebble Crab',  zone: 0, rarity: 'common',   value: 9,   weight: 30, color: '#c46a5a', shape: 'crab' },
  { id: 'koi',    name: 'Lantern Koi',  zone: 0, rarity: 'rare',     value: 45,  weight: 8,  color: '#f4c85a', shape: 'round' },
  // Kelp Bay
  { id: 'pike',   name: 'Kelp Pike',    zone: 1, rarity: 'common',   value: 18,  weight: 42, color: '#68da86', shape: 'long' },
  { id: 'perch',  name: 'Reed Perch',   zone: 1, rarity: 'uncommon', value: 34,  weight: 26, color: '#9bd14a', shape: 'small' },
  { id: 'eel',    name: 'Emerald Eel',  zone: 1, rarity: 'rare',     value: 82,  weight: 10, color: '#2ee6a6', shape: 'long' },
  { id: 'thorn',  name: 'Thornfish',    zone: 1, rarity: 'uncommon', value: 40,  weight: 20, color: '#4ab0a0', shape: 'round' },
  // Moon Reef
  { id: 'moonfin',name: 'Moonfin',      zone: 2, rarity: 'uncommon', value: 70,  weight: 28, color: '#b7a7ff', shape: 'flat' },
  { id: 'jelly',  name: 'Glow Jelly',   zone: 2, rarity: 'common',   value: 40,  weight: 34, color: '#7fd9ff', shape: 'jelly' },
  { id: 'tang',   name: 'Prism Tang',   zone: 2, rarity: 'rare',     value: 150, weight: 12, color: '#5ad0e0', shape: 'flat' },
  { id: 'ray',    name: 'Star Ray',     zone: 2, rarity: 'epic',     value: 260, weight: 6,  color: '#edf4ff', shape: 'flat' },
  // Sunken Crown
  { id: 'bass',   name: 'Gilded Bass',  zone: 3, rarity: 'rare',     value: 320, weight: 16, color: '#e8c878', shape: 'round' },
  { id: 'lurker', name: 'Abyss Lurker', zone: 3, rarity: 'epic',     value: 560, weight: 7,  color: '#6a5fa8', shape: 'long' },
  { id: 'levi',   name: 'Crown Levi',   zone: 3, rarity: 'mythic',   value: 1200,weight: 2,  color: '#fff7c4', shape: 'serpent' },
];

const UPGRADE = {
  rod:  { base: 24,  name: 'Rod' },
  bait: { base: 34,  name: 'Bait' },
  boat: { base: 90,  name: 'Boat' },
  crew: { base: 120, name: 'Crew' },
};

const OFFLINE_CAP_SECONDS = 8 * 3600;

function fishById(id) { return FISH.find(f => f.id === id); }
