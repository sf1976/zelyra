(() => {
  try {
    if (localStorage.getItem("zelyra-editor-theme") === "light") {
      document.documentElement.dataset.theme = "light";
    }
  } catch {
    // Keep the dark default if the browser blocks local storage.
  }

  const initializeStudio = () => {
    const caption = document.getElementById("studio-splash-caption");
    const splash = document.getElementById("studio-splash");
    const fileCount = document.getElementById("file-count");
    if (!splash) return;

    try {
      const language = localStorage.getItem("zelyra-editor-language") === "en" ? "en" : "de";
      if (caption) caption.textContent = caption.dataset[language] || caption.textContent;
      splash.setAttribute("aria-label", language === "en" ? "Starting Zelyra Studio" : "Zelyra Studio wird gestartet");
    } catch {
      // Keep the German fallback text if local storage is unavailable.
    }

    const startedAt = performance.now();
    let finished = false;
    let observer;
    const finish = () => {
      if (finished) return;
      finished = true;
      splash.classList.add("is-hidden");
      splash.setAttribute("aria-hidden", "true");
      window.setTimeout(() => splash.remove(), 400);
      observer?.disconnect();
    };
    const scheduleFinish = () => {
      if (/^\d+ (Dateien|files)$/.test(fileCount.textContent.trim())) {
        const delay = Math.max(0, 520 - (performance.now() - startedAt));
        window.setTimeout(finish, delay);
      }
    };
    if (fileCount) {
      observer = new MutationObserver(scheduleFinish);
      observer.observe(fileCount, { childList: true, characterData: true, subtree: true });
      scheduleFinish();
    }
    window.setTimeout(finish, 2600);
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initializeStudio, { once: true });
  } else {
    initializeStudio();
  }
})();
