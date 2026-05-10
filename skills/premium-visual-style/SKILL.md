# Premium Visual Style For Samples

Use this skill when building or refining a sample that must feel polished,
cohesive, and professionally designed: games, tools, dashboards, landing pages,
presentation decks, and interactive demos.

## Goal

Create interfaces that look intentional at first glance and remain usable after
the novelty fades. The work should feel like a high-quality product sample, not
a placeholder or prototype.

## Design Baseline

- **Choose a domain-appropriate tone.** Games can be expressive; operational
  tools should be quiet and information-dense; decks should be editorial and
  export-friendly.
- **Make the primary subject visible immediately.** The first viewport should
  show the game, product, slide, board, canvas, terminal, or artifact itself.
- **Use restrained, repeatable systems.** Define spacing, radii, borders,
  surface colors, typography, and interaction states once.
- **Prefer real structure over decoration.** Grids, panels, metrics, terminals,
  cards, and gameplay surfaces are stronger than decorative blobs.
- **Design for scanning.** Headings, labels, values, controls, and feedback
  should be easy to locate under time pressure.

## Color Guidance

Build a palette with roles:

```css
:root {
  --bg: #060910;
  --surface: #0d1728;
  --surface-2: #101d31;
  --line: rgba(125, 211, 252, 0.18);
  --text: #eaf4ff;
  --dim: #9aaec4;
  --cyan: #3dd8eb;
  --blue: #0ea5e9;
  --green: #10b981;
  --amber: #fbbf24;
}
```

Use accents sparingly:

- One primary accent for calls to action.
- One success/resource color.
- One warning/damage color.
- One neutral line and panel system.
- Avoid making the whole UI variations of a single hue.

## Layout Standards

- Use stable dimensions for boards, canvases, decks, cards, HUD bars, and icon
  buttons.
- Avoid nested cards. Use cards for repeated items, modals, framed tools, and
  true content units.
- Keep sections full-width or canvas-centered; avoid floating marketing cards
  unless they are the actual content.
- Preserve clear hierarchy: title, state, primary surface, secondary controls,
  log/details.
- On mobile, prioritize the interactive surface and the next required action.

## Typography

- Use system fonts unless a project already has a font system.
- Keep letter spacing at `0` for normal text; use positive tracking only for
  small uppercase labels.
- Avoid viewport-width font scaling for compact UI components.
- Make text fit inside buttons and cards. Use wrapping, shorter labels, or
  container-specific font sizes.

Example:

```css
.eyebrow {
  color: var(--cyan);
  font-size: 12px;
  font-weight: 800;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}

.card-title {
  font-size: 16px;
  line-height: 1.15;
  letter-spacing: 0;
}
```

## Motion And Feedback

Use motion to explain state:

- Button hover: small border/color shift.
- Game hit: flash, particle burst, damage number.
- Drag/drop: selection outline and invalid placement feedback.
- Wave/reward: short log entries, clear phase labels.
- Deck export: print button with a direct action.

Avoid long ornamental animations that compete with interaction.

## Verification Checklist

- Desktop screenshot looks complete without scrolling unless the format is a
  long page.
- Mobile screenshot has no horizontal overflow or clipped text.
- Text does not overlap buttons, cards, canvas, or HUD.
- The visual palette has enough contrast and is not a one-note hue.
- The first viewport communicates what the sample is.
- The implementation does not rely on external assets unless intentionally
  required.

## When In Doubt

Prefer fewer, better components. A polished sample with a strong board, clear
HUD, tight controls, and three excellent effects beats a cluttered sample with
ten half-finished panels.
