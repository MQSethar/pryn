# PRYN website

The public website is the static site in `site/`. It has three user-facing destinations:

- `site/index.html` — product overview and the 20-second explanation.
- `site/install.html` — install, update and remove instructions for each distribution, plus Cargo.
- `site/docs/index.html` — a readable documentation page covering commands, results, subsystem checks, privacy, compatibility and architecture.

## Design and assets

The site uses the PRYN symbol supplied by the project owner at `site/assets/pryn-symbol.png`. Keep this mark as the source for header and footer branding. `site/favicon.png` uses the same mark.

The site has dark and light themes, a high-contrast layout, system font stacks, large readable body text, keyboard-operable tabs, responsive navigation and no third-party font, icon, analytics or JavaScript dependencies.

## Preview locally

From the repository root:

```bash
python3 -m http.server 4173 --directory site
```

Open `http://localhost:4173`. Stop the local server with `Ctrl+C`.

## Deploy

The `.github/workflows/pages.yml` workflow uploads `site/` and deploys it through GitHub Pages when site files change on `main`.

The repository's Pages source must be set to **GitHub Actions** under Settings → Pages.

## Update release information

Edit `site/release.json` for each release. Keep the version, tag and asset filenames synchronized with the files actually attached to the GitHub release:

- `version` and `tag`;
- `debAsset`, `rpmAsset`, `archiveAsset`, `pkgbuildAsset` and `checksumsAsset`;
- `aurPublished`, which must stay `false` until `pryn-bin` is publicly available in the AUR;
- `aurUrl`, the canonical AUR package page.

The installer page builds package filenames, download links and commands from this file. `pkgbuildUrl` points to the maintained `site/assets/pryn-bin.PKGBUILD` used for local Arch installs while AUR registration is closed. Keep its version, pkgrel, source URL and checksum in sync with the upstream release. After changing it, verify every link and every install/update command against the actual release.

## Keep documentation consistent

`docs/installation.md` is the detailed Markdown installation reference. `site/install.html` is its user-facing website counterpart. Keep their commands, package names, version and AUR status synchronized.

`site/docs/index.html` is a concise web presentation of the repository documentation. The longer technical Markdown documents remain in `docs/` and `SECURITY.md`, and the website links to those original files on GitHub for full details.
