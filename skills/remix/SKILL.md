# Remix Paean Apps Square Games

Use this skill when the user wants to remix, fork, combine, or mash up one or
more published `*.clide.app` / Paean Apps Square games (usually by hash) into a
brand-new game.

Deeptide ships a native **Remix** tool — this skill is the playbook for using
it. Do not shell out or curl the API; call the Remix tool so the Paean token
stays in-process.

## Goal

Pull the source of each upstream game, then build a genuinely new game that
combines the chosen aspects (the classic recipe is *game A's gameplay + game B's
art + game C's theme*), while recording the multi-parent remix lineage so every
upstream creator can be credited.

## Workflow

1. **Dry run first.** Call Remix with `dry_run: true` and the user's `sources`.
   Each source must resolve to a Square `hashKey`: bare `hashKey`,
   `hashKey.8x.gg`, `https://8x.gg/hashKey`, `https://x.8x.gg/pub/hashKey`, or
   `hashKey=role` to tag the borrowed aspect (e.g. `h1=gameplay`, `h2=art`,
   `h3=theme`). A `*.clide.app` URL is a deployed handle, not necessarily the
   Square `hashKey`; ask for the hashKey or resolve it from the backend before
   calling Remix. Review the resolved titles/authors and the planned remix graph.
2. **Confirm.** Remixing records lineage for each source — it credits each
   upstream creator and counts as a remix on their listing. Get explicit
   confirmation.
3. **Download + scaffold.** Call Remix (no `dry_run`). The target directory then
   holds `.remix-sources/<hash>/` (each upstream's full source), `clide.json`
   (the remix graph), `LICENSE`, and `.clideignore`.
4. **Build the new game.** Read each `.remix-sources/<hash>/` and synthesize a
   new game at the target top level, honoring each source's assigned aspect. Do
   NOT ship the upstream sources verbatim — combine the chosen aspects into a
   new `index.html` (and assets). `.remix-sources/` stays local and unpublished.
   Preserve compatible attribution/license notices, change project identity and
   storage/save keys, and add top-level `favicon.svg` plus `banner.jpg` at
   exactly 800x400 when possible.
5. **Finalize `clide.json`.** Set a fitting `title`, `summary`, `category`, and
   `tags`, and set each `remix.parents[].role` to the aspect actually borrowed
   (adjust `weight` for an uneven upstream split).
6. **Publish** with the **publish** skill from the target directory; it reads
   `clide.json` for naming, metadata, and lineage automatically.

## The remix graph (`clide.json`)

Lineage is recorded in two compatible shapes at once:

- `remix.parent` — the single primary upstream (tree form; mirrors the backend's
  `remixOfHashKey` field). Pick the strongest direct source, not a transitive
  ancestor.
- `remix.parents[]` — every direct upstream with its borrowed `role` and a
  revenue `weight`. Publish forwards these as backend `remixOfHashKeys`. This is
  the adjacency list of the multi-parent remix DAG, ready for upstream
  revenue-sharing. Include direct sources only; do not copy transitive ancestors
  unless the remix directly used them.

## Copyright and assets

- Treat `.remix-sources/` as reference material. Do not publish it or copy an
  upstream project unchanged as the new game.
- Keep license-compatible attribution for borrowed art, code, sounds, writing,
  mechanics, and recognizable style. If compatibility is unclear, ask before
  publishing.
- `favicon.svg` and 800x400 `banner.jpg` are expected Square listing assets for
  remix and create workflows. Publish warns if they are missing, but it can still
  submit after user confirmation.

## Failure handling

- Auth missing/expired → run `tide auth login`.
- A source that is not found / not listed / not remixable is reported before any
  download; fix or drop that source and retry.
- If the user only gives a `*.clide.app` play URL, get the Square hashKey first;
  the handle alone is not reliable lineage input.
