try {
  if (localStorage.getItem("zelyra-editor-theme") === "light") {
    document.documentElement.dataset.theme = "light";
  }
} catch {
  // Keep the dark default if the browser blocks local storage.
}
