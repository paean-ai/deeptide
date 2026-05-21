// English / 中文 strings for Pixel Bucket Brigade.

const STRINGS = {
  en: {
    title: 'BUCKET BRIGADE',
    subtitle: 'Catch every bomb before the rim',
    start: 'Start',
    gameOver: 'Sky fell',
    score: 'Score',
    best: 'Best',
    wave: 'Wave',
    retry: 'Retry',
    menu: 'Menu',
    slow: 'SLOW',
    magnet: 'MAGNET',
    rulesTxt1: 'Drag (or ◀ ▶) to slide your bucket stack.',
    rulesTxt2: 'Catch bombs at the rim — a miss costs a bucket.',
    rulesTxt3: 'Grab power-ups: +bucket, slow-motion, wide magnet.',
  },
  zh: {
    title: '水桶接力',
    subtitle: '在边缘前接住每颗炸弹',
    start: '开始',
    gameOver: '天塌了',
    score: '分数',
    best: '最佳',
    wave: '波次',
    retry: '再来',
    menu: '菜单',
    slow: '减速',
    magnet: '磁吸',
    rulesTxt1: '拖动（或 ◀ ▶）左右滑动水桶。',
    rulesTxt2: '在边缘处接住炸弹——漏接会损失一个桶。',
    rulesTxt3: '拾取道具：加桶、慢动作、宽磁吸。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-bucket-brigade:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-bucket-brigade:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
