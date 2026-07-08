# Publish to Paean Apps Square

Use this skill when the user wants to publish, deploy, ship, or list a static
frontend (a game or site with a top-level `index.html`) to **Paean Apps Square**
and a `*.clide.app` URL.

Deeptide ships a native **Publish** tool — this skill is the playbook for using
it well. Do not shell out, curl the API, or run `tide publish`; call the Publish
tool so the Paean token stays in-process.

## Goal

Get the project's static output onto a public `*.clide.app` URL and into the
Square gallery, under a name that reflects the content, with complete project
metadata and no leaked secrets.

## Workflow

1. **Dry run first.** Call Publish with `dry_run: true`. Read back the resolved
   publish directory, file/byte summary, the chosen title, secret scan, remix
   API fields, and any asset warnings.
2. **Name it well.** Don't publish under the bare directory name. From the
   game's `index.html` title/heading, `clide.json`, or `package.json`, pick a
   `title` that reflects the theme and gameplay (e.g. "Neon Drift Racer"). Add
   `summary`, `category`, and `tags` when they improve the listing.
3. **Confirm.** Publishing is public — the site is reachable at `*.clide.app`
   and listed in Square. If `favicon.svg` or `banner.jpg` is missing, tell the
   user these listing assets are recommended but non-blocking. Get explicit user
   confirmation.
4. **Publish.** Call Publish (no `dry_run`) with the chosen metadata.
5. **Report** the `*.clide.app` URL, Square app hash, workspace hash, and file
   count.

## Behavior to rely on

- Auth comes from `tide auth login` (or `PAEAN_AUTH_TOKEN`), even when another
  model provider is active.
- Built output is preferred over source: `dist`, `build`, `out`,
  `.output/public`, then `public`; the published dir must hold a top-level
  `index.html`. If there's no output but a build script exists, build first.
- The published directory must contain top-level `index.html` and should contain
  top-level `favicon.svg` plus `banner.jpg` at exactly 800x400. Missing or
  wrong-size listing assets warn but do not block publish.
- `.clideignore`, `clide.json`, and `LICENSE` are ensured before upload;
  `clide.json` and `.remix-sources/` are not published.
- Secrets, `.env`, credentials, `.git`, `node_modules`, logs, and source maps
  are excluded, and text assets are scanned for high-confidence secrets.
- If `clide.json` records a remix (from the **remix** skill), the publish
  forwards the upstream lineage so original creators are credited:
  `remix.parent` maps to backend `remixOfHashKey`; all direct entries in
  `remix.parents[]` map to backend `remixOfHashKeys`.

## Copyright and lineage

- Publish only work the project is allowed to distribute. Keep compatible
  upstream license notices and attribution when a remix borrows visible assets,
  code structure, sounds, or gameplay.
- Do not publish `.remix-sources/` or verbatim upstream code/assets as the new
  top-level game. The top-level `index.html` and assets should be a new combined
  work with its own title, storage keys, and identity.
- `remix.parent` is the single primary parent: the most important direct source
  for tree-style displays. `remix.parents[]` is the direct-parent DAG adjacency
  list used for credit/revenue graphing. Include direct sources only; do not add
  transitive ancestors unless the remix directly used them.

## Failure handling

- Auth missing/expired → run `tide auth login`.
- No top-level `index.html` → publish the build output, not the source.
- Missing `favicon.svg` / `banner.jpg` → offer to generate them; publishing can
  still proceed if the user confirms.
