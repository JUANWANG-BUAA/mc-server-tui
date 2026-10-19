# Website

The landing page is a dependency-free static site in [`site/`](../site/). It is
deployed to GitHub Pages by [`.github/workflows/pages.yml`](../.github/workflows/pages.yml).

- **Production URL**: https://juanwang-buaa.github.io/mc-server-tui/
- **Source**: `site/index.html`, `site/assets/styles.css`, `site/assets/app.js`
- **Assets**: terminal screenshots copied from `output/playwright/`, plus a pixel
  Steve favicon
- **Deploy trigger**: changes under `site/**` or the Pages workflow on `main`

## Preview locally

```sh
python3 -m http.server 8099 --directory site
```

Open <http://127.0.0.1:8099/>. There is no build step or package manager.

## What to check

- 1440px desktop hero and each section at 1000px viewport height;
- 390px mobile hero, theme picker, install block, architecture diagram;
- theme buttons update both screenshots, accent color, labels, and persist;
- copy button writes the four install lines;
- no horizontal overflow, console errors, missing images, or broken in-page anchors;
- `prefers-reduced-motion` leaves all content visible;
- canonical URL, Open Graph image, JSON-LD, sitemap, robots, and 404 page remain
  valid when the repository is renamed (update `site/` URLs if that happens).

## Design notes

The page deliberately uses system fonts and no framework: it should render
offline and deploy as a plain artifact. The visual language mirrors the TUI:
near-black surfaces, one accent color, mono microcopy, real 80×24 screenshots,
and one animated pixel Steve.
