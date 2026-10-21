(() => {
    "use strict";
    const themes = new Map([
        ["claude", "Claude"],
        ["deepseek", "DeepSeek"],
        ["codex", "Codex"],
        ["kimi", "Kimi"],
        ["pcl2", "PCL2"],
        ["mono", "Mono"],
        ["ocean", "Ocean"],
    ]);
    const image = document.querySelector("#theme-shot");
    const label = document.querySelector("#theme-preview-label");
    const controls = [...document.querySelectorAll("[data-set-theme]")];
    const assets = document.body.dataset.assets || "assets/";
    function applyTheme(id, persist = true) {
        if (!themes.has(id) || !image) return;
        const name = themes.get(id);
        image.src = `${assets}session-${id}-0.2.1.png`;
        image.alt = `${name} · mc-server-tui`;
        if (label) label.textContent = name;
        controls.forEach((button) =>
            button.setAttribute(
                "aria-pressed",
                String(button.dataset.setTheme === id),
            ),
        );
        if (persist) {
            try {
                localStorage.setItem("mcst-preview-theme", id);
            } catch {
                /* Optional preference. */
            }
        }
    }
    controls.forEach((button) =>
        button.addEventListener("click", () =>
            applyTheme(button.dataset.setTheme),
        ),
    );
    try {
        applyTheme(
            localStorage.getItem("mcst-preview-theme") || "claude",
            false,
        );
    } catch {
        /* Default stays visible. */
    }

    const tabs = [...document.querySelectorAll("[data-view]")];
    function activateTab(selected) {
        tabs.forEach((tab) => {
            const active = tab === selected;
            tab.setAttribute("aria-selected", String(active));
            tab.tabIndex = active ? 0 : -1;
            const panel = document.getElementById(
                tab.getAttribute("aria-controls"),
            );
            if (panel) panel.hidden = !active;
        });
    }
    tabs.forEach((tab, index) => {
        tab.addEventListener("click", () => activateTab(tab));
        tab.addEventListener("keydown", (event) => {
            const target =
                event.key === "ArrowRight"
                    ? tabs[(index + 1) % tabs.length]
                    : event.key === "ArrowLeft"
                      ? tabs[(index + tabs.length - 1) % tabs.length]
                      : event.key === "Home"
                        ? tabs[0]
                        : event.key === "End"
                          ? tabs.at(-1)
                          : null;
            if (target) {
                event.preventDefault();
                activateTab(target);
                target.focus();
            }
        });
    });

    document
        .querySelectorAll("[data-copy-target], .docs-code-copy")
        .forEach((button) => {
            let resetTimer;
            const original = button.textContent;
            button.setAttribute("aria-live", "polite");
            button.addEventListener("click", async () => {
                const code = button.dataset.copyTarget
                    ? document.getElementById(button.dataset.copyTarget)
                    : button.closest(".docs-code")?.querySelector("code");
                if (!code) return;
                try {
                    await navigator.clipboard.writeText(code.textContent);
                    button.textContent =
                        button.dataset.copied ||
                        button.dataset.copiedLabel ||
                        "Copied";
                } catch {
                    const selection = window.getSelection();
                    const range = document.createRange();
                    range.selectNodeContents(code);
                    selection?.removeAllRanges();
                    selection?.addRange(range);
                    button.textContent =
                        button.dataset.failed ||
                        button.dataset.failedLabel ||
                        "Select commands";
                }
                window.clearTimeout(resetTimer);
                resetTimer = window.setTimeout(() => {
                    button.textContent = original;
                }, 2200);
            });
        });
})();
