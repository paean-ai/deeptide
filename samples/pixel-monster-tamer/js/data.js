window.PIXEL_GAME_DATA = {
  "slug": "pixel-monster-tamer",
  "title": "Pixel Monster Tamer",
  "zhTitle": "像素怪兽训练师",
  "tagline": "Hatch companions, train teams, and clear arena leagues.",
  "zhTagline": "孵化伙伴、训练队伍，挑战竞技联盟。",
  "port": 4196,
  "upgradeCurrency": "coins",
  "theme": {
    "bg": "#14213d",
    "panel": "#1f2f55",
    "accent": "#ffb703",
    "accent2": "#8ecae6",
    "danger": "#fb8500",
    "water": "#219ebc",
    "soil": "#4d908e"
  },
  "resources": [
    [
      "coins",
      "Coins",
      "金币",
      110
    ],
    [
      "food",
      "Food",
      "食物",
      20
    ],
    [
      "shards",
      "Shards",
      "碎片",
      4
    ],
    [
      "eggs",
      "Eggs",
      "蛋",
      2
    ],
    [
      "badges",
      "Badges",
      "徽章",
      0
    ],
    [
      "energy",
      "Care",
      "照料",
      18
    ]
  ],
  "stats": [
    [
      "bond",
      "Bond",
      "羁绊",
      0
    ],
    [
      "league",
      "League",
      "联盟",
      1
    ]
  ],
  "facilities": [
    [
      "nest",
      "Hatchery",
      "孵化室",
      "egg",
      1
    ],
    [
      "field",
      "Training Field",
      "训练场",
      "target",
      0
    ],
    [
      "lab",
      "Trait Lab",
      "特性研究所",
      "chip",
      0
    ],
    [
      "arena",
      "Arena",
      "竞技场",
      "tower",
      0
    ],
    [
      "kitchen",
      "Feed Kitchen",
      "饲育厨房",
      "pot",
      0
    ],
    [
      "sanctum",
      "Sanctum",
      "圣所",
      "crystal",
      0
    ]
  ],
  "actions": [
    {
      "id": "feed",
      "name": "Cook Feed",
      "zh": "制作饲料",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "coins": 18,
        "energy": 1
      },
      "gain": {
        "food": 10
      },
      "facility": "kitchen",
      "xp": 7
    },
    {
      "id": "hatch",
      "name": "Hatch Egg",
      "zh": "孵化怪兽",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "eggs": 1,
        "food": 6,
        "energy": 2
      },
      "gain": {
        "bond": 6,
        "shards": 2
      },
      "facility": "nest",
      "xp": 16
    },
    {
      "id": "train",
      "name": "Train Team",
      "zh": "队伍训练",
      "kind": "boost",
      "turns": 1,
      "cost": {
        "food": 5,
        "energy": 2
      },
      "gain": {
        "bond": 7
      },
      "facility": "field",
      "xp": 13
    },
    {
      "id": "trait",
      "name": "Refine Trait",
      "zh": "洗练特性",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "shards": 3,
        "coins": 26
      },
      "gain": {
        "bond": 5,
        "badges": 1
      },
      "facility": "lab",
      "xp": 15
    },
    {
      "id": "duel",
      "name": "Arena Duel",
      "zh": "竞技对战",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "bond": 8,
        "energy": 2
      },
      "gain": {
        "coins": 92,
        "badges": 4
      },
      "facility": "arena",
      "xp": 22
    },
    {
      "id": "ascend",
      "name": "Awaken Partner",
      "zh": "伙伴觉醒",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "shards": 6,
        "badges": 3
      },
      "gain": {
        "bond": 14
      },
      "facility": "sanctum",
      "xp": 28
    }
  ],
  "upgrades": [
    [
      "nest",
      "Warm Lamps",
      "保温灯",
      85
    ],
    [
      "kitchen",
      "Berry Mixer",
      "树果搅拌机",
      80
    ],
    [
      "field",
      "Obstacle Ring",
      "障碍训练环",
      120
    ],
    [
      "lab",
      "Gene Lens",
      "基因透镜",
      140
    ],
    [
      "arena",
      "League Banner",
      "联盟旗帜",
      155
    ],
    [
      "sanctum",
      "Moon Altar",
      "月光祭坛",
      180
    ]
  ],
  "events": [
    [
      "Rare Egg",
      "稀有蛋",
      "hatch",
      "gain",
      "eggs",
      1
    ],
    [
      "Team Rival",
      "劲敌挑战",
      "duel",
      "gain",
      "badges",
      3
    ],
    [
      "Snack Spoiled",
      "食物变质",
      "feed",
      "lose",
      "food",
      5
    ],
    [
      "Spirit Surge",
      "灵力涌动",
      "ascend",
      "gain",
      "bond",
      8
    ]
  ],
  "goal": {
    "stat": "badges",
    "value": 45,
    "label": "Badges",
    "zh": "徽章"
  }
};
