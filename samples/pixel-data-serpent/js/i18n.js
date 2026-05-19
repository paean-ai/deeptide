// Pixel Data Serpent - localization (English / 中文)
const LANG_KEY = 'pixel-data-serpent-lang';
let currentLang = localStorage.getItem(LANG_KEY) || 'en';

const TEXT = {
  en: {
    title: 'DATA SERPENT',
    subtitle: 'Devour the data nodes. Don’t bite your own tail.',
    play: 'JACK IN', howto: 'Arrows / WASD or swipe to turn. Walls and firewalls are fatal.',
    score: 'Score', sector: 'Sector', best: 'Best', length: 'Len',
    paused: 'PAUSED', resume: 'Resume', restart: 'Restart', menu: 'Menu',
    gameOver: 'CONNECTION LOST', finalScore: s => `Score: ${s}`,
    reachedSector: s => `Reached sector ${s}`, bestScore: s => `Best: ${s}`,
    again: 'RECONNECT', sectorUp: s => `SECTOR ${s}`,
    golden: 'BONUS!', shrink: 'TRIMMED', slow: 'SLOW-MO',
  },
  zh: {
    title: '数据之蛇',
    subtitle: '吞噬数据节点，别咬到自己的尾巴。',
    play: '接入', howto: '方向键 / WASD 或滑动转向。墙壁与防火墙会致命。',
    score: '分数', sector: '分区', best: '最高', length: '长度',
    paused: '已暂停', resume: '继续', restart: '重新开始', menu: '菜单',
    gameOver: '连接中断', finalScore: s => `分数：${s}`,
    reachedSector: s => `到达第 ${s} 区`, bestScore: s => `最高：${s}`,
    again: '重新连接', sectorUp: s => `第 ${s} 区`,
    golden: '奖励！', shrink: '缩短', slow: '慢动作',
  },
};

function t(key, ...args) {
  const v = TEXT[currentLang][key] ?? TEXT.en[key] ?? key;
  return typeof v === 'function' ? v(...args) : v;
}
function applyStaticText() {
  document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
}
function setupLanguageToggle(onChange) {
  const btn = document.getElementById('lang-toggle');
  if (btn) {
    btn.onclick = () => {
      currentLang = currentLang === 'en' ? 'zh' : 'en';
      localStorage.setItem(LANG_KEY, currentLang);
      btn.textContent = currentLang === 'en' ? '中文' : 'EN';
      applyStaticText();
      if (onChange) onChange();
    };
    btn.textContent = currentLang === 'en' ? '中文' : 'EN';
  }
  applyStaticText();
}
