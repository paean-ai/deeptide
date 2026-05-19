// Canvas Brick Breaker - localization (English / 中文)
const LANG_KEY = 'canvas-brick-breaker-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BRICK BREAKER',
    score: 'Score', stage: 'Stage', balls: 'Balls', best: 'Best', restart: 'Restart',
    hint: 'Move mouse or drag · Space launches · collect W/M/L/S power cores',
    combo: 'Combo',
    breakAll: 'Break every core brick.',
    stageMsg: (s, n) => `Stage ${s}: ${n} bricks online.`,
    launchHint: 'Press Space or tap to launch',
    gameOver: 'GAME OVER',
    finalScore: s => `Final Score ${s}`,
    pu_wide: 'Wide Paddle', pu_multi: 'Multi Ball',
    pu_laser: 'Laser Paddle', pu_slow: 'Slow Time',
  },
  zh: {
    title: '打砖块',
    score: '分数', stage: '关卡', balls: '小球', best: '最高', restart: '重新开始',
    hint: '移动鼠标或拖动 · 空格发射 · 收集 W/M/L/S 能量核心',
    combo: '连击',
    breakAll: '击碎所有核心砖块。',
    stageMsg: (s, n) => `第 ${s} 关：${n} 块砖块上线。`,
    launchHint: '按空格或点击发射',
    gameOver: '游戏结束',
    finalScore: s => `最终得分 ${s}`,
    pu_wide: '加宽挡板', pu_multi: '多重球',
    pu_laser: '激光挡板', pu_slow: '时间减速',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
// resolve a {k, a} message object
function mt(m) { return m ? t(m.k, ...(m.a || [])) : ''; }

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
