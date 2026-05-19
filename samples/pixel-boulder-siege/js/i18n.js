// Pixel Boulder Siege - localization (English / 中文)
const LANG_KEY = 'pixel-boulder-siege-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'BOULDER SIEGE',
    subtitle: 'Smash the fortress. Squash every goblin.',
    howto: 'Drag toward the fortress to aim — pull farther for more power — and release to fling a boulder.',
    keys: 'Glass shatters, wood splinters, stone holds. Topple towers to reach goblins.',
    play: 'SIEGE', again: 'RETRY', menu: 'MENU',
    score: 'SCORE', round: 'ROUND',
    gameOver: 'SIEGE FAILED',
    roundClear: 'FORTRESS DOWN!',
    finalLine: (sc, rd) => `Score ${sc} · reached round ${rd}`,
    bestLine: sc => `Best ${sc}`,
    newBest: 'NEW BEST!',
    outOfShots: 'OUT OF BOULDERS',
  },
  zh: {
    title: '投石攻城',
    subtitle: '砸碎堡垒,踩扁每一只哥布林。',
    howto: '朝堡垒方向拖动来瞄准 —— 拉得越远力量越大 —— 松手即可投出巨石。',
    keys: '玻璃易碎、木头会裂、石头坚固。推倒高塔才能砸到哥布林。',
    play: '攻城', again: '重来', menu: '菜单',
    score: '分数', round: '回合',
    gameOver: '攻城失败',
    roundClear: '堡垒陷落！',
    finalLine: (sc, rd) => `得分 ${sc} · 抵达第 ${rd} 回合`,
    bestLine: sc => `最高 ${sc}`,
    newBest: '新纪录！',
    outOfShots: '巨石用尽',
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
