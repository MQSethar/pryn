# PRYN website

The public installation website is a dependency-free static page in `site/`. It uses original PRYN branding, a high-contrast dark theme, an optional light theme, system-font typography, and inline Wi-Fi, Bluetooth and audio icons. There is no npm build, remote font service, analytics tracker or backend.

## Preview locally

From the repository root:

```bash
python3 -m http.server 4173 --directory site
```

Open `http://localhost:4173`. Stop the local server with `Ctrl+C`.

## Deploy with GitHub Pages

The `.github/workflows/pages.yml` workflow uploads `site/` and deploys it through GitHub Pages when site files change on `main`.

One-time repository setting:

1. Open **Settings → Pages**.
2. Under **Build and deployment**, select **GitHub Actions** as the source.
3. Save the setting if GitHub shows a Save button.
4. Open **Actions** and inspect **Deploy PRYN installation website**.
5. Use the URL shown in the successful `github-pages` deployment environment.

If `actions/configure-pages` reports `404 Not Found`, the repository Pages site has not been created or enabled for Actions. Re-check the Pages source setting before rerunning the workflow. Do not repeatedly rerun a failed workflow without fixing that setting.

## Update the release shown on the page

Edit `site/release.json` for each release. Keep these values synchronized with actual, publicly uploaded release assets:

- `version` and `tag`;
- `debAsset`, `rpmAsset`, `archiveAsset`, `pkgbuildAsset` and `checksumsAsset`;
- `aurPublished`, which must remain `false` until the package is publicly visible in the AUR;
- `aurUrl`, the canonical AUR package page.

The installer buttons download the package file. The command beside each button only installs the file already saved in `~/Downloads`; it must not repeat the download. Review every command against the generated release assets before deployment.

## Brand and accessibility

The PRYN symbol is `site/assets/pryn-mark.svg`; the browser icon is `site/favicon.svg`. The mark is used in the header and footer. The page supports dark and light themes, remembers the selected theme when browser storage is available, uses a high-contrast focus state, supports keyboard operation of the distro tabs, collapses navigation on small screens, and respects reduced-motion preferences. All icons are inline SVGs or local files; no third-party icon or font CDN is used.
