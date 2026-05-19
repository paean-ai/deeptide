// Pixel Beat Runner - localization (English / 中文)
const LANG_KEY = 'pixel-beat-runner-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BEAT RUNNER',
    subtitle: 'Tap the lanes on the beat. Make the melody.',
    play: 'START', howto: 'Tap a lane (or D F J K) as its note hits the line.',
    score: 'Score', combo: 'Combo', best: 'Best', accuracy: 'Acc',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'OUT OF SYNC', finalScore: s => `Score: ${s}`,
    maxCombo: c => `Best combo: ${c}`, bestScore: s => `Best: ${s}`,
    again: 'RUN AGAIN',
    perfect: 'PERFECT', good: 'GOOD', miss: 'MISS',
  },
  zh: {
    title: '节拍奔跑',
    subtitle: '踩着节拍点击轨道,亲手奏出旋律。',
    play: '开始', howto: '当音符落到判定线时点击对应轨道(或 D F J K)。',
    score: '分数', combo: '连击', best: '最高', accuracy: '准度',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '节奏脱拍', finalScore: s => `分数：${s}`,
    maxCombo: c => `最高连击：${c}`, bestScore: s => `最高：${s}`,
    again: '再来一次',
    perfect: '完美', good: '不错', miss: '失误',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}
function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (btn) {
    btn.onclick = () => {
      currentLang = currentLang === 'en' ? 'zh' : 'en';
      localStorage.setItem(LANG_KEY, currentLang);
      btn.textContent = currentLang === 'en' ? '中文' : 'EN';
      applyStaticText();
      if (onChange) onChange();
    };
    btn.textContent = currentLang === 'en' ? '中文' : 'EN';
  }
  applyStaticText();
}
