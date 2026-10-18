(() => {
  "use strict";

  const themeKey = "zelyra-ui-theme";
  const densityKey = "zelyra-ui-density";
  const controllers = new WeakMap();
  const dialogOpeners = new WeakMap();

  function controllerFor(element) {
    let controller = controllers.get(element);
    if (!controller) {
      controller = new AbortController();
      controllers.set(element, controller);
    }
    return controller;
  }

  function bind(element, type, listener) {
    element.addEventListener(type, listener, { signal: controllerFor(element).signal });
  }

  function setTheme(theme, persist = true) {
    if (!["system", "light", "dark"].includes(theme)) theme = "system";
    document.documentElement.dataset.zTheme = theme;
    if (persist) {
      try { localStorage.setItem(themeKey, theme); } catch (_) { /* storage is optional */ }
    }
    const german = document.documentElement.lang.toLowerCase().startsWith("de");
    const themeNames = german ? { system: "System", light: "hell", dark: "dunkel" } : { system: "system", light: "light", dark: "dark" };
    document.querySelectorAll("[data-z-theme-toggle]").forEach((button) => {
      button.setAttribute("aria-pressed", String(theme !== "system"));
      button.setAttribute("aria-label", german
        ? `Farbschema aktiv: ${themeNames[theme]}`
        : `Color theme active: ${themeNames[theme]}`);
    });
  }

  function setDensity(density, persist = true) {
    if (!["comfortable", "compact"].includes(density)) density = "comfortable";
    if (density === "compact") document.documentElement.dataset.zDensity = "compact";
    else delete document.documentElement.dataset.zDensity;
    if (persist) {
      try { localStorage.setItem(densityKey, density); } catch (_) { /* storage is optional */ }
    }
    const german = document.documentElement.lang.toLowerCase().startsWith("de");
    document.querySelectorAll("[data-z-density-toggle]").forEach((button) => {
      button.setAttribute("aria-pressed", String(density === "compact"));
      button.setAttribute("aria-label", german
        ? `Darstellungsdichte: ${density === "compact" ? "kompakt" : "komfortabel"}`
        : `Display density: ${density}`);
    });
  }

  function setupThemeAndDensity(root) {
    if (root === document) {
      const theme = (() => { try { return localStorage.getItem(themeKey); } catch (_) { return null; } })();
      const density = (() => { try { return localStorage.getItem(densityKey); } catch (_) { return null; } })();
      setTheme(theme || "system", Boolean(theme));
      setDensity(density || "comfortable", Boolean(density));
    }

    root.querySelectorAll("[data-z-theme-toggle]").forEach((button) => {
      if (button.dataset.zReady) return;
      button.dataset.zReady = "true";
      button.hidden = false;
      bind(button, "click", () => {
        const current = document.documentElement.dataset.zTheme || "system";
        setTheme(current === "system" ? "dark" : current === "dark" ? "light" : "system");
      });
    });
    root.querySelectorAll("[data-z-density-toggle]").forEach((button) => {
      if (button.dataset.zReady) return;
      button.dataset.zReady = "true";
      button.hidden = false;
      bind(button, "click", () => {
        setDensity(document.documentElement.dataset.zDensity === "compact" ? "comfortable" : "compact");
      });
    });
  }

  function setupNavigation(root) {
    const toggles = Array.from(root.querySelectorAll("[data-z-mobile-nav-toggle]"));
    if (!toggles.length) return;

    const configure = () => {
      const compact = window.matchMedia("(max-width: 600px)").matches;
      for (const button of toggles) {
        const panel = document.getElementById(button.getAttribute("aria-controls"));
        if (!panel) continue;
        button.hidden = !compact;
        if (!compact) {
          delete panel.dataset.zCollapsed;
          button.setAttribute("aria-expanded", "true");
          button.dataset.zNavCompact = "false";
        } else if (button.dataset.zNavCompact !== "true") {
          panel.dataset.zCollapsed = "true";
          button.setAttribute("aria-expanded", "false");
          button.dataset.zNavCompact = "true";
        }
      }
    };

    for (const button of toggles) {
      if (button.dataset.zReady) continue;
      button.dataset.zReady = "true";
      bind(button, "click", () => {
        const panel = document.getElementById(button.getAttribute("aria-controls"));
        if (!panel) return;
        const collapsed = panel.dataset.zCollapsed !== "true";
        if (collapsed) panel.dataset.zCollapsed = "true";
        else delete panel.dataset.zCollapsed;
        button.setAttribute("aria-expanded", String(!collapsed));
      });
      bind(button, "keydown", (event) => {
        if (event.key !== "Escape") return;
        const panel = document.getElementById(button.getAttribute("aria-controls"));
        if (panel && panel.dataset.zCollapsed !== "true") {
          panel.dataset.zCollapsed = "true";
          button.setAttribute("aria-expanded", "false");
          button.focus();
        }
      });
    }
    configure();
    if (root === document && !window.__zelyraUiNavResizeBound) {
      window.__zelyraUiNavResizeBound = true;
      window.addEventListener("resize", configure, { passive: true });
    }
  }

  function setupDialogs(root) {
    root.querySelectorAll("[data-z-dialog-open]").forEach((button) => {
      if (button.dataset.zReady) return;
      button.dataset.zReady = "true";
      bind(button, "click", () => {
        const dialog = document.getElementById(button.getAttribute("data-z-dialog-open"));
        if (!(dialog instanceof HTMLDialogElement)) return;
        dialogOpeners.set(dialog, button);
        if (!dialog.open) dialog.showModal();
      });
    });

    root.querySelectorAll("dialog[data-z-dialog]").forEach((dialog) => {
      if (dialog.dataset.zReady) return;
      dialog.dataset.zReady = "true";
      dialog.dataset.zEnhanced = "true";
      // The open, non-modal markup is the no-JavaScript fallback. Once this
      // script is running, close it and only open it in response to an action.
      if (dialog.open) dialog.close();
      bind(dialog, "close", () => {
        const opener = dialogOpeners.get(dialog);
        if (opener?.isConnected && !opener.disabled) opener.focus();
        dialogOpeners.delete(dialog);
      });
      bind(dialog, "click", (event) => {
        if (event.target === dialog && dialog.dataset.zBackdropClose === "true") dialog.close();
      });
      bind(dialog, "keydown", (event) => {
        if (event.key !== "Tab" || !dialog.open) return;
        const focusable = Array.from(dialog.querySelectorAll(
          'a[href], area[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), iframe, object, embed, [contenteditable="true"], [tabindex]:not([tabindex="-1"])',
        )).filter((element) => element.getClientRects().length > 0);
        if (!focusable.length) {
          event.preventDefault();
          dialog.focus();
          return;
        }
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        const active = document.activeElement;
        if (!dialog.contains(active) || (!event.shiftKey && active === last)) {
          event.preventDefault();
          (event.shiftKey ? last : first).focus();
        } else if (event.shiftKey && active === first) {
          event.preventDefault();
          last.focus();
        }
      });
      dialog.querySelectorAll("[data-z-dialog-close]").forEach((button) => {
        if (button.hasAttribute("data-z-dialog-close-default") && button.textContent.trim() === "Schließen") {
          const german = document.documentElement.lang.toLowerCase().startsWith("de");
          if (!german) button.textContent = "Close";
        }
        bind(button, "click", () => dialog.close());
      });
    });
  }

  function setupTabs(root) {
    root.querySelectorAll("[data-z-tabs]").forEach((tabs) => {
      if (tabs.dataset.zReady) return;
      tabs.dataset.zReady = "true";
      const links = Array.from(tabs.querySelectorAll("[data-z-tab]"));
      if (!links.length) return;
      tabs.setAttribute("role", "tablist");
      const activate = (active, moveFocus = false) => {
        for (const link of links) {
          const selected = link === active;
          const panelId = link.getAttribute("href")?.slice(1);
          const panel = panelId ? document.getElementById(panelId) : null;
          link.setAttribute("role", "tab");
          link.setAttribute("aria-selected", String(selected));
          link.tabIndex = selected ? 0 : -1;
          if (panel) {
            panel.setAttribute("role", "tabpanel");
            panel.setAttribute("aria-labelledby", link.id || "");
            panel.hidden = !selected;
          }
          if (moveFocus && selected) link.focus();
        }
      };
      links.forEach((link, index) => {
        if (!link.id) link.id = `${tabs.id || "z-tabs"}-tab-${index + 1}`;
        bind(link, "click", (event) => {
          event.preventDefault();
          activate(link);
        });
        bind(link, "keydown", (event) => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
          event.preventDefault();
          const next = event.key === "Home" ? 0 : event.key === "End" ? links.length - 1 :
            (index + (event.key === "ArrowRight" ? 1 : -1) + links.length) % links.length;
          activate(links[next], true);
        });
      });
      const initial = links.find((link) => link.hash === location.hash) || links[0];
      activate(initial);
    });
  }

  function setupDropdowns(root) {
    root.querySelectorAll("details.z-dropdown").forEach((dropdown) => {
      if (dropdown.dataset.zReady) return;
      dropdown.dataset.zReady = "true";
      bind(dropdown, "keydown", (event) => {
        if (event.key === "Escape" && dropdown.open) {
          dropdown.open = false;
          dropdown.querySelector("summary")?.focus();
        }
      });
    });
  }

  function setupPrint(root) {
    root.querySelectorAll("[data-z-print]").forEach((button) => {
      if (button.dataset.zReady) return;
      button.dataset.zReady = "true";
      bind(button, "click", () => window.print());
    });
  }

  function initialize(root = document) {
    setupThemeAndDensity(root);
    setupNavigation(root);
    setupDialogs(root);
    setupTabs(root);
    setupDropdowns(root);
    setupPrint(root);
  }

  function destroy(root = document) {
    const selectors = [
      "[data-z-theme-toggle]", "[data-z-density-toggle]", "[data-z-mobile-nav-toggle]",
      "[data-z-dialog-open]", "dialog[data-z-dialog]", "[data-z-tabs]",
      "details.z-dropdown", "[data-z-print]", "[data-z-dialog-close]", "[data-z-tab]",
    ];
    const elements = new Set();
    if (root instanceof Element && root.matches(selectors.join(","))) elements.add(root);
    root.querySelectorAll(selectors.join(",")).forEach((element) => elements.add(element));
    for (const element of elements) {
      controllers.get(element)?.abort();
      controllers.delete(element);
      delete element.dataset.zReady;
    }
  }

  function toast(message, appearance = "default", duration = 5000) {
    const region = document.querySelector("[data-z-toast-region]") || (() => {
      const element = document.createElement("div");
      element.className = "z-toast-region";
      element.dataset.zToastRegion = "true";
      element.setAttribute("aria-live", "polite");
      element.setAttribute("aria-relevant", "additions text");
      document.body.append(element);
      return element;
    })();
    const item = document.createElement("div");
    item.className = `z-toast${["success", "warning", "danger"].includes(appearance) ? ` z-toast--${appearance}` : ""}`;
    item.setAttribute("role", appearance === "danger" ? "alert" : "status");
    item.textContent = String(message);
    region.append(item);
    if (Number.isFinite(duration) && duration > 0) window.setTimeout(() => item.remove(), duration);
    return item;
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => initialize(), { once: true });
  } else {
    initialize();
  }
  window.ZelyraUI = Object.freeze({ initialize, destroy, setTheme, setDensity, toast });
})();
