// Shared website behaviour: theme control, mobile navigation, copy buttons,
// and the "Run in Playground" handoff. No Aura semantics live here.
//
// The documentation Table of Contents highlight uses an IntersectionObserver.
// Playground behaviour lives in the dedicated playground bundle.

const root = document.documentElement;
const STORAGE_KEY = "aura-theme";

/* ---------------------------------------------------------------- theme */
function currentTheme() {
  const stored = safeGet(STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme(theme) {
  root.setAttribute("data-theme", theme);
  safeSet(STORAGE_KEY, theme);
  for (const el of document.querySelectorAll("[data-theme-icon-light]")) {
    el.hidden = theme !== "light";
  }
  for (const el of document.querySelectorAll("[data-theme-icon-dark]")) {
    el.hidden = theme !== "dark";
  }
  for (const btn of document.querySelectorAll("[data-theme-toggle]")) {
    btn.setAttribute(
      "aria-label",
      theme === "dark" ? "Switch to light theme" : "Switch to dark theme",
    );
  }
}

safeInit();
for (const btn of document.querySelectorAll("[data-theme-toggle]")) {
  btn.addEventListener("click", () => {
    applyTheme(currentTheme() === "dark" ? "light" : "dark");
  });
}
// Follow OS changes until the user makes an explicit choice.
window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", () => {
    if (!safeGet(STORAGE_KEY)) applyTheme(currentTheme());
  });

function safeInit() {
  const t = safeGet(STORAGE_KEY);
  applyTheme(t === "light" || t === "dark" ? t : currentTheme());
}

function safeGet(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function safeSet(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage disabled; theme still applies for this view */
  }
}

/* ------------------------------------------------------------- mobile nav */
for (const toggle of document.querySelectorAll("[data-nav-toggle]")) {
  const nav = document.getElementById(toggle.getAttribute("aria-controls"));
  const openIcon = toggle.querySelector("[data-nav-icon-open]");
  const closeIcon = toggle.querySelector("[data-nav-icon-close]");
  if (!nav) continue;
  toggle.addEventListener("click", () => {
    const open = nav.getAttribute("data-open") === "true";
    nav.setAttribute("data-open", String(!open));
    toggle.setAttribute("aria-expanded", String(!open));
    if (openIcon) openIcon.hidden = !open;
    if (closeIcon) closeIcon.hidden = open;
  });
  // Close on navigation and on Escape.
  nav.addEventListener("click", (e) => {
    if (e.target.closest("a")) {
      nav.setAttribute("data-open", "false");
      toggle.setAttribute("aria-expanded", "false");
      if (openIcon) openIcon.hidden = false;
      if (closeIcon) closeIcon.hidden = true;
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && nav.getAttribute("data-open") === "true") {
      nav.setAttribute("data-open", "false");
      toggle.setAttribute("aria-expanded", "false");
      toggle.focus();
    }
  });
}

/* ----------------------------------------------------------------- copy */
for (const btn of document.querySelectorAll("[data-copy]")) {
  btn.addEventListener("click", async () => {
    const block = btn.closest(".code-block");
    const explicit = btn.getAttribute("data-code");
    const text =
      explicit != null
        ? explicit
        : block
          ? block.querySelector("code").textContent
          : "";
    try {
      await navigator.clipboard.writeText(text);
      markCopied(btn);
    } catch {
      // Fallback for contexts without the async clipboard API.
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("aria-hidden", "true");
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.append(ta);
      ta.select();
      try {
        document.execCommand("copy");
        markCopied(btn);
      } finally {
        ta.remove();
      }
    }
  });
}

function markCopied(btn) {
  const label = btn.querySelector("span");
  const previous = label ? label.textContent : null;
  btn.setAttribute("data-copied", "true");
  if (label) label.textContent = "Copied";
  setTimeout(() => {
    btn.removeAttribute("data-copied");
    if (label && previous != null) label.textContent = previous;
  }, 1500);
}

/* --------------------------------------------------- run-in-playground */
// A "Run" button on an example stashes the source so the Playground can load
// it. The Playground reads `sessionStorage` on load; this never executes code.
for (const link of document.querySelectorAll("[data-run-example]")) {
  link.addEventListener("click", () => {
    const block = link.closest(".code-block");
    const btn = block && block.querySelector("[data-copy]");
    const code = btn ? btn.getAttribute("data-code") : null;
    if (code) {
      try {
        sessionStorage.setItem("aura-playground-source", code);
      } catch {
        /* ignore */
      }
    }
  });
}

/* --------------------------------------------------------- docs toc */
const tocLinks = [...document.querySelectorAll(".toc a[href^='#']")];
if (tocLinks.length) {
  const targets = tocLinks
    .map((a) => document.getElementById(decodeURIComponent(a.hash.slice(1))))
    .filter(Boolean);
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          for (const a of tocLinks) {
            a.setAttribute(
              "aria-current",
              String(a.getAttribute("href") === `#${entry.target.id}`),
            );
          }
        }
      }
    },
    { rootMargin: "-80px 0px -70% 0px", threshold: 0 },
  );
  for (const t of targets) observer.observe(t);
}

/* --------------------------------------------------- external links */
// Mark external links so styling and analytics-free behaviour is predictable.
for (const a of document.querySelectorAll("a[href^='http']")) {
  if (a.hostname && a.hostname !== window.location.hostname) {
    a.setAttribute("rel", "noopener");
  }
}
