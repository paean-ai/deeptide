// Pixel Quest - English / Chinese strings.

const I18N = {
  en: {
    title: 'PIXEL QUEST',
    tagline: 'Three heroes, turn by turn, into the dark',
    play: 'PLAY',
    pick: 'SELECT A BATTLE',
    level: 'STAGE',
    menu: 'MENU', restart: 'RETREAT',
    attack: 'ATTACK', skill: 'SKILL', defend: 'DEFEND',
    pickEnemy: 'CHOOSE A TARGET',
    pickAlly: 'CHOOSE AN ALLY',
    noMp: 'NOT ENOUGH MP',
    win: 'VICTORY!', winLine: 'The party stands triumphant.',
    lose: 'DEFEATED', loseLine: 'The party has fallen.',
    next: 'NEXT', retry: 'RETRY',
    turnOf: n => n + '’s turn',
    howto: 'Heroes and foes act in speed order. On a hero’s turn pick Attack, their Skill (spends MP), or Defend to halve the next hit. Win every battle to clear the quest.',
  },
  zh: {
    title: '像素远征',
    tagline: '三位勇者，回合推进，深入黑暗',
    play: '开始',
    pick: '选择战斗',
    level: '关卡',
    menu: '菜单', restart: '撤退',
    attack: '攻击', skill: '技能', defend: '防御',
    pickEnemy: '选择目标',
    pickAlly: '选择队友',
    noMp: '魔法不足',
    win: '胜利！', winLine: '队伍凯旋而归。',
    lose: '战败', loseLine: '队伍全灭了。',
    next: '下一关', retry: '重试',
    turnOf: n => n + ' 的回合',
    howto: '勇者与敌人按速度顺序行动。轮到勇者时选择攻击、技能（消耗魔法）或防御（减半下次受击）。赢下每场战斗即可通关。',
  },
};

let lang = 'en';
try {
  const s = localStorage.getItem('pixel-quest-lang');
  if (s === 'en' || s === 'zh') lang = s;
} catch (e) { /* ignore */ }

function t(key, ...args) {
  const v = I18N[lang][key];
  return typeof v === 'function' ? v(...args) : v;
}

function setupLanguageToggle(onChange) {
  const btn = document.getElementById('btn-lang');
  if (!btn) return;
  const sync = () => { btn.textContent = lang === 'en' ? '中文' : 'EN'; };
  sync();
  btn.addEventListener('click', () => {
    lang = lang === 'en' ? 'zh' : 'en';
    try { localStorage.setItem('pixel-quest-lang', lang); } catch (e) { /* ignore */ }
    sync();
    if (onChange) onChange();
  });
}
