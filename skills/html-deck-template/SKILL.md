# HTML Deck Template

Use this skill when creating a presentation deck as HTML/CSS that must work for
desktop presenting, mobile reading, and browser-to-PDF export.

## Goal

Build a deck that behaves like a strong web page and exports like a polished
slide deck. The implementer should be able to replace copy and duplicate slides
without reworking the layout system.

## File Structure

Recommended static structure:

```text
deck-template/
  index.html
  style.css
  README.md
```

Keep it dependency-free unless the user explicitly asks for a framework.

## Slide Architecture

Use semantic sections:

```html
<main class="deck">
  <section class="slide cover" id="cover">...</section>
  <section class="slide split" id="problem">...</section>
  <section class="slide product" id="product">...</section>
  <section class="slide workflow" id="workflow">...</section>
</main>
```

Each slide should include:

- A clear kicker or section label.
- One dominant headline.
- One primary visual or structured content unit.
- A footer with title/context and slide number.

## Desktop Layout

- Use 16:9 slides.
- Center the deck with a reasonable max width.
- Make each slide nearly viewport-height but not taller than the available
  screen.
- Use strong hierarchy and ample negative space.
- Use product-specific visuals: terminal windows, workflow diagrams, metrics,
  screenshots, tables, or proof cards.

Example:

```css
.deck {
  width: calc(100% - 32px);
  max-width: 1240px;
  margin: 0 auto;
}

.slide {
  width: 100%;
  aspect-ratio: 16 / 9;
  min-height: min(820px, calc(100dvh - 96px));
  padding: clamp(38px, 4.2vw, 58px);
  border-radius: 24px;
}
```

## Mobile Layout

Mobile decks should become readable documents:

- Remove strict 16:9.
- Stack columns into one column.
- Reduce headline size.
- Keep the deck width within `100vw`.
- Prevent terminal/code blocks from causing horizontal overflow.
- Hide dense navigation links, but keep the primary action if useful.

Example:

```css
@media (max-width: 920px) {
  .deck {
    width: calc(100vw - 20px);
    max-width: calc(100vw - 20px);
  }

  .slide {
    aspect-ratio: auto;
    min-height: auto;
    width: calc(100vw - 20px);
    padding: 34px 20px 58px;
  }

  .cover-grid,
  .timeline,
  .proof-grid {
    grid-template-columns: 1fr;
  }
}
```

## PDF Export

Add a print button:

```html
<button type="button" id="printBtn">Export PDF</button>
<script>
  document.getElementById('printBtn').addEventListener('click', () => window.print());
</script>
```

Use print CSS:

```css
@page {
  size: 16in 9in;
  margin: 0;
}

@media print {
  .deck-nav {
    display: none;
  }

  .deck {
    display: block;
    width: 16in;
    margin: 0;
    padding: 0;
  }

  .slide {
    width: 16in;
    height: 9in;
    min-height: 9in;
    border-radius: 0;
    break-after: page;
    page-break-after: always;
  }
}
```

Recommended user-facing export instructions:

- Destination: Save as PDF
- Layout: Landscape
- Margins: None
- Background graphics: Enabled

## Content Pattern

A strong product deck usually includes:

1. Cover: product or template promise.
2. Problem: why the audience should care.
3. Product: what exists and what it does.
4. Workflow: how it works in practice.
5. Differentiator: local, private, faster, cheaper, more beautiful, etc.
6. Proof: metrics, ecosystem, customers, screenshots, or source.
7. Template mechanics or implementation detail.
8. Close: next action and links.

## Quality Checklist

- One slide per PDF page.
- No clipped headlines on desktop.
- No horizontal overflow on mobile.
- Navigation is hidden or simplified on mobile.
- Product visuals are visible in the first slide.
- The deck can be opened directly as a static file if relative assets permit.
- README explains run and export instructions.
