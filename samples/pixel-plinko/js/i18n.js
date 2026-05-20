// English / 中文 strings for Pixel Plinko.

const STRINGS = {
  en: {
    title: 'PIXEL PLINKO',
    subtitle: 'Tap above the board to drop · score the slot it lands in',
    start: 'Start',
    levelSelect: 'Choose board',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Target hit!',
    lose: 'Target missed.',
    next: 'Next board',
    retry: 'Retry',
    score: 'Score',
    target: 'Target',
    balls: 'Balls',
    drop: 'TAP TO DROP',
    rulesTxt1: 'Tap above the board — the ball drops from your tap-x.',
    rulesTxt2: 'Pegs deflect the ball; bigger slot value = bigger score.',
    rulesTxt3: '10 balls per round. Beat the target to clear the board.',
  },
  zh: {
    title: '像素小钢珠',
    subtitle: '在板上方点击落球 · 命中槽位记分',
    start: '开始',
    levelSelect: '选择板面',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '达标！',
    lose: '未达标。',
    next: '下一关',
    retry: '再来',
    score: '分数',
    target: '目标',
    balls: '剩球',
    drop: '点击落球',
    rulesTxt1: '在板上方点击 — 球从点击的横坐标落下。',
    rulesTxt2: '钉子弹开小球；槽位数字大得分高。',
    rulesTxt3: '每局 10 球，达到目标即过关。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-plinko:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-plinko:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
