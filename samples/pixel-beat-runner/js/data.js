// Pixel Beat Runner - content data: lanes, timing windows, tuning

const VW = 420, VH = 640;
const LANES = 4;
const LANE_W = VW / LANES;
const HIT_Y = VH - 120;        // y of the judgement line
const SPEED = 470;             // note fall speed px/s

// Timing windows (seconds of offset from the perfect moment).
const W_PERFECT = 0.055;
const W_GOOD = 0.125;
const W_MISS = 0.20;           // past this, the note is a miss

// Each lane has a colour and a musical pitch (C major: C E G C).
const LANE_COLOR = ['#ff6b8b', '#ffd24d', '#5fd9a0', '#7ab0ff'];
const LANE_HZ = [262, 330, 392, 523];

const START_BPM = 100;
const MAX_BPM = 188;
const BPM_STEP = 6;            // bpm gained every RAMP_EVERY seconds
const RAMP_EVERY = 15;

const HEALTH_MAX = 100;
const HP_MISS = -15;
const HP_GOOD = 3;
const HP_PERFECT = 5;

function beatInterval(bpm) { return 60 / bpm; }
