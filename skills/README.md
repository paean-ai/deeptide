# Sample-Derived Coding Skills

This directory turns the repository's strongest samples into reusable agent
skills. Each skill is a self-contained `SKILL.md` written for agents that need
to produce high-quality code, visual systems, or workflow artifacts without
rediscovering the same product and engineering patterns.

## Skills

- `pixel-art-code` - create pixel art, sprites, tiles, glyphs, and effects with
  code-native Canvas, SVG, HTML, and CSS.
- `premium-visual-style` - design polished, world-class visual systems for
  samples, tools, games, and decks.
- `html-deck-template` - build responsive, PDF-ready HTML presentation decks.
- `roguelike-game-template` - implement roguelike and roguelite game samples
  with satisfying loops, procedural systems, and progression.
- `casual-game-systems` - design casual, idle, merge, backpack, match-3, tower
  defense, and arcade game loops.
- `responsive-canvas-game` - implement robust desktop and mobile Canvas game
  shells, input, HUD, rendering, and verification.
- `coding-agent-delivery` - turn local coding-agent work into reviewable PR,
  handoff, verification, and risk artifacts.
- `research-brief-delivery` - produce decision-ready research briefs,
  comparison memos, evidence matrices, and action plans.
- `publish` - publish a static frontend to Paean Apps Square and `*.clide.app`
  via the native Publish tool, with good naming and complete metadata.
- `remix` - remix one or more published Square games (by hash) into a new game
  via the native Remix tool, recording the multi-parent remix lineage.

These are documentation skills, not runtime dependencies. They should remain
plain Markdown so agents can load them cheaply and apply them in any project.
The `publish` and `remix` skills are the playbooks for Deeptide's built-in
Publish and Remix tools (also exposed as the `/publish` and `/remix`
slash-command skills).
