window.PIXEL_GAME_DATA = {
  "slug": "pixel-colony-builder",
  "title": "Pixel Colony Builder",
  "zhTitle": "像素殖民地",
  "tagline": "Gather, shelter, research, and survive hostile seasons.",
  "zhTagline": "采集、建造、研究，熬过恶劣季节。",
  "port": 4200,
  "upgradeCurrency": "wood",
  "theme": {
    "bg": "#151d1f",
    "panel": "#223033",
    "accent": "#e9c46a",
    "accent2": "#90be6d",
    "danger": "#f94144",
    "water": "#577590",
    "soil": "#7f5539"
  },
  "resources": [
    [
      "food",
      "Food",
      "食物",
      24
    ],
    [
      "wood",
      "Wood",
      "木材",
      18
    ],
    [
      "stone",
      "Stone",
      "石料",
      8
    ],
    [
      "tools",
      "Tools",
      "工具",
      2
    ],
    [
      "colonists",
      "Colonists",
      "居民",
      5
    ],
    [
      "energy",
      "Labor",
      "劳力",
      18
    ]
  ],
  "stats": [
    [
      "stability",
      "Stability",
      "稳定度",
      0
    ],
    [
      "cycle",
      "Cycle",
      "周期",
      1
    ]
  ],
  "facilities": [
    [
      "camp",
      "Base Camp",
      "营地",
      "home",
      1
    ],
    [
      "forest",
      "Lumber Grove",
      "伐木林",
      "tree",
      0
    ],
    [
      "quarry",
      "Quarry",
      "采石场",
      "rock",
      0
    ],
    [
      "farm",
      "Hydro Farm",
      "水培农场",
      "field",
      0
    ],
    [
      "lab",
      "Research Lab",
      "研究所",
      "chip",
      0
    ],
    [
      "wall",
      "Storm Wall",
      "风暴墙",
      "shield",
      0
    ]
  ],
  "actions": [
    {
      "id": "forage",
      "name": "Forage Supplies",
      "zh": "野外采集",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "food": 9,
        "wood": 4
      },
      "facility": "camp",
      "xp": 8
    },
    {
      "id": "lumber",
      "name": "Cut Timber",
      "zh": "砍伐木材",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "wood": 10
      },
      "facility": "forest",
      "xp": 9
    },
    {
      "id": "mine",
      "name": "Mine Stone",
      "zh": "开采石料",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "tools": 1,
        "energy": 2
      },
      "gain": {
        "stone": 9
      },
      "facility": "quarry",
      "xp": 13
    },
    {
      "id": "farm",
      "name": "Hydro Harvest",
      "zh": "水培收获",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "wood": 4,
        "energy": 2
      },
      "gain": {
        "food": 16,
        "stability": 2
      },
      "facility": "farm",
      "xp": 15
    },
    {
      "id": "research",
      "name": "Research Tech",
      "zh": "科技研究",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "stone": 4,
        "tools": 1
      },
      "gain": {
        "tools": 3,
        "stability": 5
      },
      "facility": "lab",
      "xp": 22
    },
    {
      "id": "defend",
      "name": "Weather Storm",
      "zh": "抵御风暴",
      "kind": "support",
      "turns": 2,
      "cost": {
        "wood": 8,
        "stone": 5,
        "food": 6
      },
      "gain": {
        "stability": 12,
        "colonists": 1
      },
      "facility": "wall",
      "xp": 28
    }
  ],
  "upgrades": [
    [
      "camp",
      "Heated Bunks",
      "保温床铺",
      80
    ],
    [
      "forest",
      "Saw Team",
      "锯木队",
      95
    ],
    [
      "quarry",
      "Pulley Crane",
      "滑轮吊机",
      115
    ],
    [
      "farm",
      "Nutrient Tanks",
      "营养罐",
      130
    ],
    [
      "lab",
      "Survey Drone",
      "测绘无人机",
      145
    ],
    [
      "wall",
      "Layered Barrier",
      "复合屏障",
      175
    ]
  ],
  "events": [
    [
      "New Survivors",
      "幸存者加入",
      "camp",
      "gain",
      "colonists",
      2
    ],
    [
      "Blight",
      "作物病害",
      "farm",
      "lose",
      "food",
      6
    ],
    [
      "Rich Vein",
      "富矿层",
      "mine",
      "gain",
      "stone",
      7
    ],
    [
      "Solar Calm",
      "晴朗窗口",
      "defend",
      "gain",
      "stability",
      5
    ]
  ],
  "goal": {
    "stat": "stability",
    "value": 90,
    "label": "Stability",
    "zh": "稳定度"
  }
};
