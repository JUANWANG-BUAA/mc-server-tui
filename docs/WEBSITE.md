# Website

The landing page is a dependency-free static site in [`site/`](../site/). It is
deployed to GitHub Pages by [`.github/workflows/pages.yml`](../.github/workflows/pages.yml).

- **Production URL**: https://juanwang-buaa.github.io/mc-server-tui/
- **Source**: `site/index.html`, `site/docs/**`, `site/assets/styles.css`,
  `site/assets/app.js`, and `site/assets/docs.js`
- **Assets**: terminal screenshots copied from `output/playwright/`, plus a pixel
  Steve favicon
- **Deploy trigger**: changes under `site/**`, the two user-guide Markdown
  files, the docs generator, or the Pages workflow on `main`

The native [English guide](https://juanwang-buaa.github.io/mc-server-tui/docs/)
and [Chinese guide](https://juanwang-buaa.github.io/mc-server-tui/docs/zh-CN/)
are generated from `docs/USER_GUIDE.md` and `docs/USER_GUIDE.zh-CN.md`:

```sh
python3 scripts/build_docs_site.py
```

Commit both the Markdown source and generated pages. The generator uses only
the Python standard library.

## Preview locally

```sh
python3 -m http.server 8099 --directory site
```

Open <http://127.0.0.1:8099/>. The landing page has no build step. Documentation
pages are regenerated with the standard-library Python command above; there is
no package manager dependency.

## What to check

- 1440px desktop hero and each section at 1000px viewport height;
- 390px mobile hero, theme picker, install block, architecture diagram;
- theme buttons update both screenshots, accent color, labels, and persist;
- copy button writes the four install lines;
- documentation pages at desktop width and 390px mobile have a usable chapter
  menu, active section, code copy buttons, language switch, and no horizontal
  page overflow;
- every documentation link resolves locally except release downloads, source,
  licenses, and other explicitly secondary references;
- no horizontal overflow, console errors, missing images, or broken in-page anchors;
- `prefers-reduced-motion` leaves all content visible;
- canonical URL, Open Graph image, JSON-LD, sitemap, robots, and 404 page remain
  valid when the repository is renamed (update `site/` URLs if that happens).

## Design notes

The page deliberately uses system fonts and no framework: it should render
offline and deploy as a plain artifact. The visual language mirrors the TUI:
near-black surfaces, one accent color, mono microcopy, real 80×24 screenshots,
and one animated pixel Steve. The documentation uses a three-column reading
layout on wide screens, a chapter drawer on mobile, and printable prose.
