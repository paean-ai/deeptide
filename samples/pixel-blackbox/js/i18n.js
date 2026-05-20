// English / 中文 strings for Pixel Black Box.

const STRINGS = {
  en: {
    title: 'PIXEL BLACK BOX',
    subtitle: 'Fire probes from the edges · deduce where the hidden atoms sit',
    start: 'Start',
    levelSelect: 'Choose box',
    cleared: 'Cleared',
    locked: 'Locked',
    backToMenu: 'Menu',
    reveal: 'Reveal',
    revealQ: 'Reveal?',
    yes: 'Yes',
    no: 'No',
    next: 'Next box',
    retry: 'Retry',
    score: 'Score',
    high: 'Best',
    atoms: 'Atoms',
    marks: 'Marks',
    probes: 'Probes',
    correct: 'Correct',
    wrong: 'Wrong',
    perfect: 'Perfect — all atoms found!',
    summary: 'Marks scored',
    rulesTxt1: 'Tap a number on the edge to fire a probe straight inward.',
    rulesTxt2: 'H = hit; an atom on the edge cell side reflects; letters pair entry/exit.',
    rulesTxt3: 'Tap a cell to mark a suspected atom. Reveal to score.',
  },
  zh: {
    title: '像素黑盒',
    subtitle: '从边缘发射探针 · 推理原子隐藏的位置',
    start: '开始',
    levelSelect: '选择黑盒',
    cleared: '已通关',
    locked: '未解锁',
    backToMenu: '菜单',
    reveal: '揭晓',
    revealQ: '现在揭晓？',
    yes: '确定',
    no: '取消',
    next: '下一关',
    retry: '再来',
    score: '分数',
    high: '最佳',
    atoms: '原子',
    marks: '标记',
    probes: '探针',
    correct: '正确',
    wrong: '错误',
    perfect: '完美——全部命中！',
    summary: '标记得分',
    rulesTxt1: '点击边缘数字向内发射探针。',
    rulesTxt2: 'H = 命中；侧旁有原子则反射；同一字母配对入口与出口。',
    rulesTxt3: '点击格子标记可疑原子。点揭晓查看得分。',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-blackbox:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-blackbox:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
