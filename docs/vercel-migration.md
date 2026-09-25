# Website hosting: GitHub Pages → Vercel

**Status:** prepared but not activated. The repository side of the migration is
complete; the account-level steps (Vercel project, DNS record, custom domain)
are external actions that require credentials this repository does not hold.

## Target architecture

```text
GitHub (JoaoValentimTheo/aura-lang)
  ├── source
  ├── CI
  └── Releases
        │
        ▼
      Vercel            ← website + Playground hosting (static)
        │
        ▼
  https://aura.lang.dev
        └── /playground  → Browser → Web Worker → Aura WASM → Aura Runtime
```

GitHub remains source, CI, and releases. Vercel hosts the static site. The
Playground stays entirely client-side; Vercel runs no Aura code.

## What the repository provides

* **`vercel.json`** (root): framework-less static build.
  * build command: `AURA_SITE_BASE=/ node website/build.mjs`
  * output directory: `website/dist`
  * content types and cache headers for the wasm, worker, and assets.
* **Explicit base** (`website/lib/base.mjs`): root (`/`) for Vercel and the
  custom domain; `/aura-lang/` for the GitHub Pages project-site fallback.
* **No `CNAME` by default.** `CNAME` is emitted only when `AURA_EMIT_CNAME=1`.
  It is a GitHub Pages-only mechanism; leaving it out of the Vercel build keeps
  Vercel in charge of its own domains.

## Steps to activate (external, account-level)

These require a human with Vercel and DNS access. They are **not** performed by
the repository.

1. **Create the Vercel project.** Import `JoaoValentimTheo/aura-lang`. Vercel
   reads `vercel.json`, so the build command and output directory need no manual
   entry. Set the production branch to the branch that should publish the site.
2. **Verify the preview deployment.** Vercel builds every push to a preview URL.
   Confirm the homepage, `/docs/`, `/examples/`, `/releases/`, and `/playground/`
   load with CSS applied and the Playground executing `print(1 + 2)` → `3`.
3. **Add the domain in Vercel.** Project → Settings → Domains → add
   `aura.lang.dev`. **Use the exact DNS target Vercel reports for this project**;
   do not assume a generic target.
4. **Create the DNS record.** `lang.dev` is served by **Google Cloud DNS**
   (`ns-cloud-e{1,2,3,4}.googledomains.com`), not Cloudflare. Add the record
   Vercel specifies for `aura.lang.dev` at the DNS provider that actually hosts
   the zone. Change no other record.
5. **Verify HTTPS** once Vercel has issued the certificate, then confirm the
   public URLs:
   `https://aura.lang.dev/`, `/docs/`, `/playground/`, `/examples/`,
   `/releases/`.
6. **Retire the GitHub Pages fallback** only after the above is green: disable
   the Pages site and remove `.github/workflows/pages.yml`.

## Rollback

Until step 5 is verified, leave the GitHub Pages project-site deployment in
place. It is a working fallback. If Vercel fails, fix it and retry; do not
remove the fallback first.

## Why no CNAME is emitted for Vercel

An unconditional `CNAME` file previously made GitHub Pages claim
`aura.lang.dev` and 301-redirect the project URL to it — while the domain had
no DNS record — which broke the fallback. The file is now opt-in
(`AURA_EMIT_CNAME=1`) and used only for a deliberate Pages custom-domain
deployment. Vercel manages custom domains through its project settings and
ignores this file.
