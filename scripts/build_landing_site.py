#!/usr/bin/env python3
"""Generate both landing pages using the workspace version and shared structure."""

import html
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.search(
    r'^version = "([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.MULTILINE
).group(1)
REPO = "https://github.com/JUANWANG-BUAA/mc-server-tui"
URL = "https://juanwang-buaa.github.io/mc-server-tui/"

COPY = {
    "en": {
        "title": "Your Minecraft world. Your terminal.",
        "description": "Create Paper and Fabric servers, follow the console, and verify world backups. A focused Minecraft Java server manager for macOS and Linux.",
        "skip": "Skip to content",
        "nav": ["The workflow", "A closer look", "Learn", "Install"],
        "alternate": "简体中文",
        "eyebrow": "MINECRAFT JAVA · MACOS + LINUX",
        "hero": "Good worlds.<br>Keep them <em>running.</em>",
        "lede": "From the first server to the next world backup. Run Paper and Fabric, follow the right console, and know what a restore will change — all from your terminal.",
        "install": "Install the alpha",
        "guide": "Read the guide",
        "release": "Unsigned alpha",
        "release_link": "What’s new",
        "screen": "Actual TUI · deterministic demonstration data",
        "hero_note": "Close the interface. Keep the world online.",
        "hero_note_body": "An independent runner owns each Java process. Return to the TUI when you need it.",
        "facts": [
            ("02", "Server families", "Paper + Fabric"),
            ("07", "Terminal themes", "One familiar workflow"),
            ("03", "Release targets", "Apple Silicon + GNU Linux"),
        ],
        "workflow_label": "01 / THE EVERYDAY WORKFLOW",
        "workflow_title": "A little less setup.<br>A lot more <em>world.</em>",
        "workflow_lede": "One place to prepare a server, see what is happening, and look after the files that matter.",
        "steps": [
            (
                "Create with context",
                "Choose an exact Paper version, review Java and memory, then accept the EULA yourself. Fabric has its own explicit command.",
                "/create",
            ),
            (
                "Keep the right console",
                "Your selected instance stays visible. Console drafts retain their destination when you switch servers.",
                "/open",
            ),
            (
                "Keep a verifiable copy",
                "Stop the server, create a snapshot, verify its files, and review the impact before confirming a restore.",
                "/backups",
            ),
        ],
        "detail_label": "02 / KNOW WHAT YOU ARE DOING",
        "detail_title": "The next action.<br><em>Already in view.</em>",
        "tabs": [
            (
                "session",
                "01",
                "Run your server",
                "The selected instance, logs, and a target-labelled composer share one workspace. Management connectivity and Minecraft readiness stay distinct.",
                "session-claude",
            ),
            (
                "create",
                "02",
                "Review before creating",
                "Pick a version and inspect a settings summary. Expand Java, memory and port with F3; downloads report their actual source and byte progress.",
                "create",
            ),
            (
                "backups",
                "03",
                "See the restore impact",
                "Local timestamps help you choose a snapshot. Verify it, inspect the affected worlds, and type the exact confirmation to restore.",
                "backups",
            ),
        ],
        "caption": "v0.2.1 capture · layout retained in v"
        + VERSION
        + " · demonstration data",
        "themes_label": "03 / MAKE IT YOUR TERMINAL",
        "themes_title": "Seven palettes.<br><em>The same clear view.</em>",
        "themes_body": "Choose a palette here to preview the actual interface. In the app, use F2 or /theme. Your command and your server stay the same.",
        "learn_label": "04 / YOUR NEXT SESSION STARTS HERE",
        "learn_title": "From “installed”<br>to <em>“I’ve got this.”</em>",
        "lessons": [
            (
                "Start your first server",
                "Install the release, check Java, create a Paper instance, and find the address for your client.",
                "install",
                "INSTALL → CREATE → CONNECT",
            ),
            (
                "Take care of a world",
                "Follow the stop, backup, verify and restore sequence. Know when the server needs to be started again.",
                "backups-restore",
                "STOP → BACK UP → VERIFY",
            ),
            (
                "Find a useful answer",
                "Read the failure reason, expand the evidence, and check the matching recovery action without losing your draft.",
                "troubleshooting",
                "REASON → EVIDENCE → ACTION",
            ),
        ],
        "lesson_link": "Open the lesson",
        "install_label": "05 / READY WHEN YOU ARE",
        "install_title": "One install.<br><em>Your next server.</em>",
        "install_body": "Install for your current user, run setup, then open the TUI. The installer selects the artifact for your machine.",
        "copy": "Copy commands",
        "copied": "Copied",
        "failed": "Select commands",
        "install_note": "Java must match your game version. Paper discovers an installed runtime. Start with an 80 × 24 terminal.",
        "platforms": "macOS 13+ Apple Silicon · Linux x86_64 / arm64",
        "source": "Build from source",
        "source_body": "Prefer to build it yourself? Use the pinned Rust toolchain and the locked dependency versions.",
        "support_title": "Know the current scope.",
        "support": [
            (
                "Available now",
                "Paper / Fabric · console and logs · controlled restart · four common settings · stopped-world backups · compatible ZIP/MRPACK import",
            ),
            (
                "Still ahead",
                "Vanilla · existing-directory takeover · online/scheduled backups · Forge/NeoForge · signed releases",
            ),
        ],
        "alpha_note": "v"
        + VERSION
        + " is an unsigned alpha. SHA-256 checks verify integrity, not publisher identity. Keep an independent backup of important worlds.",
        "faq_title": "Before you start.",
        "faqs": [
            (
                "Does closing the TUI stop Minecraft?",
                "No. Each instance has a resident runner. /quit or Ctrl-C exits the interface; use /stop to stop the selected Minecraft server.",
            ),
            (
                "Will unfinished tasks resume after a daemon restart?",
                "Task history persists. Queued/running tasks become interrupted and are not automatically replayed. A retained partial download may be reused by a new attempt.",
            ),
            (
                "Can my friends join as soon as the server says ready?",
                "Readiness confirms the local server state. Use /connect for address hints, then check client version, firewall and port forwarding separately.",
            ),
            (
                "What has actually been tested?",
                "Local Rust and PTY checks cover the application. An earlier isolated Paper drill covered backup and restore. Real client world verification and a real Linux user-service drill remain open.",
            ),
        ],
        "footer": "A calmer place to run your Minecraft server.",
        "safety": "Scope, evidence & limitations",
        "github": "Source on GitHub",
    },
    "zh-CN": {
        "title": "你的 Minecraft 世界，你的终端。",
        "description": "在终端创建 Paper 与 Fabric 服务器，查看实时控制台，校验和恢复世界备份。面向 macOS 与 Linux 的 Minecraft Java 服务端管理工具。",
        "skip": "跳到正文",
        "nav": ["管理流程", "界面一览", "使用教程", "安装"],
        "alternate": "English",
        "eyebrow": "MINECRAFT JAVA · MACOS + LINUX",
        "hero": "好世界，<br>值得<em>一直在线。</em>",
        "lede": "从第一个服务器，到下一份世界备份。在终端创建 Paper 与 Fabric 服务器、跟进正确的控制台，并在恢复前看清影响。把时间留给你的世界。",
        "install": "安装 alpha 版",
        "guide": "阅读使用教程",
        "release": "未签名 alpha",
        "release_link": "本版更新",
        "screen": "真实 TUI · 确定性演示数据",
        "hero_note": "关掉界面，世界继续在线。",
        "hero_note_body": "每个 Java 进程由独立 runner 持有。需要管理时，再回到终端。",
        "facts": [
            ("02", "服务端类型", "Paper + Fabric"),
            ("07", "终端主题", "同一套熟悉的操作"),
            ("03", "发布平台", "Apple Silicon + GNU Linux"),
        ],
        "workflow_label": "01 / 每天都会用到的流程",
        "workflow_title": "少一点配置周旋，<br>多一点<em>世界时间。</em>",
        "workflow_lede": "准备服务端、查看运行情况、照看世界文件，在一个地方完成。",
        "steps": [
            (
                "建服之前，先看清楚",
                "选择精确的 Paper 版本，确认 Java 和内存，亲自接受 EULA。Fabric 通过独立命令明确创建参数。",
                "/create",
            ),
            (
                "让控制台始终有目标",
                "当前实例始终可见。切换服务器时，控制台草稿保留原有目标，避免一句命令发错世界。",
                "/open",
            ),
            (
                "让备份可以被核验",
                "停止服务端、创建快照、校验文件。恢复前检查受影响的世界，再明确确认。",
                "/backups",
            ),
        ],
        "detail_label": "02 / 每一步，都知道下一步",
        "detail_title": "当前实例，<br><em>始终在眼前。</em>",
        "tabs": [
            (
                "session",
                "01",
                "管理运行中的世界",
                "实例、日志与标注目标的输入框共用一个工作区。管理连接是否正常、Minecraft 是否就绪，分别显示。",
                "session-claude",
            ),
            (
                "create",
                "02",
                "提交前检查设置",
                "先选择版本，再查看运行设置摘要。F3 展开 Java、内存和端口；下载按真实来源和字节进度反馈。",
                "create",
            ),
            (
                "backups",
                "03",
                "恢复之前看清影响",
                "按本地时间找到快照，先校验，再检查世界目录和恢复计划，输入精确确认后提交。",
                "backups",
            ),
        ],
        "caption": "v0.2.1 实际截图 · v" + VERSION + " 沿用此布局 · 演示数据",
        "themes_label": "03 / 让终端更合你的习惯",
        "themes_title": "七套配色，<br><em>同样清楚。</em>",
        "themes_body": "点击配色预览真实界面。应用内按 F2 或输入 /theme 切换，命令和当前服务器保持不变。",
        "learn_label": "04 / 下一次打开，就更从容",
        "learn_title": "从“装好了”，<br>到<em>“我会用了”。</em>",
        "lessons": [
            (
                "启动第一个服务器",
                "安装发行版、检查 Java、创建 Paper 实例，再找到客户端需要的连接地址。",
                "install",
                "安装 → 创建 → 连接",
            ),
            (
                "照看一个重要世界",
                "按停服、备份、校验、恢复的顺序操作，知道什么时候需要再次启动服务器。",
                "backups-restore",
                "停服 → 备份 → 校验",
            ),
            (
                "看懂一次操作失败",
                "先看原因，再展开证据，跟随对应建议处理。关闭弹窗后，原来的草稿还在。",
                "troubleshooting",
                "原因 → 证据 → 操作",
            ),
        ],
        "lesson_link": "开始这一课",
        "install_label": "05 / 从这里，打开你的服务器",
        "install_title": "一次安装，<br><em>开始管理。</em>",
        "install_body": "按当前用户安装，完成初始化后打开 TUI。安装器会为你的机器选择对应产物。",
        "copy": "复制安装命令",
        "copied": "已复制",
        "failed": "请选择命令",
        "install_note": "Java 需要匹配游戏版本，Paper 向导检测本机已有运行时。建议使用 80 × 24 或更大的终端。",
        "platforms": "macOS 13+ Apple Silicon · Linux x86_64 / arm64",
        "source": "从源码构建",
        "source_body": "也可以自行构建。使用仓库固定的 Rust 工具链和锁定依赖。",
        "support_title": "了解当前支持范围。",
        "support": [
            (
                "已经支持",
                "Paper / Fabric · 控制台与日志 · 受控重启 · 四项常用配置 · 停服世界备份 · 兼容 ZIP/MRPACK 导入",
            ),
            (
                "后续工作",
                "Vanilla · 接管现有目录 · 在线与定时备份 · Forge/NeoForge · 签名发布",
            ),
        ],
        "alpha_note": "v"
        + VERSION
        + " 是未签名 alpha。SHA-256 校验文件完整性，不证明发布者身份。重要世界请保留独立备份。",
        "faq_title": "开始前，你可能想知道。",
        "faqs": [
            (
                "关闭 TUI 会停止 Minecraft 吗？",
                "不会。每个实例都有常驻 runner。/quit 或 Ctrl-C 退出管理界面；需要停服时，使用 /stop。",
            ),
            (
                "daemon 重启后，未完成的任务会继续吗？",
                "任务历史会保留。排队和执行中的任务会变为 interrupted，不会自动重放。重新尝试下载时，可能复用保留的部分文件。",
            ),
            (
                "显示就绪，朋友就一定能连上吗？",
                "就绪只确认本机服务端状态。/connect 提供地址提示，还需要分别检查客户端版本、防火墙与端口转发。",
            ),
            (
                "目前实际验证到哪一步？",
                "本机 Rust 与 PTY 检查覆盖应用行为；此前的隔离 Paper 演练覆盖备份恢复。真实客户端进入世界核验与真实 Linux 用户服务演练仍待完成。",
            ),
        ],
        "footer": "在终端里，从容照看你的 Minecraft 世界。",
        "safety": "支持范围、证据与限制",
        "github": "GitHub 源码",
    },
}


