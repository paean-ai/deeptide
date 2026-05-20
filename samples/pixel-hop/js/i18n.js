// English / 中文 strings for Pixel Hop. Toggle persists in localStorage.

const STRINGS = {
  en: {
    title: 'PIXEL HOP',
    subtitle: 'Drag to steer · auto-jump · climb the sky',
    start: 'Start',
    levelSelect: 'Choose route',
    cleared: 'Cleared',
    locked: 'Locked',
    pause: 'Pause',
    resume: 'Resume',
    quit: 'Quit',
    backToMenu: 'Menu',
    altitude: 'Altitude',
    target: 'Target',
    gems: 'Gems',
    score: 'Score',
    you: 'You',
    win: 'Peak reached!',
    lose: 'You fell.',
    next: 'Next route',
    retry: 'Retry',
    instructions: 'Tap to launch, then drag anywhere to tilt left or right. Spring tiles fling you higher, clouds vanish after one bounce, movers slide across the gap. Reach the altitude target before you drop off the screen.',
    legend: 'Tile guide',
    legStatic: 'Plank · regular bounce',
    legMover: 'Mover · slides',
    legSpring: 'Spring · big launch',
    legCloud: 'Cloud · one-shot',
    legGem: 'Gem · +50 score',
  },
  zh: {
    title: '像素跃升',
    subtitle: '拖动控制 · 自动起跳 · 一路向天',
    start: '开始',
    levelSelect: '选择路线',
    cleared: '已通关',
    locked: '未解锁',
    pause: '暂停',
    resume: '继续',
    quit: '退出',
    backToMenu: '菜单',
    altitude: '高度',
    target: '目标',
    gems: '宝石',
    score: '分数',
    you: '你',
    win: '登顶成功！',
    lose: '跌落了。',
    next: '下一条路线',
    retry: '重试',
    instructions: '点击屏幕起跳，按住屏幕拖动控制左右倾斜。弹簧板把你弹得更高，云朵踩一次就消失，移动板会左右滑动。在掉出画面之前到达目标高度。',
    legend: '台面图例',
    legStatic: '木板 · 常规反弹',
    legMover: '滑板 · 左右移动',
    legSpring: '弹簧 · 强力弹跳',
    legCloud: '云朵 · 仅一次',
    legGem: '宝石 · +50 分',
  },
};

function loadLang() {
  try { return localStorage.getItem('pixel-hop:lang') || 'en'; }
  catch (_) { return 'en'; }
}
function saveLang(l) {
  try { localStorage.setItem('pixel-hop:lang', l); } catch (_) {}
}
function t(lang, k) { return (STRINGS[lang] || STRINGS.en)[k] || k; }
