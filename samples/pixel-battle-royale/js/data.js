window.PIXEL_GAME_DATA = {
  "slug": "pixel-battle-royale",
  "title": "Pixel Battle Royale",
  "zhTitle": "像素缩圈求生",
  "tagline": "Scout, loot, rotate, and outplay the final circle.",
  "zhTagline": "侦察、搜刮、转移，在最终安全区胜出。",
  "port": 4195,
  "upgradeCurrency": "intel",
  "theme": {
    "bg": "#161a1d",
    "panel": "#24292d",
    "accent": "#e9c46a",
    "accent2": "#2a9d8f",
    "danger": "#e63946",
    "water": "#457b9d",
    "soil": "#5f4b32"
  },
  "resources": [
    [
      "hp",
      "HP",
      "生命",
      100
    ],
    [
      "ammo",
      "Ammo",
      "弹药",
      24
    ],
    [
      "armor",
      "Armor",
      "护甲",
      10
    ],
    [
      "meds",
      "Meds",
      "药品",
      3
    ],
    [
      "intel",
      "Intel",
      "情报",
      0
    ],
    [
      "energy",
      "Stamina",
      "耐力",
      18
    ]
  ],
  "stats": [
    [
      "standing",
      "Standing",
      "排名积分",
      0
    ],
    [
      "circle",
      "Circle",
      "圈数",
      1
    ]
  ],
  "facilities": [
    [
      "drop",
      "Drop Zone",
      "落点",
      "crate",
      1
    ],
    [
      "town",
      "Loot Town",
      "资源城",
      "home",
      0
    ],
    [
      "ridge",
      "Ridge",
      "山脊",
      "tower",
      0
    ],
    [
      "safe",
      "Safe Zone",
      "安全区",
      "shield",
      0
    ],
    [
      "craft",
      "Craft Bench",
      "改装台",
      "gear",
      0
    ],
    [
      "final",
      "Final Ring",
      "决赛圈",
      "target",
      0
    ]
  ],
  "actions": [
    {
      "id": "loot",
      "name": "Loot Buildings",
      "zh": "搜刮建筑",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "ammo": 10,
        "armor": 3,
        "meds": 1
      },
      "facility": "town",
      "xp": 10
    },
    {
      "id": "scout",
      "name": "Scout Circle",
      "zh": "侦察圈型",
      "kind": "boost",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "intel": 5,
        "standing": 2
      },
      "facility": "ridge",
      "xp": 12
    },
    {
      "id": "rotate",
      "name": "Smart Rotate",
      "zh": "聪明转移",
      "kind": "support",
      "turns": 1,
      "cost": {
        "intel": 3,
        "energy": 2
      },
      "gain": {
        "standing": 6,
        "circle": 1
      },
      "facility": "safe",
      "xp": 16
    },
    {
      "id": "craft",
      "name": "Tune Loadout",
      "zh": "改装配装",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "ammo": 6,
        "armor": 2
      },
      "gain": {
        "standing": 4,
        "ammo": 4
      },
      "facility": "craft",
      "xp": 13
    },
    {
      "id": "med",
      "name": "Reset Fight",
      "zh": "治疗重整",
      "kind": "support",
      "turns": 1,
      "cost": {
        "meds": 1,
        "energy": 1
      },
      "gain": {
        "hp": 14,
        "armor": 2
      },
      "facility": "safe",
      "xp": 8
    },
    {
      "id": "clutch",
      "name": "Final Duel",
      "zh": "决赛对枪",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "ammo": 14,
        "armor": 6,
        "intel": 4
      },
      "gain": {
        "standing": 18
      },
      "facility": "final",
      "xp": 28
    }
  ],
  "upgrades": [
    [
      "drop",
      "Hot Drop Read",
      "落点预判",
      85
    ],
    [
      "town",
      "Route Map",
      "搜刮路线图",
      95
    ],
    [
      "ridge",
      "Spotter Scope",
      "侦察镜",
      120
    ],
    [
      "safe",
      "Smoke Kit",
      "烟雾套装",
      130
    ],
    [
      "craft",
      "Grip Bench",
      "握把工坊",
      110
    ],
    [
      "final",
      "Zone Coach",
      "圈边教练",
      160
    ]
  ],
  "events": [
    [
      "Care Package",
      "空投补给",
      "loot",
      "gain",
      "ammo",
      12
    ],
    [
      "Third Party",
      "被劝架",
      "clutch",
      "lose",
      "hp",
      12
    ],
    [
      "Free Rotate",
      "天命圈",
      "rotate",
      "gain",
      "standing",
      6
    ],
    [
      "Armor Crack",
      "护甲破裂",
      "craft",
      "lose",
      "armor",
      3
    ]
  ],
  "goal": {
    "stat": "standing",
    "value": 100,
    "label": "Standing",
    "zh": "排名积分"
  }
};
