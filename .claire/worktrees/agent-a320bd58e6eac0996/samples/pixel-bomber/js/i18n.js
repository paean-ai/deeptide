// Pixel Bomber - English / Chinese strings.

const LANG_KEY = 'pixel-bomber-lang';
let currentLang = 'en';
try {
  const v = localStorage.getItem(LANG_KEY);
  if (v === 'en' || v === 'zh') currentLang = v;
} catch (_) {}

const TEXT = {
  en: {
    title: 'PIXEL BOMBER',
    subtitle: 'Plant bombs · blast walls · clear enemies',
    howto: 'Move with D-pad or WASD/arrows. Press B or Space to plant a bomb — it explodes after 2.5s in 4 directions. Destroy soft walls to find power-ups and uncover the exit. Clear all enemies then reach the exit!',
    floor: 'Floor', lives: 'Lives', score: 'Score',
    gameOver: 'GAME OVER', cleared: 'CLEARED!',
    best: 'Best', play: 'PLAY', menu: 'MENU',
    newBest: 'NEW BEST!', retry: 'RETRY', next: 'NEXT FLOOR',
    langBtn: '中文',
  },
  zh: {
    title: '炸弹人',
    subtitle: '放炸弹 · 炸墙 · 消灭敌人',
    howto: '用方向键移动，按 B 放炸弹——2.5秒后在四个方向爆炸。炸软墙找道具和出口。消灭所有敌人后走到出口！',
    floor: '关卡', lives: '生命', score: '得分',
    gameOver: '游戏结束', cleared: '通关！',
    best: '最佳', play: '开始', menu: '菜单',
    newBest: '新纪录！', retry: '再来', next: '下一关',
    langBtn: 'English',
  },
};

function t(k) { return (TEXT[currentLang] || TEXT.en)[k] || k; }
function setLang(l) {
  currentLang = l;
  try { localStorage.setItem(LANG_KEY, l); } catch (_) {}
}
