# Publish to Clide Hosting or Paean Apps Square

Use this skill when the user wants to host or publish a static frontend with a top-level
`index.html` on `*.clide.app`, optionally with an Apps Square listing.

Deeptide ships a native **Publish** tool. Use it directly so credentials stay in-process; do
not shell out, curl the API, or run `tide publish`.

## Select the mode

| Intent | Publish input | Result |
|---|---|---|
| Clide hosting only; keep it out of the gallery | `hosting_only: true` | Public static site, no Paean workspace and no Apps Square row |
| Explicit Apps Square/list/gallery request | omit `hosting_only` | Workspace, public Square listing, and Clide site |

Do not turn a hosting request into a Square listing. Both modes create a public URL, so the
user must confirm the selected mode immediately before a real upload.

## Workflow

1. Call Publish with `dry_run: true` plus the final `hosting_only`, `dir`, and `handle` values.
2. Review the resolved directory, files/bytes, secret scan, destination, effective handle, and
   runtime compatibility.
3. Explain the exact public effect and get confirmation.
4. Call Publish again without `dry_run`, preserving the reviewed mode and arguments.
5. Report mode, URL, handle, file count, and `.clide/publish.json`. Report workspace/Square
   hashes only in Square mode.

## Runtime boundary

Publish uploads static browser files. It does not deploy Worker code, execute D1 migrations,
create D1/R2/KV/Durable Object resources, or configure Worker bindings/routes.

Wrangler/Worker binding detection returns a blocked runtime status in dry-run and stops a real
upload. Set `allow_static_only: true` only after the user explicitly accepts that the backend
will not be deployed and the frontend may be non-functional. Prefer the project's Worker
deployment workflow.

## Handles and repeat deploys

- `handle` requests a custom `*.clide.app` subdomain. The server decides validity,
  availability, subscription eligibility, and credit cost.
- Hosting-only repeat deploys reuse the handle saved in `.clide/publish.json` when `handle` is
  omitted.
- Passing a different handle for an already tracked hosting-only project is blocked rather
  than silently creating a second site.
- 400 means malformed/reserved, 402 means the account is not eligible, and 409 means taken.
  Do not retry the same value unchanged.

## Packaging and security

- Built output is preferred: `dist`, `build`, `out`, `.output/public`, then `public`; the
  selected directory must contain top-level `index.html`.
- `.clideignore` excludes credentials, `.env`, keys, `.git`, dependencies, local publish
  state, logs, source maps, and editor/OS files.
- Included text assets are scanned for high-confidence secrets. Use `allow_secrets` only after
  the user accepts a concrete finding; excluding the file is preferred.
- Square mode carries listing metadata, license, assets, and remix lineage. Hosting-only mode
  never creates or updates an Apps Square listing.
- Authentication comes from `tide auth login` or `PAEAN_AUTH_TOKEN`; never expose the token in
  tool output or chat.
