// Pixel Codebreaker - Mastermind levels, code generation and feedback.

const VW = 360, VH = 480;

const PEG_COLORS = ['#e8554f', '#4a9be8', '#5fc06e', '#f2cf3f',
                    '#9a6cd8', '#ef9b3e', '#4fd6d6'];

// Each level: code length, how many colours are in play, guess attempts, and
// a seed for the hidden code (so a level is the same puzzle every time).
const LEVELS = [
  { len: 3, colors: 5, attempts: 10, seed: 1733 },
  { len: 4, colors: 5, attempts: 10, seed: 2914 },
  { len: 4, colors: 6, attempts: 10, seed: 3508 },
  { len: 4, colors: 6, attempts: 9,  seed: 4641 },
  { len: 5, colors: 6, attempts: 10, seed: 5277 },
  { len: 5, colors: 6, attempts: 9,  seed: 6820 },
  { len: 5, colors: 7, attempts: 10, seed: 7395 },
  { len: 5, colors: 7, attempts: 9,  seed: 8166 },
  { len: 5, colors: 7, attempts: 8,  seed: 9504 },
  { len: 6, colors: 6, attempts: 10, seed: 10732 },
  { len: 6, colors: 7, attempts: 9,  seed: 12085 },
  { len: 6, colors: 7, attempts: 8,  seed: 13471 },
  { len: 6, colors: 7, attempts: 9,  seed: 14860 },
  { len: 6, colors: 7, attempts: 8,  seed: 16238 },
  { len: 6, colors: 7, attempts: 8,  seed: 17715 },
  { len: 6, colors: 7, attempts: 9,  seed: 19173 },
  { len: 6, colors: 7, attempts: 8,  seed: 20694 },
  { len: 6, colors: 7, attempts: 7,  seed: 22315 },
  { len: 6, colors: 7, attempts: 8,  seed: 23842 },
  { len: 6, colors: 7, attempts: 7,  seed: 25390 },
  { len: 6, colors: 7, attempts: 6,  seed: 26937 },
];
const LEVEL_COUNT = LEVELS.length;

function seededRandom(seed) {
  let s = seed % 2147483647;
  if (s <= 0) s += 2147483646;
  return function () {
    s = (s * 16807) % 2147483647;
    return (s - 1) / 2147483646;
  };
}

// The hidden code for a level — `len` colour indices in [0, colors).
function makeCode(level) {
  const rng = seededRandom(level.seed);
  const code = [];
  for (let i = 0; i < level.len; i++) code.push((rng() * level.colors) | 0);
  return code;
}

// Mastermind feedback: black = right colour & spot, white = right colour wrong
// spot. Handles repeated colours correctly.
function feedback(guess, code) {
  let black = 0;
  const codeLeft = {}, guessLeft = {};
  for (let i = 0; i < code.length; i++) {
    if (guess[i] === code[i]) {
      black++;
    } else {
      codeLeft[code[i]] = (codeLeft[code[i]] || 0) + 1;
      guessLeft[guess[i]] = (guessLeft[guess[i]] || 0) + 1;
    }
  }
  let white = 0;
  for (const c in guessLeft) {
    if (codeLeft[c]) white += Math.min(codeLeft[c], guessLeft[c]);
  }
  return { black, white };
}
