window.PIXEL_GAME_DATA = {
  "slug": "pixel-rail-dispatch",
  "title": "Pixel Rail Dispatch",
  "zhTitle": "像素铁路调度",
  "tagline": "Route trains, ship cargo, and prevent platform gridlock.",
  "zhTagline": "编排列车、调运货物，避免站台堵塞。",
  "port": 4193,
  "upgradeCurrency": "coins",
  "theme": {
    "bg": "#101923",
    "panel": "#1d2b36",
    "accent": "#ffd166",
    "accent2": "#7cc7ff",
    "danger": "#ef476f",
    "water": "#4cc9f0",
    "soil": "#5d6470"
  },
  "resources": [
    [
      "coins",
      "Budget",
      "预算",
      140
    ],
    [
      "cargo",
      "Cargo",
      "货物",
      14
    ],
    [
      "mail",
      "Mail",
      "邮件",
      10
    ],
    [
      "fuel",
      "Fuel",
      "燃料",
      18
    ],
    [
      "parts",
      "Parts",
      "零件",
      6
    ],
    [
      "energy",
      "Dispatch",
      "调度力",
      16
    ]
  ],
  "stats": [
    [
      "onTime",
      "On-Time",
      "准点率",
      0
    ],
    [
      "network",
      "Network",
      "路网",
      1
    ]
  ],
  "facilities": [
    [
      "yard",
      "Switch Yard",
      "编组站",
      "track",
      1
    ],
    [
      "depot",
      "Depot",
      "机务段",
      "home",
      0
    ],
    [
      "station",
      "Station",
      "车站",
      "shop",
      0
    ],
    [
      "signal",
      "Signal Box",
      "信号楼",
      "tower",
      0
    ],
    [
      "warehouse",
      "Warehouse",
      "货仓",
      "crate",
      0
    ],
    [
      "workshop",
      "Workshop",
      "工坊",
      "gear",
      0
    ]
  ],
  "actions": [
    {
      "id": "sort",
      "name": "Sort Freight",
      "zh": "编组货列",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "cargo": 8,
        "mail": 3
      },
      "facility": "yard",
      "xp": 8
    },
    {
      "id": "fuel",
      "name": "Refuel Engines",
      "zh": "补给燃料",
      "kind": "buy",
      "turns": 1,
      "cost": {
        "coins": 30
      },
      "gain": {
        "fuel": 9,
        "parts": 2
      },
      "facility": "depot",
      "xp": 7
    },
    {
      "id": "signal",
      "name": "Clear Signals",
      "zh": "开放信号",
      "kind": "boost",
      "turns": 1,
      "cost": {
        "energy": 2
      },
      "gain": {
        "onTime": 4
      },
      "facility": "signal",
      "xp": 10
    },
    {
      "id": "ship",
      "name": "Express Cargo",
      "zh": "快运货物",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "cargo": 8,
        "fuel": 4
      },
      "gain": {
        "coins": 92,
        "onTime": 5
      },
      "facility": "station",
      "xp": 20
    },
    {
      "id": "mailrun",
      "name": "Night Mail",
      "zh": "夜间邮车",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "mail": 8,
        "fuel": 3
      },
      "gain": {
        "coins": 70,
        "onTime": 6
      },
      "facility": "station",
      "xp": 18
    },
    {
      "id": "repair",
      "name": "Repair Switches",
      "zh": "检修道岔",
      "kind": "support",
      "turns": 1,
      "cost": {
        "parts": 3,
        "energy": 2
      },
      "gain": {
        "onTime": 5,
        "cargo": 2
      },
      "facility": "workshop",
      "xp": 14
    }
  ],
  "upgrades": [
    [
      "yard",
      "Longer Sidings",
      "加长股道",
      90
    ],
    [
      "depot",
      "Fuel Tower",
      "燃料塔",
      105
    ],
    [
      "signal",
      "Block Signals",
      "闭塞信号",
      130
    ],
    [
      "station",
      "Island Platform",
      "岛式站台",
      155
    ],
    [
      "warehouse",
      "Cargo Lift",
      "货物升降机",
      125
    ],
    [
      "workshop",
      "Tool Wall",
      "工具墙",
      115
    ]
  ],
  "events": [
    [
      "Fog Bank",
      "大雾",
      "signal",
      "lose",
      "onTime",
      3
    ],
    [
      "Holiday Mail",
      "节日邮件",
      "mailrun",
      "gain",
      "mail",
      8
    ],
    [
      "Priority Freight",
      "急件货运",
      "ship",
      "gain",
      "coins",
      55
    ],
    [
      "Broken Coupler",
      "车钩故障",
      "repair",
      "lose",
      "parts",
      2
    ]
  ],
  "goal": {
    "stat": "onTime",
    "value": 78,
    "label": "On-Time",
    "zh": "准点率"
  }
};
