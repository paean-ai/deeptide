// Pixel Pinball - localization (English / 中文)
const LANG_KEY = 'pixel-pinball-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'PIXEL PINBALL',
    subtitle: 'Keep the ball alive. Rack up the score.',
    howto: 'Hold to charge the launch. Tap the left / right side of the table to flip.',
    keys: 'Keys: A / D or ← / → flip · hold Space to launch',
    play: 'PLAY', launch: 'HOLD TO LAUNCH', again: 'PLAY AGAIN', menu: 'MENU',
    score: 'SCORE', best: 'BEST', balls: 'BALLS',
    ballLost: 'BALL LOST', lastBall: 'LAST BALL!',
    gameOver: 'GAME OVER', targetBonus: 'TARGET BONUS!',
    finalScore: s => `Final score: ${s}`,
    newBest: 'NEW BEST!',
  },
  zh: {
    title: '像素弹珠',
    subtitle: '别让弹珠落下,刷高分数。',
    howto: '按住蓄力发射。点击桌面左 / 右侧拨动挡板。',
    keys: '按键:A / D 或 ← / → 拨挡板 · 按住空格发射',
    play: '开始', launch: '按住发射', again: '再玩一次', menu: '菜单',
    score: '分数', best: '最高', balls: '弹珠',
    ballLost: '失去弹珠', lastBall: '最后一颗！',
    gameOver: '游戏结束', targetBonus: '标靶奖励！',
    finalScore: s => `最终分数：${s}`,
    newBest: '新纪录！',
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
