// Pixel Fruit Slash - object types and spawn pacing.

const VW = 360, VH = 480;
const GRAVITY = 540;

// Fruit archetypes: a body colour, a juicier inner colour, and a radius.
const FRUITS = [
  { id: 'apple',  color: '#e8554f', inner: '#ff9b8a', r: 23 },
  { id: 'lemon',  color: '#f2cf3f', inner: '#fff09a', r: 20 },
  { id: 'melon',  color: '#5fc06e', inner: '#b6f0a8', r: 26 },
  { id: 'berry',  color: '#9a6cd8', inner: '#cdaef0', r: 18 },
  { id: 'orange', color: '#ef9b3e', inner: '#ffd28a', r: 21 },
  { id: 'dragon', color: '#ff7fb8', inner: '#fff7ed', r: 22 },
];
const BOMB = { color: '#23232c', spark: '#ff8f4a', r: 21 };
// A rare golden fruit pays 3 x the regular score on a clean slice. Visually
// it pulses with a halo so the player can spot it amid the spawn wave.
const GOLD = { id: 'gold', color: '#f4d27b', inner: '#fff0c8', r: 19 };
const GOLD_MULT = 3;

// Seconds between spawn waves — tightens as the run goes on.
function spawnInterval(time) {
  return Math.max(0.62, 1.45 - time * 0.012);
}
// Objects per wave — 1 early, up to 4 deep in a run.
function waveSize(time) {
  return 1 + Math.min(3, Math.floor(time / 22));
}
// Chance any single spawned object is a bomb.
function bombChance(time) {
  return Math.min(0.24, 0.05 + time * 0.0045);
}
// Chance any single non-bomb spawn is the rare golden fruit (3-7%).
function goldChance(time) {
  return Math.min(0.07, 0.03 + time * 0.0005);
}
