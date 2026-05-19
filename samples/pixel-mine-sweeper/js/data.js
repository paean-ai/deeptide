// Pixel Mine Sweeper - level campaign data.
//
// Nine hand-tuned levels on an escalating curve: growing boards and rising
// mine density (~15% -> ~22%, classic beginner -> expert range). star2 / star3
// are clear-time thresholds in seconds for the 2- and 3-star ratings.

const VW = 360, VH = 480;

const LEVELS = [
  { cols: 8,  rows: 8,  mines: 10, star2: 75,  star3: 35  },
  { cols: 9,  rows: 9,  mines: 13, star2: 95,  star3: 45  },
  { cols: 10, rows: 10, mines: 16, star2: 120, star3: 60  },
  { cols: 10, rows: 10, mines: 20, star2: 150, star3: 75  },
  { cols: 11, rows: 11, mines: 24, star2: 180, star3: 90  },
  { cols: 12, rows: 12, mines: 30, star2: 220, star3: 110 },
  { cols: 12, rows: 12, mines: 34, star2: 260, star3: 130 },
  { cols: 13, rows: 13, mines: 38, star2: 300, star3: 150 },
  { cols: 14, rows: 14, mines: 44, star2: 360, star3: 180 },
];
const LEVEL_COUNT = LEVELS.length;
