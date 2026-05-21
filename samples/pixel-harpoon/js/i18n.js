// English / 中文 strings for Pixel Harpoon.

const STRINGS = {
  en: {
    title: 'PIXEL HARPOON',
    subtitle: 'Pop every bouncing orb',
    start: 'Start',
    levelSelect: 'Choose stage',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Stage clear!',
    lose: 'Game over',
    next: 'Next stage',
    retry: 'Retry',
    score: 'Score',
    best: 'Best',
    lives: 'Lives',
    rulesTxt1: 'Hold ◀ ▶ to move; tap FIRE to launch the harpoon.',
    rulesTxt2: 'A hit splits an orb in two; the smallest just pop.',
    rulesTxt3: 'Don’t let an orb touch you. Clear them all.',
  },
  zh: {
    title: '像素鱼叉',
    subtitle: '击破所有弹跳球',
    start: '开始',
    levelSelect: '选择关卡',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '过关！',
    lose: '游戏结束',
    next: '下一关',
    retry: '重玩',
    score: '分数',
    best: '最佳',
    lives: '生命',
    rulesTxt1: '按住 ◀ ▶ 移动，点 发射 射出鱼叉。',
    rulesTxt2: '命中会把球一分为二，最小的球直接破裂。',
    rulesTxt3: '别让球碰到你，清光全部即可过关。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-harpoon:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-harpoon:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
