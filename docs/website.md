# PRYN installation website

The static installation/download page is in `site/`. It is dependency-free HTML, CSS, and JavaScript; there is no npm build, external font service, analytics tracker, or backend.

## Preview locally

From the repository root:

```bash
python -m http.server 4173 --directory site
```

Open `http://localhost:4173`.

## Deploy

The `.github/workflows/pages.yml` workflow uploads `site/` and deploys it through GitHub Pages when changes are pushed to `main`.

One-time repository setting:

1. Open the repository on GitHub.
2. Go to **Settings → Pages**.
3. Under **Build and deployment**, select **GitHub Actions** as the source.
4. Merge the site changes to `main`, then check the Pages workflow run.

Use the exact URL shown in the deployment job's `github-pages` environment. Do not assume the final domain or add a custom domain until GitHub reports the successful deployment address.

## Update the release shown on the page

Edit `site/release.json` for each release. Keep the following in sync with actual uploaded release assets:

- `version` and `tag`;
- `debAsset`, `rpmAsset`, `archiveAsset`, `pkgbuildAsset`, and `checksumsAsset`;
- `aurPublished`, which must remain `false` until the package is actually visible in the AUR.

Review every install command in `site/index.html` and `docs/installation.md` before deployment. The page uses explicit versioned release URLs so a future release cannot silently redirect users to an unintended asset.

## Design and accessibility notes

The page uses a dark, high-contrast grid layout inspired by minimal developer-product download pages. It is original PRYN branding, not a copy of another project's artwork or text. It has keyboard-operable distro tabs, responsive layouts, reduced-motion support, visible install commands when JavaScript config loading fails, and no third-party dependencies.
