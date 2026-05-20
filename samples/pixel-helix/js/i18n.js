// English / 中文 strings for Pixel Helix.

const STRINGS = {
  en: {
    title: 'PIXEL HELIX',
    subtitle: 'Rotate the disc · drop through gaps · descend the tower',
    start: 'Start',
    levelSelect: 'Choose tower',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Tower descended!',
    lose: 'Caught on the spikes',
    next: 'Next tower',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    depth: 'Depth',
    rulesTxt1: 'Drag left or right (or arrow keys) to spin the disc.',
    rulesTxt2: 'A pale GAP under the ball drops you to the next disc — bank the combo.',
    rulesTxt3: 'A red SPIKE under the ball is fatal. Plan two discs ahead.',
  },
  zh: {
    title: '像素螺旋塔',
    subtitle: '旋转圆盘 · 穿过缝隙 · 一路下沉',
    start: '开始',
    levelSelect: '选择高塔',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '通天塔登顶！',
    lose: '被尖刺扎中',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    depth: '深度',
    rulesTxt1: '左右拖动（或方向键）旋转圆盘。',
    rulesTxt2: '让缝隙转到球下方即可掉到下一层——保持连击。',
    rulesTxt3: '红色尖刺正对球时致命。提前两层规划。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-helix:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-helix:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
