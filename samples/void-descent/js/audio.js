// === VOID DESCENT - Synthesized Sound Effects ===

let audioCtx = null;

function getCtx() {
  if (!audioCtx) {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  }
  return audioCtx;
}

function playTone(freq, duration, type, vol, slide) {
  try {
    const ctx = getCtx();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = type || 'square';
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(vol || 0.08, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
    if (slide) osc.frequency.linearRampToValueAtTime(slide, ctx.currentTime + duration);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start(ctx.currentTime);
    osc.stop(ctx.currentTime + duration);
  } catch(e) { /* audio not available */ }
}

function playNoise(duration, vol) {
  try {
    const ctx = getCtx();
    const bufferSize = ctx.sampleRate * duration;
    const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      data[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / bufferSize, 2);
    }
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(vol || 0.06, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
    source.connect(gain);
    gain.connect(ctx.destination);
    source.start(ctx.currentTime);
  } catch(e) {}
}

const SFX = {
  attack() { playTone(180, 0.08, 'square', 0.07, 60); },
  crit() { playTone(300, 0.1, 'square', 0.1, 80); playTone(450, 0.06, 'square', 0.06, 120); },
  kill() { playTone(600, 0.06, 'square', 0.06); playTone(900, 0.04, 'square', 0.04); playNoise(0.1, 0.05); },
  hit() { playTone(80, 0.1, 'sawtooth', 0.06, 30); },
  dodge() { playTone(500, 0.05, 'sine', 0.04, 800); },
  heal() { playTone(400, 0.08, 'sine', 0.05, 600); playTone(600, 0.1, 'sine', 0.04, 900); },
  pickup() { playTone(500, 0.04, 'sine', 0.04, 800); playTone(800, 0.04, 'sine', 0.04, 1000); },
  stairs() { playTone(220, 0.1, 'triangle', 0.06, 440); playTone(330, 0.15, 'triangle', 0.06, 660); },
  death() { playTone(200, 0.3, 'sawtooth', 0.1, 40); playNoise(0.4, 0.08); },
  shield() { playTone(300, 0.06, 'sine', 0.04, 500); },
  phoenix() { playTone(500, 0.15, 'triangle', 0.08, 800); playTone(800, 0.2, 'triangle', 0.08, 1200); },
  move() { playTone(120, 0.03, 'sine', 0.02, 60); },
};
