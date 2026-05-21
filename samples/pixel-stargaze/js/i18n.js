// Pixel Stargaze - English / Chinese strings.

const STRINGS = {
  en: {
    title: 'PIXEL STARGAZE',
    subtitle: 'Gather the light of the night sky.',
    start: 'START',
    light: 'LIGHT',
    tapHint: 'tap the sky',
    scopesTab: 'TELESCOPES',
    researchTab: 'RESEARCH',
    buy: 'BUY',
    done: 'DONE',
    publish: 'PUBLISH',
    renown: 'RENOWN',
    awayTitle: 'WHILE YOU WERE AWAY',
    collect: 'COLLECT',
    rules1: 'Tap the night sky to gather Light by hand.',
    rules2: 'Buy telescopes — they gather Light on their own, even while away.',
    rules3: 'Publish a paper to reset the run for Renown, a permanent boost.',
  },
  zh: {
    title: '像素观星',
    subtitle: '采集夜空之光',
    start: '开始',
    light: '光',
    tapHint: '点击夜空',
    scopesTab: '望远镜',
    researchTab: '研究',
    buy: '购买',
    done: '已研究',
    publish: '发表',
    renown: '声望',
    awayTitle: '离线收益',
    collect: '收取',
    rules1: '点击夜空，亲手采集光',
    rules2: '购买望远镜，它们会自动采集光，离线也不停',
    rules3: '发表论文可重置进度并获得声望——永久增益',
  },
};

let lang = 'en';
function loadLang() {
  try {
    const v = localStorage.getItem('pixel-stargaze:lang');
    if (v === 'en' || v === 'zh') lang = v;
  } catch (e) { /* storage unavailable */ }
}
function saveLang() {
  try { localStorage.setItem('pixel-stargaze:lang', lang); } catch (e) { /* ignore */ }
}
function t(key) { return STRINGS[lang][key]; }
function L(pair) { return pair[lang === 'zh' ? 1 : 0]; }
