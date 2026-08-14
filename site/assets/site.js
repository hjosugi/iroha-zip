(() => {
  "use strict";

  const repository = "hjosugi/iroha-zip";
  const releasePage = `https://github.com/${repository}/releases/latest`;
  const preferredLanguageKey = "iroha-zip-language";

  document.querySelectorAll("[data-language-choice]").forEach((link) => {
    link.addEventListener("click", () => {
      try {
        localStorage.setItem(preferredLanguageKey, link.dataset.languageChoice);
      } catch (_) {
        // Storage is optional; navigation remains a normal link.
      }
    });
  });

  if (document.body.dataset.page === "language-gate") {
    try {
      const preferred = localStorage.getItem(preferredLanguageKey);
      if (preferred === "ja" || preferred === "en") {
        window.location.replace(`${preferred}/`);
      }
    } catch (_) {
      // Keep the explicit language selector visible when storage is unavailable.
    }
    return;
  }

  const locale = document.documentElement.lang === "ja" ? "ja-JP" : "en-US";
  const fallbackVersion = "v0.4.1";

  const setText = (selector, value) => {
    document.querySelectorAll(selector).forEach((node) => {
      node.textContent = value;
    });
  };

  const setHref = (selector, value) => {
    document.querySelectorAll(selector).forEach((node) => {
      node.href = value;
    });
  };

  setHref("[data-release-url]", releasePage);
  setHref("[data-download-url]", releasePage);
  setText("[data-release-version]", fallbackVersion);

  fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  })
    .then((response) => {
      if (!response.ok) {
        throw new Error(`GitHub API returned ${response.status}`);
      }
      return response.json();
    })
    .then((release) => {
      const version = release.tag_name || fallbackVersion;
      const zip = Array.isArray(release.assets)
        ? release.assets.find((asset) => /-windows-x64\.zip$/i.test(asset.name))
        : null;
      const date = release.published_at
        ? new Intl.DateTimeFormat(locale, {
            year: "numeric",
            month: "short",
            day: "numeric",
          }).format(new Date(release.published_at))
        : "";

      setText("[data-release-version]", version);
      setText("[data-release-date]", date);
      setHref("[data-release-url]", release.html_url || releasePage);
      if (zip) {
        setHref("[data-download-url]", zip.browser_download_url);
        setText("[data-download-label]", zip.name);
      }
    })
    .catch(() => {
      // Static fallbacks deliberately remain usable during API limits or outages.
    });
})();
