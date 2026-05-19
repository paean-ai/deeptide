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
];
const BOMB = { color: '#23232c', spark: '#ff8f4a', r: 21 };

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
