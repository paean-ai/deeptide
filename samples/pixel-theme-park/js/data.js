window.PIXEL_GAME_DATA = {
  "slug": "pixel-theme-park",
  "title": "Pixel Theme Park",
  "zhTitle": "像素主题乐园",
  "tagline": "Balance rides, queues, snacks, and guest happiness.",
  "zhTagline": "平衡游乐设施、队列、小吃与游客快乐。",
  "port": 4194,
  "upgradeCurrency": "coins",
  "theme": {
    "bg": "#151826",
    "panel": "#24213b",
    "accent": "#f7aef8",
    "accent2": "#b8f2e6",
    "danger": "#ff6b6b",
    "water": "#80ed99",
    "soil": "#5c3d5e"
  },
  "resources": [
    [
      "coins",
      "Coins",
      "金币",
      160
    ],
    [
      "guests",
      "Guests",
      "游客",
      18
    ],
    [
      "snacks",
      "Snacks",
      "小吃",
      10
    ],
    [
      "parts",
      "Parts",
      "零件",
      6
    ],
    [
      "tickets",
      "Tickets",
      "门票",
      0
    ],
    [
      "energy",
      "Staff",
      "员工",
      16
    ]
  ],
  "stats": [
    [
      "joy",
      "Joy",
      "欢乐值",
      0
    ],
    [
      "rank",
      "Rank",
      "评级",
      1
    ]
  ],
  "facilities": [
    [
      "gate",
      "Front Gate",
      "入口大门",
      "shop",
      1
    ],
    [
      "coaster",
      "Coaster",
      "过山车",
      "tower",
      0
    ],
    [
      "wheel",
      "Sky Wheel",
      "摩天轮",
      "wheel",
      0
    ],
    [
      "snack",
      "Snack Row",
      "小吃街",
      "pot",
      0
    ],
    [
      "arcade",
      "Arcade",
      "游戏厅",
      "chip",
      0
    ],
    [
      "maint",
      "Maintenance",
      "维修间",
      "gear",
      0
    ]
  ],
  "actions": [
    {
      "id": "promo",
      "name": "Ticket Promo",
      "zh": "门票促销",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "coins": 24,
        "energy": 1
      },
      "gain": {
        "guests": 12,
        "tickets": 8
      },
      "facility": "gate",
      "xp": 8
    },
    {
      "id": "snacks",
      "name": "Prep Snacks",
      "zh": "准备小吃",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "coins": 18,
        "energy": 1
      },
      "gain": {
        "snacks": 9
      },
      "facility": "snack",
      "xp": 7
    },
    {
      "id": "coaster",
      "name": "Run Coaster",
      "zh": "运营过山车",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "guests": 8,
        "parts": 2
      },
      "gain": {
        "coins": 105,
        "joy": 6,
        "tickets": 4
      },
      "facility": "coaster",
      "xp": 20
    },
    {
      "id": "wheel",
      "name": "Sky Wheel Ride",
      "zh": "摩天轮游览",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "guests": 6,
        "energy": 2
      },
      "gain": {
        "coins": 76,
        "joy": 7
      },
      "facility": "wheel",
      "xp": 17
    },
    {
      "id": "arcade",
      "name": "Arcade Tokens",
      "zh": "游戏币活动",
      "kind": "sell",
      "turns": 1,
      "cost": {
        "snacks": 4,
        "guests": 5
      },
      "gain": {
        "coins": 66,
        "joy": 4
      },
      "facility": "arcade",
      "xp": 15
    },
    {
      "id": "maint",
      "name": "Safety Sweep",
      "zh": "安全巡检",
      "kind": "support",
      "turns": 1,
      "cost": {
        "parts": 3,
        "energy": 2
      },
      "gain": {
        "joy": 5,
        "guests": 2
      },
      "facility": "maint",
      "xp": 12
    }
  ],
  "upgrades": [
    [
      "gate",
      "Neon Gate",
      "霓虹大门",
      100
    ],
    [
      "snack",
      "Popcorn Lab",
      "爆米花工坊",
      90
    ],
    [
      "coaster",
      "Loop Track",
      "回环轨道",
      170
    ],
    [
      "wheel",
      "Glass Cabins",
      "玻璃座舱",
      145
    ],
    [
      "arcade",
      "Prize Wall",
      "奖品墙",
      125
    ],
    [
      "maint",
      "Sensor Kit",
      "传感器套件",
      115
    ]
  ],
  "events": [
    [
      "School Trip",
      "学校春游",
      "promo",
      "gain",
      "guests",
      14
    ],
    [
      "Ride Photo Hit",
      "游乐照片热卖",
      "coaster",
      "gain",
      "coins",
      40
    ],
    [
      "Queue Jam",
      "排队拥堵",
      "gate",
      "lose",
      "joy",
      4
    ],
    [
      "Snack Trend",
      "小吃爆款",
      "snacks",
      "gain",
      "snacks",
      7
    ]
  ],
  "goal": {
    "stat": "joy",
    "value": 85,
    "label": "Joy",
    "zh": "欢乐值"
  }
};
