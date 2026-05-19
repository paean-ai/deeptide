// Pixel Bubble Pop - content data: grid layout, colours, tuning.

const COLS = 10;               // bubbles per even row
const BUB = 36;                // bubble diameter
const ROWH = Math.round(BUB * 0.86);
const VW = COLS * BUB;         // 360
const VH = 560;
const DANGER_Y = VH - 132;     // bubbles past this end the game
const SHOT_SPEED = 620;        // px/s
const DROP_EVERY = 6;          // a new row descends every N shots

const BUB_COLORS = ['#ff6b8b', '#ffd24d', '#5fd9a0', '#7ab0ff', '#c98fff', '#ff9d52'];

// even rows hold COLS bubbles; odd rows hold COLS-1 (shifted half a cell)
function rowLen(r) { return r % 2 === 0 ? COLS : COLS - 1; }
function cellX(r, c) { return (r % 2 === 0 ? BUB / 2 : BUB) + c * BUB; }
function cellY(r) { return BUB / 2 + r * ROWH; }

// 6 hex neighbours of an offset-grid cell
function neighbors(r, c) {
  const odd = r % 2 !== 0;
  return [
    [r, c - 1], [r, c + 1],
    [r - 1, odd ? c : c - 1], [r - 1, odd ? c + 1 : c],
    [r + 1, odd ? c : c - 1], [r + 1, odd ? c + 1 : c],
  ];
}
