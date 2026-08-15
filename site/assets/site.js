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
  const fallbackVersion = "v0.6.2";
  const stableTagPattern = /^v(\d+\.\d+\.\d+)$/;

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
  setHref('[data-download-url="x64"]', releasePage);
  setHref('[data-download-url="arm64"]', releasePage);
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
      const tag = typeof release.tag_name === "string" ? release.tag_name : "";
      const versionMatch = stableTagPattern.exec(tag);
      if (
        !versionMatch ||
        release.draft !== false ||
        release.prerelease !== false ||
        release.immutable !== true
      ) {
        throw new Error("Latest release does not satisfy the stable immutable contract");
      }

      const versionNumber = versionMatch[1];
      const expectedAssetNames = [
        `iroha-zip-${versionNumber}-windows-arm64.exe`,
        `iroha-zip-${versionNumber}-windows-arm64.zip`,
        `iroha-zip-${versionNumber}-windows-arm64.zip.sha256`,
        `iroha-zip-${versionNumber}-windows-x64.exe`,
        `iroha-zip-${versionNumber}-windows-x64.zip`,
        `iroha-zip-${versionNumber}-windows-x64.zip.sha256`,
        `iroha-zip-settings-${versionNumber}-windows-arm64.exe`,
        `iroha-zip-settings-${versionNumber}-windows-x64.exe`,
        `iroha-zip-shell-${versionNumber}-windows-arm64.exe`,
        `iroha-zip-shell-${versionNumber}-windows-x64.exe`,
        "SHA256SUMS.txt",
      ];
      const assets = Array.isArray(release.assets) ? release.assets : [];
      const assetByName = new Map(assets.map((asset) => [asset.name, asset]));
      if (
        assets.length !== expectedAssetNames.length ||
        expectedAssetNames.some((name) => {
          const asset = assetByName.get(name);
          const expectedUrl = `https://github.com/${repository}/releases/download/${tag}/${name}`;
          return asset?.state !== "uploaded" || asset.browser_download_url !== expectedUrl;
        })
      ) {
        throw new Error("Latest release does not have the exact uploaded asset inventory");
      }

      const x64Zip = assetByName.get(`iroha-zip-${versionNumber}-windows-x64.zip`);
      const arm64Zip = assetByName.get(`iroha-zip-${versionNumber}-windows-arm64.zip`);
      const releaseUrl = `https://github.com/${repository}/releases/tag/${tag}`;
      const date = release.published_at
        ? new Intl.DateTimeFormat(locale, {
            year: "numeric",
            month: "short",
            day: "numeric",
          }).format(new Date(release.published_at))
        : "";

      setText("[data-release-version]", tag);
      setText("[data-release-date]", date);
      setHref("[data-release-url]", releaseUrl);
      setHref('[data-download-url="x64"]', x64Zip.browser_download_url);
      setText('[data-download-label="x64"]', x64Zip.name);
      setHref('[data-download-url="arm64"]', arm64Zip.browser_download_url);
      setText('[data-download-label="arm64"]', arm64Zip.name);
    })
    .catch(() => {
      // Static fallbacks deliberately remain usable during API limits or outages.
    });
})();
