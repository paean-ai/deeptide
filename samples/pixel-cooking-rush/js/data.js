window.PIXEL_GAME_DATA = {
  "slug": "pixel-cooking-rush",
  "title": "Pixel Cooking Rush",
  "zhTitle": "像素料理冲刺",
  "tagline": "Prep stations, serve combos, survive the dinner rush.",
  "zhTagline": "备菜、出餐、连击服务，顶住晚餐高峰。",
  "port": 4192,
  "upgradeCurrency": "coins",
  "theme": {
    "bg": "#201b20",
    "panel": "#33242a",
    "accent": "#ffcc66",
    "accent2": "#ff8c42",
    "danger": "#d95d39",
    "water": "#7bdff2",
    "soil": "#704632"
  },
  "resources": [
    [
      "coins",
      "Coins",
      "金币",
      90
    ],
    [
      "veg",
      "Veg",
      "蔬菜",
      18
    ],
    [
      "meat",
      "Meat",
      "肉类",
      10
    ],
    [
      "sauce",
      "Sauce",
      "酱汁",
      2
    ],
    [
      "plates",
      "Plates",
      "餐盘",
      8
    ],
    [
      "energy",
      "Focus",
      "专注",
      18
    ]
  ],
  "stats": [
    [
      "stars",
      "Stars",
      "星级",
      0
    ],
    [
      "rush",
      "Rush",
      "客流",
      1
    ]
  ],
  "facilities": [
    [
      "prep",
      "Prep Table",
      "备菜台",
      "table",
      1
    ],
    [
      "grill",
      "Grill",
      "烤炉",
      "fire",
      0
    ],
    [
      "soup",
      "Soup Pot",
      "汤锅",
      "pot",
      0
    ],
    [
      "counter",
      "Service Counter",
      "出餐台",
      "shop",
      0
    ],
    [
      "dish",
      "Dishwasher",
      "洗碗机",
      "water",
      0
    ],
    [
      "pantry",
      "Pantry",
      "储藏室",
      "crate",
      0
    ]
  ],
  "actions": [
    {
      "id": "prepveg",
      "name": "Chop Veg",
      "zh": "切配蔬菜",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 1
      },
      "gain": {
        "veg": 8
      },
      "facility": "prep",
      "xp": 6
    },
    {
      "id": "stock",
      "name": "Stock Pantry",
      "zh": "补充食材",
      "kind": "buy",
      "turns": 1,
      "cost": {
        "coins": 28
      },
      "gain": {
        "meat": 8,
        "veg": 6,
        "plates": 4
      },
      "facility": "pantry",
      "xp": 5
    },
    {
      "id": "grill",
      "name": "Grill Skewers",
      "zh": "烤制肉串",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "meat": 5,
        "sauce": 1,
        "energy": 2
      },
      "gain": {
        "plates": 2,
        "stars": 2
      },
      "facility": "grill",
      "xp": 14
    },
    {
      "id": "soup",
      "name": "Simmer Soup",
      "zh": "熬制浓汤",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "veg": 8,
        "meat": 2,
        "energy": 2
      },
      "gain": {
        "sauce": 4,
        "stars": 2
      },
      "facility": "soup",
      "xp": 12
    },
    {
      "id": "wash",
      "name": "Clear Dishes",
      "zh": "清洗餐盘",
      "kind": "support",
      "turns": 1,
      "cost": {
        "energy": 1
      },
      "gain": {
        "plates": 7
      },
      "facility": "dish",
      "xp": 7
    },
    {
      "id": "serve",
      "name": "Serve Combo",
      "zh": "套餐出餐",
      "kind": "sell",
      "turns": 1,
      "cost": {
        "plates": 4,
        "sauce": 2,
        "veg": 4
      },
      "gain": {
        "coins": 96,
        "stars": 5
      },
      "facility": "counter",
      "xp": 23
    }
  ],
  "upgrades": [
    [
      "prep",
      "Knife Rail",
      "刀具架",
      75
    ],
    [
      "pantry",
      "Cold Pantry",
      "冷藏储藏室",
      95
    ],
    [
      "grill",
      "Double Grill",
      "双层烤炉",
      125
    ],
    [
      "soup",
      "Stock Mastery",
      "高汤技艺",
      120
    ],
    [
      "dish",
      "Steam Washer",
      "蒸汽洗碗机",
      110
    ],
    [
      "counter",
      "Bell Counter",
      "铃铛出餐台",
      160
    ]
  ],
  "events": [
    [
      "Food Critic",
      "美食评论员",
      "serve",
      "gain",
      "stars",
      6
    ],
    [
      "Burnt Pan",
      "糊锅",
      "grill",
      "lose",
      "sauce",
      2
    ],
    [
      "Lunch Crowd",
      "午餐客流",
      "serve",
      "gain",
      "coins",
      45
    ],
    [
      "Supplier Delay",
      "供应延误",
      "stock",
      "lose",
      "meat",
      2
    ]
  ],
  "goal": {
    "stat": "stars",
    "value": 80,
    "label": "Stars",
    "zh": "星级"
  }
};
