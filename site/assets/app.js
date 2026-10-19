(() => {
  "use strict";

  const themes = {
    claude: {
      label: "Claude",
      accent: "#d97757",
      rgb: "217, 119, 87",
      image: "assets/shell-session-claude-80x24.png",
    },
    deepseek: {
      label: "DeepSeek",
      accent: "#4d6bfe",
      rgb: "77, 107, 254",
      image: "assets/shell-session-deepseek-80x24.png",
    },
    codex: {
      label: "Codex",
      accent: "#10a37f",
      rgb: "16, 163, 127",
      image: "assets/shell-session-codex-80x24.png",
    },
    kimi: {
      label: "Kimi",
      accent: "#a78bfa",
      rgb: "167, 139, 250",
      image: "assets/shell-session-kimi-80x24.png",
    },
    pcl2: {
      label: "PCL2",
      accent: "#56b8e8",
      rgb: "86, 184, 232",
      image: "assets/shell-session-pcl2-80x24.png",
    },
    mono: {
      label: "Mono",
      accent: "#d8d8d8",
      rgb: "216, 216, 216",
      image: "assets/shell-session-mono-80x24.png",
    },
    ocean: {
      label: "Ocean",
      accent: "#6cb6c9",
      rgb: "108, 182, 201",
      image: "assets/shell-session-ocean-80x24.png",
    },
  };

  const root = document.documentElement;
  const heroShot = document.querySelector("#hero-shot");
  const themeShot = document.querySelector("#theme-shot");
  const heroLabel = document.querySelector("#hero-theme-label");
  const themeLabel = document.querySelector("#theme-preview-label");
  const controls = [...document.querySelectorAll("[data-set-theme]")];
  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function transitionImage(image, source, label) {
    if (!image) return;
    image.classList.add("is-switching");
    const next = new Image();
    next.onload = () => {
      image.src = source;
      image.alt = `mc-server-tui ${label} theme screenshot`;
      requestAnimationFrame(() => image.classList.remove("is-switching"));
    };
    next.onerror = () => image.classList.remove("is-switching");
    next.src = source;
  }

  function applyTheme(id, persist = true) {
    const theme = themes[id] ? themes[id] : themes.claude;
    const selected = themes[id] ? id : "claude";

    root.style.setProperty("--accent", theme.accent);
    root.style.setProperty("--accent-rgb", theme.rgb);
    root.style.setProperty("--accent-soft", `rgba(${theme.rgb}, 0.12)`);
    root.dataset.theme = selected;

    if (heroLabel) heroLabel.textContent = theme.label.toUpperCase();
    if (themeLabel) themeLabel.textContent = theme.label.toUpperCase();
    transitionImage(heroShot, theme.image, theme.label);
    transitionImage(themeShot, theme.image, theme.label);

    controls.forEach((control) => {
      const active = control.dataset.setTheme === selected;
      control.classList.toggle("is-active", active);
      control.setAttribute("aria-pressed", String(active));
    });

    if (persist) {
      try {
        localStorage.setItem("mcst-theme", selected);
      } catch {
        /* localStorage can be unavailable in private contexts. */
      }
    }
  }

  controls.forEach((control) => {
    control.addEventListener("click", () => applyTheme(control.dataset.setTheme));
  });

  let savedTheme = "claude";
  try {
    const stored = localStorage.getItem("mcst-theme");
    if (stored && themes[stored]) savedTheme = stored;
  } catch {
    /* Keep the default. */
  }
  applyTheme(savedTheme, false);

  Object.values(themes).forEach((theme) => {
    const image = new Image();
    image.src = theme.image;
  });

  const copyButton = document.querySelector("#copy-install");
  const installSnippet = document.querySelector("#install-snippet");
  if (copyButton && installSnippet) {
    copyButton.addEventListener("click", async () => {
      const text = installSnippet.textContent
        .split("\n")
        .map((line) => line.replace(/^\$\s?/, ""))
        .join("\n")
        .trim();
      let copied = false;
      try {
        await navigator.clipboard.writeText(text);
        copied = true;
      } catch {
        const helper = document.createElement("textarea");
        helper.value = text;
        helper.setAttribute("readonly", "");
        helper.style.position = "fixed";
        helper.style.opacity = "0";
        document.body.appendChild(helper);
        helper.select();
        copied = document.execCommand("copy");
        helper.remove();
      }
      copyButton.textContent = copied ? "Copied" : "Select";
      copyButton.classList.toggle("is-copied", copied);
      window.setTimeout(() => {
        copyButton.textContent = "Copy";
        copyButton.classList.remove("is-copied");
      }, 1800);
    });
  }

  const revealItems = [...document.querySelectorAll("[data-reveal]")];
  if (!prefersReducedMotion && "IntersectionObserver" in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    );
    revealItems.forEach((item) => observer.observe(item));
  } else {
    revealItems.forEach((item) => item.classList.add("is-visible"));
  }

  const header = document.querySelector("#site-header");
  const navLinks = [...document.querySelectorAll(".nav a[href^='#']")];
  const sections = navLinks
    .map((link) => document.querySelector(link.getAttribute("href")))
    .filter(Boolean);

  function updateHeader() {
    if (header) header.classList.toggle("is-scrolled", window.scrollY > 10);
    if (!sections.length) return;
    const offset = window.innerHeight * 0.3;
    let active = sections[0];
    sections.forEach((section) => {
      if (section.getBoundingClientRect().top <= offset) active = section;
    });
    navLinks.forEach((link) => {
      link.classList.toggle("is-active", link.getAttribute("href") === `#${active.id}`);
    });
  }

  updateHeader();
  window.addEventListener("scroll", updateHeader, { passive: true });

  const year = document.querySelector("#year");
  if (year) year.textContent = String(new Date().getFullYear());
})();
