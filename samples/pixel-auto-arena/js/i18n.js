// Pixel Auto Arena - localization (English / 中文)
const LANG_KEY = 'pixel-auto-arena-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'AUTO ARENA',
    subtitle: 'Draft a squad. They fight on their own.',
    play: 'ENTER ARENA', howto: 'Buy units, merge three alike, tap two to swap. Then battle.',
    round: 'Round', gold: 'Gold', lives: 'Lives', best: 'Best',
    battle: 'BATTLE', reroll: 'Reroll', sell: 'Sell', shopTitle: 'Recruit',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    win: 'VICTORY', lose: 'DEFEAT', draw: 'DRAW', next: 'NEXT ROUND',
    gameOver: 'ARENA CLOSED', reachedRound: r => `You fell on round ${r}.`,
    bestRound: r => `Best round: ${r}`, again: 'FIGHT AGAIN',
    teamFull: 'Squad is full.', needGold: 'Not enough gold.', merged: 'Merged!',
    selectSwap: 'Tap another unit to swap, or Sell.',
    synergy: 'Synergy', frontHint: 'The front unit fights first.',
    clsName: { beast: 'Beast', mech: 'Mech', mage: 'Mage' },
    uName: { wolf: 'Dust Wolf', boar: 'Iron Boar', bear: 'Cave Bear', bolt: 'Spark Bot',
      turret: 'Auto Turret', titan: 'War Titan', imp: 'Ember Imp', sage: 'Rune Sage', archon: 'Storm Archon' },
  },
  zh: {
    title: '自走竞技场',
    subtitle: '组建小队，他们会自动战斗。',
    play: '进入竞技场', howto: '购买单位，三个相同自动合成,点两个交换位置,然后开战。',
    round: '回合', gold: '金币', lives: '生命', best: '最高',
    battle: '开战', reroll: '刷新', sell: '出售', shopTitle: '招募',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    win: '胜利', lose: '失败', draw: '平局', next: '下一回合',
    gameOver: '竞技场关闭', reachedRound: r => `你在第 ${r} 回合落败。`,
    bestRound: r => `最高回合：${r}`, again: '再战',
    teamFull: '小队已满。', needGold: '金币不足。', merged: '合成成功！',
    selectSwap: '点另一个单位交换位置，或出售。',
    synergy: '羁绊', frontHint: '最前方的单位最先战斗。',
    clsName: { beast: '野兽', mech: '机械', mage: '法师' },
    uName: { wolf: '尘狼', boar: '铁猪', bear: '洞熊', bolt: '火花机器人',
      turret: '自动炮塔', titan: '战争泰坦', imp: '余烬小鬼', sage: '符文贤者', archon: '风暴执政官' },
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function tCls(id) { return TEXT[currentLang].clsName[id] || id; }
function tUnit(id) { return TEXT[currentLang].uName[id] || id; }

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
