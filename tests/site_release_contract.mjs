import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const repository = "hjosugi/iroha-zip";
const tag = "v0.5.3";
const version = tag.slice(1);
const releasePage = `https://github.com/${repository}/releases/latest`;
const script = fs.readFileSync(
  new URL("../site/assets/site.js", import.meta.url),
  "utf8",
);

const expectedAssetNames = [
  `iroha-zip-${version}-windows-arm64.exe`,
  `iroha-zip-${version}-windows-arm64.zip`,
  `iroha-zip-${version}-windows-arm64.zip.sha256`,
  `iroha-zip-${version}-windows-x64.exe`,
  `iroha-zip-${version}-windows-x64.zip`,
  `iroha-zip-${version}-windows-x64.zip.sha256`,
  `iroha-zip-settings-${version}-windows-arm64.exe`,
  `iroha-zip-settings-${version}-windows-x64.exe`,
  `iroha-zip-shell-${version}-windows-arm64.exe`,
  `iroha-zip-shell-${version}-windows-x64.exe`,
  "SHA256SUMS.txt",
];

const validRelease = () => ({
  tag_name: tag,
  draft: false,
  prerelease: false,
  immutable: true,
  published_at: "2026-08-14T08:50:20Z",
  assets: expectedAssetNames.map((name) => ({
    name,
    state: "uploaded",
    browser_download_url: `https://github.com/${repository}/releases/download/${tag}/${name}`,
  })),
});

const node = (dataset = {}) => ({
  dataset,
  href: "",
  textContent: "",
  listeners: new Map(),
  addEventListener(type, listener) {
    this.listeners.set(type, listener);
  },
});

async function exercise(release, { page = "", preferredLanguage = null } = {}) {
  const languageChoices = [node({ languageChoice: "ja" }), node({ languageChoice: "en" })];
  const selectors = new Map([
    ["[data-language-choice]", languageChoices],
    ["[data-release-url]", [node(), node()]],
    ['[data-download-url="x64"]', [node(), node()]],
    ['[data-download-url="arm64"]', [node(), node()]],
    ["[data-release-version]", [node()]],
    ["[data-release-date]", [node()]],
    ['[data-download-label="x64"]', [node()]],
    ['[data-download-label="arm64"]', [node()]],
  ]);
  const storage = new Map();
  if (preferredLanguage !== null) {
    storage.set("iroha-zip-language", preferredLanguage);
  }
  const redirects = [];
  let fetchCount = 0;
  const context = {
    document: {
      body: { dataset: { page } },
      documentElement: { lang: "en" },
      querySelectorAll(selector) {
        return selectors.get(selector) ?? [];
      },
    },
    window: {
      location: {
        replace(value) {
          redirects.push(value);
        },
      },
    },
    localStorage: {
      getItem(key) {
        return storage.get(key) ?? null;
      },
      setItem(key, value) {
        storage.set(key, value);
      },
    },
    async fetch() {
      fetchCount += 1;
      return { ok: true, status: 200, async json() { return release; } };
    },
    Intl,
    Date,
    Map,
    Array,
    Error,
    setTimeout,
    clearTimeout,
  };

  vm.runInNewContext(script, context);
  await new Promise((resolve) => setTimeout(resolve, 10));
  return { fetchCount, languageChoices, redirects, selectors, storage };
}

const accepted = await exercise(validRelease());
assert.equal(accepted.selectors.get("[data-release-version]")[0].textContent, tag);
assert.equal(
  accepted.selectors.get('[data-download-url="x64"]')[0].href,
  `https://github.com/${repository}/releases/download/${tag}/iroha-zip-${version}-windows-x64.zip`,
);
assert.equal(
  accepted.selectors.get('[data-download-url="arm64"]')[0].href,
  `https://github.com/${repository}/releases/download/${tag}/iroha-zip-${version}-windows-arm64.zip`,
);
assert.equal(
  accepted.selectors.get("[data-release-url]")[0].href,
  `https://github.com/${repository}/releases/tag/${tag}`,
);

const rejectedReleases = [
  { ...validRelease(), immutable: false },
  { ...validRelease(), draft: true },
  { ...validRelease(), prerelease: true },
  { ...validRelease(), tag_name: "v0.5.3-rc.1" },
  { ...validRelease(), assets: validRelease().assets.slice(1) },
  {
    ...validRelease(),
    assets: [...validRelease().assets, { name: "unexpected.bin", state: "uploaded" }],
  },
  {
    ...validRelease(),
    assets: validRelease().assets.map((asset, index) =>
      index === 0 ? { ...asset, state: "new" } : asset
    ),
  },
  {
    ...validRelease(),
    assets: validRelease().assets.map((asset, index) =>
      index === 0 ? { ...asset, browser_download_url: "https://example.invalid/file" } : asset
    ),
  },
];

for (const release of rejectedReleases) {
  const rejected = await exercise(release);
  assert.equal(rejected.selectors.get("[data-release-version]")[0].textContent, tag);
  assert.equal(rejected.selectors.get('[data-download-url="x64"]')[0].href, releasePage);
  assert.equal(rejected.selectors.get('[data-download-url="arm64"]')[0].href, releasePage);
}

const languageGate = await exercise(validRelease(), {
  page: "language-gate",
  preferredLanguage: "ja",
});
assert.deepEqual(languageGate.redirects, ["ja/"]);
assert.equal(languageGate.fetchCount, 0);
languageGate.languageChoices[1].listeners.get("click")();
assert.equal(languageGate.storage.get("iroha-zip-language"), "en");

console.log("Bilingual Pages release and language contracts passed.");