def render_page(lang):
    c = COPY[lang]
    zh = lang == "zh-CN"
    base = "../" if zh else ""
    assets = base + "assets/"
    guide = base + ("docs/zh-CN/" if zh else "docs/")
    alternate = "../" if zh else "zh-CN/"
    canonical = URL + ("zh-CN/" if zh else "")
    e = html.escape
    nav = "".join(
        f'<a href="#{anchor}">{e(label)}</a>'
        for anchor, label in zip(("workflow", "inside", "learn", "install"), c["nav"])
    )
    facts = "".join(
        f"<div><strong>{n}</strong><span>{e(label)}<small>{e(detail)}</small></span></div>"
        for n, label, detail in c["facts"]
    )
    steps = "".join(
        f'<article class="workflow-step"><span class="step-index">0{i + 1}</span><h3>{e(title)}</h3><p>{e(body)}</p><code>{command}</code></article>'
        for i, (title, body, command) in enumerate(c["steps"])
    )
    tabs, panels = [], []
    for index, (key, num, title, body, shot) in enumerate(c["tabs"]):
        tabs.append(
            f'<button type="button" role="tab" id="tab-{key}" aria-controls="panel-{key}" aria-selected="{str(index == 0).lower()}" tabindex="{0 if index == 0 else -1}" data-view="{key}"><span>{num}</span>{e(title)}<b aria-hidden="true">↗</b></button>'
        )
        panels.append(
            f'<section class="view-panel" role="tabpanel" id="panel-{key}" aria-labelledby="tab-{key}" tabindex="0"{" hidden" if index else ""}><p class="view-description">{e(body)}</p><figure class="screen-frame"><div class="terminal-bar"><span class="terminal-dots" aria-hidden="true">● ● ●</span><span>mc / {key}</span><span>80 × 24</span></div><img src="{assets}{shot}-0.2.1.png" alt="{e(title)}" width="867" height="504" loading="lazy"><figcaption>{e(c["caption"])}</figcaption></figure></section>'
        )
    themes = [
        ("claude", "Claude", "#d97757"),
        ("deepseek", "DeepSeek", "#4d6bfe"),
        ("codex", "Codex", "#10a37f"),
        ("kimi", "Kimi", "#a78bfa"),
        ("pcl2", "PCL2", "#56b8e8"),
        ("mono", "Mono", "#d8d8d8"),
        ("ocean", "Ocean", "#6cb6c9"),
    ]
    chips = "".join(
        f'<button type="button" data-set-theme="{key}" aria-pressed="{str(i == 0).lower()}"><i style="--swatch:{color}" aria-hidden="true"></i>{label}</button>'
        for i, (key, label, color) in enumerate(themes)
    )
    lessons = "".join(
        f'<a class="lesson" href="{guide}#{anchor}"><span class="lesson-number">0{i + 1} <span aria-hidden="true">↗</span></span><small>{e(route)}</small><h3>{e(title)}</h3><p>{e(body)}</p><span class="lesson-link">{e(c["lesson_link"])} <span aria-hidden="true">→</span></span></a>'
        for i, (title, body, anchor, route) in enumerate(c["lessons"])
    )
    support = "".join(
        f"<div><h3>{e(title)}</h3><p>{e(body)}</p></div>"
        for title, body in c["support"]
    )
    faqs = "".join(
        f'<details><summary>{e(q)}<span aria-hidden="true">+</span></summary><p>{e(a)}</p></details>'
        for q, a in c["faqs"]
    )
    install = f'TAG=v{VERSION}\ncurl -fsSL "{REPO}/releases/download/$TAG/install.sh" | sh\nmc setup\nmc'
    schema = json.dumps(
        {
            "@context": "https://schema.org",
            "@type": "SoftwareApplication",
            "name": "mc-server-tui",
            "applicationCategory": "DeveloperApplication",
            "operatingSystem": "macOS, Linux",
            "softwareVersion": VERSION,
            "url": URL,
            "codeRepository": REPO,
            "license": REPO + "/blob/main/LICENSE-MIT",
            "description": c["description"],
        },
        ensure_ascii=False,
    )
    return f'''<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>mc-server-tui — {e(c["title"])}</title>
<meta name="description" content="{e(c["description"])}"><meta name="theme-color" content="#f5f6ef">
<link rel="canonical" href="{canonical}"><link rel="alternate" hreflang="en" href="{URL}"><link rel="alternate" hreflang="zh-CN" href="{URL}zh-CN/"><link rel="alternate" hreflang="x-default" href="{URL}">
<link rel="icon" href="{assets}favicon.svg" type="image/svg+xml"><link rel="stylesheet" href="{assets}styles.css?v={VERSION}">
<meta property="og:type" content="website"><meta property="og:title" content="mc-server-tui — {e(c["title"])}"><meta property="og:description" content="{e(c["description"])}"><meta property="og:url" content="{canonical}"><meta property="og:image" content="{URL}assets/session-claude-0.2.1.png"><meta name="twitter:card" content="summary_large_image">
<script type="application/ld+json">{schema}</script><script src="{assets}app.js?v={VERSION}" defer></script>
<noscript><style>.view-panel[hidden]{{display:block!important}}.view-tabs,.theme-picker,.terminal-bar button{{display:none}}</style></noscript>
</head>
<body class="landing-page" data-assets="{assets}">
<a class="skip-link" href="#main">{c["skip"]}</a>
<header class="site-header"><div class="header-inner"><a class="brand" href="{base or "./"}"><img src="{assets}favicon.svg" width="28" height="28" alt=""><span>mc-server-tui</span><span class="alpha-tag">ALPHA</span></a><nav class="nav" aria-label="{"主导航" if zh else "Main navigation"}">{nav}</nav><div class="header-actions"><a class="lang-switch" href="{alternate}" lang="{"en" if zh else "zh-CN"}" hreflang="{"en" if zh else "zh-CN"}">{c["alternate"]}</a><a class="header-github" href="{REPO}">GitHub ↗</a></div></div></header>
<main id="main">
<section class="hero wrap"><div class="hero-copy"><p class="eyebrow"><span class="square" aria-hidden="true"></span>{c["eyebrow"]}</p><h1>{c["hero"]}</h1><p class="lede">{c["lede"]}</p><div class="hero-actions"><a class="button button-primary" href="#install">{c["install"]} <span aria-hidden="true">↗</span></a><a class="text-link" href="{guide}">{c["guide"]} <span aria-hidden="true">→</span></a></div><p class="release-line">v{VERSION} <span>· {c["release"]}</span><a href="{REPO}/releases/tag/v{VERSION}">{c["release_link"]} ↗</a></p></div><div class="hero-visual"><div class="terminal-label"><span><i aria-hidden="true"></i> mc / session</span><span>80 × 24</span></div><div class="hero-screen"><img src="{assets}session-claude-0.2.1.png" alt="{"当前实例、任务与控制台输入框" if zh else "Selected server, task progress and console composer"}" width="867" height="504" fetchpriority="high"></div><p class="capture-note">{c["screen"]} · v0.2.1</p><div class="runner-note"><span class="pixel-cube" aria-hidden="true">▟</span><div><strong>{c["hero_note"]}</strong><p>{c["hero_note_body"]}</p></div></div></div></section>
<section class="facts wrap" aria-label="{"项目概览" if zh else "Project facts"}">{facts}</section>
<section class="section wrap" id="workflow"><div class="section-head"><div><p class="eyebrow">{c["workflow_label"]}</p><h2>{c["workflow_title"]}</h2></div><p>{c["workflow_lede"]}</p></div><div class="workflow-grid">{steps}</div></section>
<section class="section showcase wrap" id="inside"><div class="section-head"><div><p class="eyebrow">{c["detail_label"]}</p><h2>{c["detail_title"]}</h2></div><a class="text-link" href="{guide}#session-shell">{c["guide"]} →</a></div><div class="view-tabs" role="tablist" aria-label="{"操作流程预览" if zh else "Workflow preview"}">{"".join(tabs)}</div>{"".join(panels)}</section>
<section class="themes-section" id="themes"><div class="wrap themes-layout"><div><p class="eyebrow">{c["themes_label"]}</p><h2>{c["themes_title"]}</h2><p class="themes-body">{c["themes_body"]}</p><div class="theme-picker" role="group" aria-label="{"预览终端主题" if zh else "Preview a terminal theme"}">{chips}</div><p class="theme-current"><span aria-hidden="true">●</span> <span id="theme-preview-label" aria-live="polite">Claude</span></p></div><figure class="theme-screen"><img id="theme-shot" src="{assets}session-claude-0.2.1.png" alt="Claude terminal theme" width="867" height="504" loading="lazy"><figcaption>{e(c["caption"])}</figcaption></figure></div></section>
<section class="section wrap" id="learn"><div class="section-head"><div><p class="eyebrow">{c["learn_label"]}</p><h2>{c["learn_title"]}</h2></div><a class="text-link" href="{guide}">{c["guide"]} →</a></div><div class="lesson-grid">{lessons}</div></section>
<section class="install-section wrap" id="install"><div class="install-copy"><p class="eyebrow">{c["install_label"]}</p><h2>{c["install_title"]}</h2><p>{c["install_body"]}</p><p class="platforms">{c["platforms"]}</p><a class="text-link" href="{guide}#install">{c["guide"]} →</a></div><div class="install-code"><div class="terminal-bar"><span>TERMINAL / v{VERSION}</span><button type="button" data-copy-target="install-command" data-copied="{c["copied"]}" data-failed="{c["failed"]}">{c["copy"]}</button></div><pre><code id="install-command">{e(install)}</code></pre><p>{c["install_note"]}</p><details class="source-install"><summary>{c["source"]}</summary><p>{c["source_body"]}</p><pre><code>git clone {REPO}.git
cd mc-server-tui
cargo build --workspace --release --locked
./target/release/mc setup
./target/release/mc</code></pre></details></div></section>
<section class="support wrap"><h2>{c["support_title"]}</h2><div class="support-grid">{support}</div><p class="alpha-notice">{c["alpha_note"]}</p></section>
<section class="section faq wrap"><h2>{c["faq_title"]}</h2><div>{faqs}</div><a class="text-link" href="{guide}#evidence-current-limitations">{c["safety"]} →</a></section>
</main>
<footer class="site-footer"><div class="wrap footer-inner"><div><a class="brand" href="{base or "./"}"><img src="{assets}favicon.svg" width="25" height="25" alt="">mc-server-tui</a><p>{c["footer"]}</p></div><div class="footer-links"><a href="{guide}">{c["guide"]}</a><a href="{REPO}">{c["github"]}</a><a href="{REPO}/releases/tag/v{VERSION}">v{VERSION} ↗</a></div><p class="fine-print">MIT OR Apache-2.0 · Minecraft is a trademark of Mojang Synergies AB. Not affiliated with Mojang, Microsoft, PaperMC, Modrinth or Fabric.</p></div></footer>
</body></html>
'''


PAGES = (("en", ROOT / "site/index.html"), ("zh-CN", ROOT / "site/zh-CN/index.html"))


def main():
    for lang, output in PAGES:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(render_page(lang))
        print(f"wrote {output.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
