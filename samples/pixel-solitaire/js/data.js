// Pixel Solitaire - layout constants and card definitions.

const VW = 360, VH = 480;

// Suits 0-3. Hearts and diamonds are red; spades and clubs are black.
const SUITS = [
  { id: 'spade',   red: false },
  { id: 'heart',   red: true  },
  { id: 'diamond', red: true  },
  { id: 'club',    red: false },
];
const RANK_LABELS = ['', 'A', '2', '3', '4', '5', '6', '7', '8', '9', '10', 'J', 'Q', 'K'];

// Card and board geometry.
const CARD_W = 46, CARD_H = 62;
const COL_GAP = 2;
const BOARD_X = Math.round((VW - (7 * CARD_W + 6 * COL_GAP)) / 2);
const TOP_Y = 50;
const TABLEAU_Y = 124;
const TABLEAU_BOTTOM = 470;
const DOWN_OFF = 8;       // vertical step for a face-down tableau card
const MAX_UP_OFF = 18;    // vertical step for a face-up tableau card

function colX(c) { return BOARD_X + c * (CARD_W + COL_GAP); }

function isRed(card) { return SUITS[card.suit].red; }
function oppositeColor(a, b) { return isRed(a) !== isRed(b); }
