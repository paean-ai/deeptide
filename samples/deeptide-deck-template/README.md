# Deeptide Deck Template

A static HTML presentation template for high-quality product storytelling. It is
designed to work as:

- a desktop presentation deck,
- a mobile-readable web page,
- a clean browser-to-PDF export.

The content uses Deeptide as the reference product and borrows the visual tone
from `deeptide.sh`: deep navy surfaces, tide gradients, terminal previews,
operator proof cards, and system-font typography.

## Run

Open `index.html` directly in a browser, or serve the repository root:

```bash
python3 -m http.server 4173
```

Then visit:

```text
http://127.0.0.1:4173/samples/deeptide-deck-template/index.html
```

## Export PDF

Open the deck in Chrome or Safari and use the `Export PDF` button, or choose
Print from the browser menu.

Recommended print settings:

- Destination: Save as PDF
- Layout: Landscape
- Margins: None
- Background graphics: Enabled

The print stylesheet uses `16in x 9in` pages and keeps one slide per page.
