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
   Each source may be a bare hashKey, `hash.8x.gg`, `hash.clide.app`, an https
   URL, or `hash=role` to tag the borrowed aspect (e.g. `h1=gameplay`,
   `h2=art`, `h3=theme`). Review the resolved titles/authors and the planned
   remix graph.
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
5. **Finalize `clide.json`.** Set a fitting `title`, `summary`, `category`, and
   `tags`, and set each `remix.parents[].role` to the aspect actually borrowed
   (adjust `weight` for an uneven upstream split).
6. **Publish** with the **publish** skill from the target directory; it reads
   `clide.json` for naming, metadata, and lineage automatically.

## The remix graph (`clide.json`)

Lineage is recorded in two compatible shapes at once:

- `remix.parent` — the single primary upstream (tree form; mirrors the backend's
  single-parent field).
- `remix.parents[]` — every direct upstream with its borrowed `role` and a
  revenue `weight`. This is the adjacency list of the multi-parent remix DAG,
  ready for upstream revenue-sharing.

## Failure handling

- Auth missing/expired → run `tide auth login`.
- A source that is not found / not listed / not remixable is reported before any
  download; fix or drop that source and retry.
