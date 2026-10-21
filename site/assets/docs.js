(() => {
    "use strict";

    const progressBar = document.querySelector("#docs-progress-bar");
    const article = document.querySelector(".docs-article");
    const sections = [...document.querySelectorAll(".doc-section")];
    const sideLinks = [...document.querySelectorAll("[data-doc-link]")];
    const tocLinks = [...document.querySelectorAll("[data-toc-link]")];
    const filter = document.querySelector("#docs-nav-filter");
    const empty = document.querySelector("#docs-nav-empty");
    const mobileMenu = document.querySelector(".docs-mobile-menu");

    function updateReadingPosition() {
        if (progressBar && article) {
            const start = article.offsetTop;
            const distance = Math.max(
                article.offsetHeight - window.innerHeight,
                1,
            );
            const progress = Math.min(
                Math.max((window.scrollY - start) / distance, 0),
                1,
            );
            progressBar.style.width = `${progress * 100}%`;
        }

        if (!sections.length) return;
        const marker = window.scrollY + Math.min(window.innerHeight * 0.3, 260);
        let active = sections[0];
        for (const section of sections) {
            if (section.offsetTop <= marker) active = section;
        }
        for (const link of [...sideLinks, ...tocLinks]) {
            const match = link.getAttribute("href") === `#${active.id}`;
            link.classList.toggle("is-active", match);
            if (match) link.setAttribute("aria-current", "location");
            else link.removeAttribute("aria-current");
        }
    }

    let scrollFrame = 0;
    function requestUpdate() {
        if (scrollFrame) return;
        scrollFrame = window.requestAnimationFrame(() => {
            scrollFrame = 0;
            updateReadingPosition();
        });
    }

    updateReadingPosition();
    window.addEventListener("scroll", requestUpdate, { passive: true });
    window.addEventListener("resize", requestUpdate);

    if (filter) {
        filter.addEventListener("input", () => {
            const query = filter.value.trim().toLocaleLowerCase();
            let visibleCount = 0;
            document.querySelectorAll("[data-doc-group]").forEach((group) => {
                let groupCount = 0;
                group.querySelectorAll("[data-doc-link]").forEach((link) => {
                    const section = document.getElementById(link.hash.slice(1));
                    const searchable =
                        `${link.textContent} ${section?.textContent || ""}`.toLocaleLowerCase();
                    const visible = !query || searchable.includes(query);
                    link.hidden = !visible;
                    if (visible) {
                        groupCount += 1;
                        visibleCount += 1;
                    }
                });
                group.hidden = groupCount === 0;
            });
            if (empty) empty.hidden = visibleCount !== 0;
        });
    }

    if (mobileMenu) {
        mobileMenu.querySelectorAll("a").forEach((link) => {
            link.addEventListener("click", () => {
                mobileMenu.open = false;
            });
        });
    }
})();
