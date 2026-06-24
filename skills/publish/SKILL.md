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
   publish directory, file/byte summary, the chosen title, and the secret scan.
2. **Name it well.** Don't publish under the bare directory name. From the
   game's `index.html` title/heading, `clide.json`, or `package.json`, pick a
   `title` that reflects the theme and gameplay (e.g. "Neon Drift Racer"). Add
   `summary`, `category`, and `tags` when they improve the listing.
3. **Confirm.** Publishing is public — the site is reachable at `*.clide.app`
   and listed in Square. Get explicit user confirmation.
4. **Publish.** Call Publish (no `dry_run`) with the chosen metadata.
5. **Report** the `*.clide.app` URL, Square app hash, workspace hash, and file
   count.

## Behavior to rely on

- Auth comes from `tide auth login` (or `PAEAN_AUTH_TOKEN`), even when another
  model provider is active.
- Built output is preferred over source: `dist`, `build`, `out`,
  `.output/public`, then `public`; the published dir must hold a top-level
  `index.html`. If there's no output but a build script exists, build first.
- `.clideignore`, `clide.json`, and `LICENSE` are ensured before upload;
  `clide.json` and `.remix-sources/` are not published.
- Secrets, `.env`, credentials, `.git`, `node_modules`, logs, and source maps
  are excluded, and text assets are scanned for high-confidence secrets.
- If `clide.json` records a remix (from the **remix** skill), the publish
  forwards the upstream lineage so original creators are credited.

## Failure handling

- Auth missing/expired → run `tide auth login`.
- No top-level `index.html` → publish the build output, not the source.
