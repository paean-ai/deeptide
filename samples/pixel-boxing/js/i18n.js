// English / 中文 strings for Pixel Boxing.

const STRINGS = {
  en: {
    title: 'PIXEL BOXING',
    subtitle: 'Read the tell · dodge · counter-punch for the KO',
    start: 'Start',
    levelSelect: 'Choose bout',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    win: 'Knockout!',
    lose: 'You went down',
    next: 'Next bout',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    dodgeL: 'DODGE ◀',
    dodgeR: 'DODGE ▶',
    block: 'BLOCK',
    punch: 'PUNCH',
    rulesTxt1: 'When the foe winds up, a glowing arm shows which side it strikes.',
    rulesTxt2: 'Dodge the OPPOSITE way to make it whiff — that staggers the foe.',
    rulesTxt3: 'Punch during the stagger window for a triple-damage counter.',
  },
  zh: {
    title: '像素拳击',
    subtitle: '看清预兆 · 闪避 · 反击一击 KO',
    start: '开始',
    levelSelect: '选择对手',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    win: '击倒对手！',
    lose: '你被击倒了',
    next: '下一场',
    retry: '再来',
    score: '分数',
    high: '最佳',
    dodgeL: '左闪 ◀',
    dodgeR: '右闪 ▶',
    block: '格挡',
    punch: '出拳',
    rulesTxt1: '对手蓄力时，发亮的手臂会显示出拳方向。',
    rulesTxt2: '朝相反方向闪避使其落空，对手会踉跄。',
    rulesTxt3: '在踉跄窗口出拳，打出三倍伤害的反击。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-boxing:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-boxing:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
