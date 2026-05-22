window.PIXEL_GAME_DATA = {
  "slug": "pixel-soccer-manager",
  "title": "Pixel Soccer Manager",
  "zhTitle": "像素足球经理",
  "tagline": "Scout talent, drill tactics, and climb the table.",
  "zhTagline": "球探、训练、战术安排，冲上积分榜。",
  "port": 4197,
  "upgradeCurrency": "cash",
  "theme": {
    "bg": "#10251a",
    "panel": "#1b3a28",
    "accent": "#f8f32b",
    "accent2": "#52b788",
    "danger": "#e63946",
    "water": "#40916c",
    "soil": "#4f772d"
  },
  "resources": [
    [
      "cash",
      "Cash",
      "资金",
      130
    ],
    [
      "fitness",
      "Fitness",
      "体能",
      18
    ],
    [
      "morale",
      "Morale",
      "士气",
      12
    ],
    [
      "tactics",
      "Tactics",
      "战术",
      4
    ],
    [
      "fans",
      "Fans",
      "球迷",
      20
    ],
    [
      "energy",
      "Staff",
      "教练组",
      18
    ]
  ],
  "stats": [
    [
      "points",
      "Points",
      "积分",
      0
    ],
    [
      "week",
      "Week",
      "周次",
      1
    ]
  ],
  "facilities": [
    [
      "pitch",
      "Training Pitch",
      "训练场",
      "field",
      1
    ],
    [
      "gym",
      "Fitness Gym",
      "体能房",
      "home",
      0
    ],
    [
      "office",
      "Club Office",
      "俱乐部办公室",
      "shop",
      0
    ],
    [
      "scout",
      "Scout Desk",
      "球探室",
      "tower",
      0
    ],
    [
      "media",
      "Media Room",
      "媒体室",
      "chip",
      0
    ],
    [
      "stadium",
      "Stadium",
      "主场",
      "target",
      0
    ]
  ],
  "actions": [
    {
      "id": "drill",
      "name": "Tactical Drill",
      "zh": "战术演练",
      "kind": "boost",
      "turns": 1,
      "cost": {
        "fitness": 3,
        "energy": 2
      },
      "gain": {
        "tactics": 5,
        "morale": 2
      },
      "facility": "pitch",
      "xp": 12
    },
    {
      "id": "recover",
      "name": "Recovery Session",
      "zh": "恢复训练",
      "kind": "support",
      "turns": 1,
      "cost": {
        "cash": 18
      },
      "gain": {
        "fitness": 8,
        "morale": 2
      },
      "facility": "gym",
      "xp": 8
    },
    {
      "id": "scout",
      "name": "Scout Prospect",
      "zh": "球探新星",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "cash": 38,
        "energy": 2
      },
      "gain": {
        "tactics": 3,
        "fans": 6
      },
      "facility": "scout",
      "xp": 16
    },
    {
      "id": "media",
      "name": "Press Day",
      "zh": "媒体开放日",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 1
      },
      "gain": {
        "fans": 12,
        "cash": 24
      },
      "facility": "media",
      "xp": 9
    },
    {
      "id": "match",
      "name": "League Match",
      "zh": "联赛比赛",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "fitness": 8,
        "tactics": 5,
        "morale": 4
      },
      "gain": {
        "points": 6,
        "cash": 88,
        "fans": 8
      },
      "facility": "stadium",
      "xp": 24
    },
    {
      "id": "transfer",
      "name": "Transfer Deal",
      "zh": "转会运作",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "cash": 75,
        "fans": 10
      },
      "gain": {
        "tactics": 8,
        "morale": 4
      },
      "facility": "office",
      "xp": 20
    }
  ],
  "upgrades": [
    [
      "pitch",
      "Set-Piece Wall",
      "定位球墙",
      90
    ],
    [
      "gym",
      "Recovery Pool",
      "恢复池",
      100
    ],
    [
      "office",
      "Analytics Desk",
      "数据分析桌",
      135
    ],
    [
      "scout",
      "Regional Network",
      "区域球探网",
      150
    ],
    [
      "media",
      "Fan Channel",
      "球迷频道",
      115
    ],
    [
      "stadium",
      "Singing Stand",
      "歌声看台",
      170
    ]
  ],
  "events": [
    [
      "Derby Week",
      "德比周",
      "match",
      "gain",
      "points",
      4
    ],
    [
      "Injury Scare",
      "伤病警报",
      "recover",
      "lose",
      "fitness",
      4
    ],
    [
      "Wonderkid Tip",
      "新星线报",
      "scout",
      "gain",
      "tactics",
      4
    ],
    [
      "Viral Chant",
      "助威歌走红",
      "media",
      "gain",
      "fans",
      10
    ]
  ],
  "goal": {
    "stat": "points",
    "value": 66,
    "label": "Points",
    "zh": "积分"
  }
};
