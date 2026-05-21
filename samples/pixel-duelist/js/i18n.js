// Pixel Duelist - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL DUELIST',
    subtitle: 'Read the blade.',
    start: 'START',
    bossSelect: 'CHOOSE YOUR FOE',
    locked: 'LOCKED',
    cleared: 'CLEARED',
    back: 'MENU',
    parry: 'PARRY',
    dodge: 'DODGE',
    execute: 'EXECUTE',
    win: 'VICTORY',
    lose: 'DEFEATED',
    retry: 'RETRY',
    next: 'NEXT FOE',
    flawless: 'FLAWLESS',
    perfect: 'PERFECT',
    combo: 'COMBO',
    rules1: 'Blue slash — tap PARRY just before it lands.',
    rules2: 'Amber thrust — tap DODGE instead.',
    rules3: 'Hit the last sliver for a PERFECT. Fill the posture bar to stagger your foe, then EXECUTE.',
  },
  zh: {
    title: '像素决斗',
    subtitle: '看清刀光',
    start: '开始',
    bossSelect: '选择对手',
    locked: '未解锁',
    cleared: '已通关',
    back: '菜单',
    parry: '格挡',
    dodge: '闪避',
    execute: '处决',
    win: '胜利',
    lose: '战败',
    retry: '重试',
    next: '下一个',
    flawless: '完美无伤',
    perfect: '完美',
    combo: '连击',
    rules1: '蓝色横斩——在击中前一刻点「格挡」',
    rules2: '橙色突刺——改点「闪避」',
    rules3: '在最后一线命中即「完美」。打满架势槽使对手失衡，然后「处决」',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-duelist:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-duelist:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
