window.PIXEL_GAME_DATA = {
  "slug": "pixel-farmstead",
  "title": "Pixel Farmstead",
  "zhTitle": "像素农场",
  "tagline": "Plant, craft, trade, and ride the seasons.",
  "zhTagline": "播种、加工、交易，在季节中扩张农场。",
  "port": 4191,
  "upgradeCurrency": "coins",
  "theme": {
    "bg": "#17281d",
    "panel": "#213629",
    "accent": "#f3c969",
    "accent2": "#79d279",
    "danger": "#e76f51",
    "water": "#4fa3d1",
    "soil": "#6b4a2f"
  },
  "resources": [
    [
      "coins",
      "Coins",
      "金币",
      120
    ],
    [
      "seeds",
      "Seeds",
      "种子",
      18
    ],
    [
      "crops",
      "Crops",
      "作物",
      0
    ],
    [
      "milk",
      "Milk",
      "牛奶",
      0
    ],
    [
      "jam",
      "Jam",
      "果酱",
      0
    ],
    [
      "energy",
      "Energy",
      "体力",
      16
    ]
  ],
  "stats": [
    [
      "rep",
      "Market Rep",
      "集市声望",
      0
    ],
    [
      "season",
      "Season",
      "季节",
      1
    ]
  ],
  "facilities": [
    [
      "field",
      "Fields",
      "田地",
      "soil",
      1
    ],
    [
      "orchard",
      "Orchard",
      "果园",
      "tree",
      0
    ],
    [
      "barn",
      "Barn",
      "畜棚",
      "home",
      0
    ],
    [
      "mill",
      "Mill",
      "磨坊",
      "gear",
      0
    ],
    [
      "stall",
      "Market Stall",
      "集市摊位",
      "shop",
      0
    ],
    [
      "well",
      "Well",
      "水井",
      "water",
      0
    ]
  ],
  "actions": [
    {
      "id": "plant",
      "name": "Plant Rows",
      "zh": "播种田垄",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "seeds": 4,
        "energy": 2
      },
      "gain": {
        "crops": 8,
        "rep": 1
      },
      "facility": "field",
      "xp": 9
    },
    {
      "id": "water",
      "name": "Irrigate",
      "zh": "灌溉保湿",
      "kind": "boost",
      "turns": 1,
      "cost": {
        "energy": 1
      },
      "gain": {
        "crops": 4,
        "seeds": 1
      },
      "facility": "well",
      "xp": 6
    },
    {
      "id": "orchard",
      "name": "Harvest Fruit",
      "zh": "采收果园",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "energy": 3
      },
      "gain": {
        "crops": 14
      },
      "facility": "orchard",
      "xp": 12
    },
    {
      "id": "dairy",
      "name": "Tend Herd",
      "zh": "照料奶牛",
      "kind": "producer",
      "turns": 2,
      "cost": {
        "crops": 5,
        "energy": 2
      },
      "gain": {
        "milk": 7,
        "rep": 1
      },
      "facility": "barn",
      "xp": 11
    },
    {
      "id": "preserve",
      "name": "Cook Preserves",
      "zh": "熬制果酱",
      "kind": "craft",
      "turns": 2,
      "cost": {
        "crops": 10,
        "milk": 2,
        "energy": 2
      },
      "gain": {
        "jam": 6,
        "rep": 2
      },
      "facility": "mill",
      "xp": 18
    },
    {
      "id": "market",
      "name": "Market Day",
      "zh": "赶集售卖",
      "kind": "sell",
      "turns": 1,
      "cost": {
        "jam": 4,
        "crops": 8
      },
      "gain": {
        "coins": 86,
        "rep": 4,
        "seeds": 3
      },
      "facility": "stall",
      "xp": 20
    }
  ],
  "upgrades": [
    [
      "field",
      "Terraced Fields",
      "梯田",
      90
    ],
    [
      "well",
      "Stone Well",
      "石井",
      70
    ],
    [
      "orchard",
      "Bee Orchard",
      "蜂箱果园",
      130
    ],
    [
      "barn",
      "Warm Barn",
      "保温畜棚",
      120
    ],
    [
      "mill",
      "Copper Kettle",
      "铜锅",
      150
    ],
    [
      "stall",
      "Painted Sign",
      "彩绘招牌",
      170
    ]
  ],
  "events": [
    [
      "Rain Front",
      "雨云来临",
      "water",
      "gain",
      "crops",
      10
    ],
    [
      "Seed Merchant",
      "种子商到访",
      "market",
      "gain",
      "seeds",
      8
    ],
    [
      "Fox Tracks",
      "狐狸脚印",
      "barn",
      "lose",
      "milk",
      3
    ],
    [
      "Harvest Fair",
      "丰收集会",
      "market",
      "gain",
      "rep",
      5
    ]
  ],
  "goal": {
    "stat": "rep",
    "value": 72,
    "label": "Market Rep",
    "zh": "集市声望"
  }
};
