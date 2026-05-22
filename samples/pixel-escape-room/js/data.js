window.PIXEL_GAME_DATA = {
  "slug": "pixel-escape-room",
  "title": "Pixel Escape Room",
  "zhTitle": "像素密室逃脱",
  "tagline": "Inspect rooms, combine clues, and unlock layered puzzles.",
  "zhTagline": "搜查房间、组合线索，解开层层机关。",
  "port": 4199,
  "upgradeCurrency": "clues",
  "theme": {
    "bg": "#11151c",
    "panel": "#1c2330",
    "accent": "#f4d35e",
    "accent2": "#8ecae6",
    "danger": "#ee964b",
    "water": "#406882",
    "soil": "#6d597a"
  },
  "resources": [
    [
      "time",
      "Time",
      "时间",
      40
    ],
    [
      "clues",
      "Clues",
      "线索",
      0
    ],
    [
      "keys",
      "Keys",
      "钥匙",
      0
    ],
    [
      "tools",
      "Tools",
      "工具",
      1
    ],
    [
      "codes",
      "Codes",
      "密码",
      0
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
      "rooms",
      "Rooms",
      "房间",
      0
    ],
    [
      "chapter",
      "Chapter",
      "章节",
      1
    ]
  ],
  "facilities": [
    [
      "foyer",
      "Foyer",
      "门厅",
      "home",
      1
    ],
    [
      "study",
      "Study",
      "书房",
      "table",
      0
    ],
    [
      "lab",
      "Hidden Lab",
      "隐藏实验室",
      "chip",
      0
    ],
    [
      "gallery",
      "Gallery",
      "画廊",
      "crystal",
      0
    ],
    [
      "workbench",
      "Workbench",
      "工作台",
      "gear",
      0
    ],
    [
      "exit",
      "Exit Door",
      "出口门",
      "target",
      0
    ]
  ],
  "actions": [
    {
      "id": "search",
      "name": "Search Room",
      "zh": "搜查房间",
      "kind": "producer",
      "turns": 1,
      "cost": {
        "time": 3,
        "energy": 1
      },
      "gain": {
        "clues": 6
      },
      "facility": "foyer",
      "xp": 8
    },
    {
      "id": "decode",
      "name": "Decode Notes",
      "zh": "解读笔记",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "clues": 5,
        "energy": 1
      },
      "gain": {
        "codes": 3
      },
      "facility": "study",
      "xp": 12
    },
    {
      "id": "repair",
      "name": "Repair Tool",
      "zh": "修复工具",
      "kind": "craft",
      "turns": 1,
      "cost": {
        "clues": 4,
        "time": 2
      },
      "gain": {
        "tools": 2
      },
      "facility": "workbench",
      "xp": 10
    },
    {
      "id": "gallery",
      "name": "Align Paintings",
      "zh": "排列画作",
      "kind": "puzzle",
      "turns": 2,
      "cost": {
        "codes": 2,
        "energy": 2
      },
      "gain": {
        "keys": 1,
        "rooms": 1
      },
      "facility": "gallery",
      "xp": 18
    },
    {
      "id": "lab",
      "name": "Power Console",
      "zh": "启动控制台",
      "kind": "puzzle",
      "turns": 2,
      "cost": {
        "tools": 2,
        "codes": 3
      },
      "gain": {
        "keys": 2,
        "rooms": 2
      },
      "facility": "lab",
      "xp": 24
    },
    {
      "id": "unlock",
      "name": "Open Exit",
      "zh": "打开出口",
      "kind": "sell",
      "turns": 2,
      "cost": {
        "keys": 4,
        "codes": 4
      },
      "gain": {
        "rooms": 5
      },
      "facility": "exit",
      "xp": 30
    }
  ],
  "upgrades": [
    [
      "foyer",
      "UV Lamp",
      "紫外灯",
      70
    ],
    [
      "study",
      "Cipher Wheel",
      "密码盘",
      95
    ],
    [
      "workbench",
      "Fine Tools",
      "精密工具",
      105
    ],
    [
      "gallery",
      "Frame Marks",
      "画框刻痕",
      115
    ],
    [
      "lab",
      "Backup Power",
      "备用电源",
      140
    ],
    [
      "exit",
      "Master Keyway",
      "主锁芯",
      160
    ]
  ],
  "events": [
    [
      "Loose Tile",
      "松动地砖",
      "search",
      "gain",
      "keys",
      1
    ],
    [
      "False Lead",
      "假线索",
      "decode",
      "lose",
      "clues",
      3
    ],
    [
      "Clock Chime",
      "钟声提示",
      "gallery",
      "gain",
      "codes",
      2
    ],
    [
      "Rusty Lock",
      "生锈门锁",
      "unlock",
      "lose",
      "tools",
      1
    ]
  ],
  "goal": {
    "stat": "rooms",
    "value": 16,
    "label": "Rooms",
    "zh": "房间"
  }
};
