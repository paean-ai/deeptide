// English / 中文 strings for Pixel Knight.

const STRINGS = {
  en: {
    title: 'PIXEL KNIGHT',
    subtitle: 'Knight\'s Tour · visit every square exactly once',
    start: 'Start',
    levelSelect: 'Choose board',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Tour complete!',
    lose: 'Stuck — try again',
    next: 'Next board',
    retry: 'Retry',
    undo: 'Undo',
    restart: 'Restart',
    hint: 'Hint',
    visited: 'Visited',
    rulesTxt1: 'Tap any of the eight L-shaped knight-target cells to hop.',
    rulesTxt2: 'A complete tour visits every square exactly once.',
    rulesTxt3: 'The pink Warnsdorff hint marks the move with the fewest exits.',
  },
  zh: {
    title: '像素骑士',
    subtitle: '骑士周游 · 每格只走一次',
    start: '开始',
    levelSelect: '选择棋盘',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '周游完成！',
    lose: '走到死胡同',
    next: '下一关',
    retry: '再来',
    undo: '撤销',
    restart: '重开',
    hint: '提示',
    visited: '已访',
    rulesTxt1: '点击 8 个 L 形目标之一进行跳跃。',
    rulesTxt2: '完美周游要走遍每一格恰好一次。',
    rulesTxt3: '粉色提示标记“出路最少”的最佳一步（Warnsdorff 法则）。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-knight:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-knight:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
