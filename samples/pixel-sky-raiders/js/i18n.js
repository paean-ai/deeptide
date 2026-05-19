// Pixel Sky Raiders - localization (English / 中文)
const LANG_KEY = 'pixel-sky-raiders-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'SKY RAIDERS',
    subtitle: 'Pilot the raider. Hold nothing back.',
    play: 'LAUNCH', howto: 'Move to aim — guns fire on their own. Tap BOMB to clear the screen.',
    score: 'Score', wave: 'Wave', hp: 'Hull', best: 'Best',
    bomb: 'BOMB', paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'RAIDER DOWN', waveCleared: w => `WAVE ${w}`,
    bossIncoming: 'WARNING — BOSS', bossDown: 'BOSS DESTROYED',
    finalScore: s => `Final score: ${s}`, reachedWave: w => `Reached wave ${w}`,
    bestScore: s => `Best: ${s}`, again: 'FLY AGAIN',
    powerUp: 'WEAPON UP', gotBomb: '+1 BOMB', shieldOn: 'SHIELD',
    weaponMax: 'WEAPON MAX',
  },
  zh: {
    title: '天空突袭者',
    subtitle: '驾驶突袭机，火力全开。',
    play: '起飞', howto: '移动瞄准 —— 武器自动开火。点击炸弹清屏。',
    score: '分数', wave: '波次', hp: '装甲', best: '最高',
    bomb: '炸弹', paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '突袭机坠毁', waveCleared: w => `第 ${w} 波`,
    bossIncoming: '警告 —— 首领来袭', bossDown: '首领已击破',
    finalScore: s => `最终分数：${s}`, reachedWave: w => `到达第 ${w} 波`,
    bestScore: s => `最高：${s}`, again: '再次起飞',
    powerUp: '武器升级', gotBomb: '+1 炸弹', shieldOn: '护盾',
    weaponMax: '武器满级',
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
