# Website maintenance

The site is dependency-free HTML/CSS/JavaScript. The two landing pages and two
teaching guides share a paper/forest visual system with real terminal captures.
They deploy through [Pages](../.github/workflows/pages.yml).

| Page | Source | Generated output |
| --- | --- | --- |
| English landing | `scripts/build_landing_site.py` | `site/index.html` |
| 中文宣传页 | `scripts/build_landing_site.py` | `site/zh-CN/index.html` |
| English tutorial | [USER_GUIDE.md](USER_GUIDE.md) | `site/docs/index.html` |
| 中文教程 | [USER_GUIDE.zh-CN.md](USER_GUIDE.zh-CN.md) | `site/docs/zh-CN/index.html` |

The workspace version in `Cargo.toml` controls generated metadata, cache keys,
release links and installer commands. Commit source and generated output together.

```sh
python3 scripts/build_landing_site.py
python3 scripts/build_docs_site.py
python3 scripts/check_docs.py
python3 -m http.server 8099 --directory site
```

Open <http://127.0.0.1:8099/> or `/zh-CN/`. No package installation or external
font request is required. Keep GitHub Pages’ `/mc-server-tui/` base path working.

## Review before deploying

- Both landing languages and both guides at 1440px, 768px and 390px.
- Workflow tabs by pointer and arrow/Home/End keys; selected tab and panel agree.
- Seven theme previews, correct image paths and preference persistence.
- Installation and tutorial code copy; denied clipboard access selects the text.
- Chapter filtering searches chapter content, shows an empty state, and recovers.
- Mobile chapter menu, next-chapter links, language switch and reading progress.
- No horizontal page overflow, missing images, console errors or broken anchors.
- JavaScript disabled: text, install commands and guides remain readable; hidden
  workflow panels are available through a no-script rule.
- Reduced motion: all content remains visible; printing omits navigation chrome.

The checker validates local paths, Markdown/HTML anchors, duplicate HTML IDs,
installer tags and the freshness of all four generated pages. External URLs
need separate review; successful local validation does not prove deployment.

## Screenshot provenance

`*-0.2.1.png` are actual 80×24 captures from the compiled v0.2.1 TUI with
controlled visual-daemon responses. The session/create/backup layout is retained
in v0.2.2. They are labelled as historical captures and demonstration data,
not recaptured or represented as live server metrics.
Fresh v0.2.2 first-use/error captures use the compiled v0.2.2
binary against an isolated fixture. See [release acceptance](acceptance/0.2.2.md).

When releasing, synchronize Cargo/lock versions, README files, user-facing
installation docs, changelog, release notes and acceptance. Preserve historical
versioned release and acceptance records; never relabel old evidence as new.
