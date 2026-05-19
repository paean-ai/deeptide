// Pixel Turbo Racer - localization (English / 中文)
const LANG_KEY = 'pixel-turbo-racer-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'TURBO RACER',
    subtitle: 'Weave the traffic. Chase the combo. Never lift.',
    play: 'START ENGINE', howto: 'Steer to dodge — near-misses build your combo. Tap NITRO for a burst.',
    score: 'Score', dist: 'Dist', speed: 'Speed', best: 'Best', combo: 'Combo',
    nitro: 'NITRO', paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    crashed: 'WRECKED', nearMiss: 'NEAR MISS', boost: 'BOOST!',
    finalScore: s => `Score: ${s}`, distRun: d => `Distance: ${d} m`,
    bestScore: s => `Best: ${s}`, again: 'RACE AGAIN',
    metres: 'm',
  },
  zh: {
    title: '极速赛车',
    subtitle: '在车流中穿梭，连击不断，油门不松。',
    play: '点火启动', howto: '转向闪避 —— 擦身而过累积连击。点击氮气加速冲刺。',
    score: '分数', dist: '距离', speed: '速度', best: '最高', combo: '连击',
    nitro: '氮气', paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    crashed: '撞毁', nearMiss: '极限擦肩', boost: '加速！',
    finalScore: s => `分数：${s}`, distRun: d => `距离：${d} 米`,
    bestScore: s => `最高：${s}`, again: '再来一局',
    metres: '米',
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
