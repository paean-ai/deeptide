window.PIXEL_GAME_DATA = {
  "slug": "pixel-poker-rogue",
  "title": "Pixel Poker Rogue",
  "zhTitle": "像素扑克构筑",
  "tagline": "Build scoring engines from hands, charms, and risky blinds.",
  "zhTagline": "用牌型、护符和风险盲注构筑得分引擎。",
  "port": 4198,
  "upgradeCurrency": "chips",
  "theme": {
    "bg": "#171219",
    "panel": "#251d29",
    "accent": "#e9c46a",
    "accent2": "#b56576",
    "danger": "#e56b6f",
    "water": "#6d597a",
    "soil": "#355070"
  },
  "resources": [
    [
      "chips",
      "Chips",
      "筹码",
      90
    ],
    [
      "cards",
      "Cards",
      "牌组",
      24
    ],
    [
      "charms",
      "Charms",
      "护符",
      2
    ],
    [
      "rerolls",
      "Rerolls",
      "重掷",
      3
    ],
    [
      "hands",
      "Hands",
      "手牌",
      5
    ],
    [
      "energy",
      "Nerve",
      "胆量",
      18
    ]
  ],
  "stats": [
    [
      "score",
      "Score",
      "分数",
      0
    ],
    [
      "ante",
      "Ante",
      "底注",
      1
    ]
  ],
  "facilities": [
    [
      "table",
      "Card Table",
      "牌桌",
      "table",
      1
    ],
    [
      "shop",
      "Charm Shop",
      "护符商店",
      "shop",
      0
    ],
    [
      "printer",
      "Card Printer",
      "印牌机",
      "gear",
      0
    ],
    [
      "vault",
      "Chip Vault",
      "筹码库",
      "crate",
      0
    ],
    [
      "oracle",
      "Odds Oracle",
      "赔率机",
      "crystal",
      0
    ],
    [
      "boss",
      "Boss Blind",
      "首领盲注",
      "target",
      0
    ]
  ],
  "actions": [
    {
      "id": "draw",
      "name": "Draw Engine",
      "zh": "抽牌引擎",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "hands": 1,
        "energy": 1
      },
      "gain": {
        "cards": 7,
        "score": 4
      },
      "facility": "table",
      "xp": 9
    },
    {
      "id": "pair",
      "name": "Pair Chain",
      "zh": "对子连锁",
      "kind": "sell",
      "turns": 1,
      "cost": {
        "cards": 6
      },
      "gain": {
        "chips": 45,
        "score": 7
      },
      "facility": "table",
      "xp": 13
    },
    {
      "id": "flush",
      "name": "Flush Burst",
      "zh": "同花爆发",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "cards": 10,
        "rerolls": 1
      },
      "gain": {
        "chips": 92,
        "score": 13
      },
      "facility": "oracle",
      "xp": 22
    },
    {
      "id": "charm",
      "name": "Buy Charm",
      "zh": "购买护符",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "chips": 45
      },
      "gain": {
        "charms": 1,
        "score": 3
      },
      "facility": "shop",
      "xp": 10
    },
    {
      "id": "print",
      "name": "Duplicate Card",
      "zh": "复制关键牌",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "charms": 1,
        "chips": 30
      },
      "gain": {
        "cards": 8,
        "rerolls": 1
      },
      "facility": "printer",
      "xp": 16
    },
    {
      "id": "blind",
      "name": "Boss Blind",
      "zh": "挑战首领盲注",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "hands": 2,
        "cards": 12,
        "charms": 2
      },
      "gain": {
        "score": 22,
        "chips": 120,
        "ante": 1
      },
      "facility": "boss",
      "xp": 30
    }
  ],
  "upgrades": [
    [
      "table",
      "Felt Table",
      "绒面牌桌",
      80
    ],
    [
      "shop",
      "Rare Shelf",
      "稀有货架",
      120
    ],
    [
      "printer",
      "Foil Press",
      "闪箔压印机",
      135
    ],
    [
      "vault",
      "Chip Rebate",
      "筹码返利",
      110
    ],
    [
      "oracle",
      "Probability Lamp",
      "概率灯",
      150
    ],
    [
      "boss",
      "Blind Trophy",
      "盲注奖杯",
      175
    ]
  ],
  "events": [
    [
      "Lucky Cut",
      "幸运切牌",
      "draw",
      "gain",
      "cards",
      8
    ],
    [
      "Cold Deck",
      "冷牌局",
      "flush",
      "lose",
      "rerolls",
      1
    ],
    [
      "Coupon Charm",
      "护符优惠券",
      "charm",
      "gain",
      "chips",
      25
    ],
    [
      "All-In Crowd",
      "全押欢呼",
      "blind",
      "gain",
      "score",
      8
    ]
  ],
  "goal": {
    "stat": "score",
    "value": 120,
    "label": "Score",
    "zh": "分数"
  }
};
