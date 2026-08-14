#[test]
fn packaged_release_documents_match_the_crate_version() {
    let version = env!("CARGO_PKG_VERSION");
    let components = version.split('.').collect::<Vec<_>>();
    assert_eq!(components.len(), 3, "release version must be X.Y.Z");
    assert!(
        components.iter().all(|component| !component.is_empty()
            && component.chars().all(|value| value.is_ascii_digit())),
        "release version must contain only numeric components"
    );

    let supported_line = format!("{}.{}.x", components[0], components[1]);
    let tag = format!("v{version}");
    let x64_zip = format!("iroha-zip-{version}-windows-x64.zip");
    let arm64_zip = format!("iroha-zip-{version}-windows-arm64.zip");

    let security = include_str!("../SECURITY.md");
    for expected in [
        format!("iroha-zip {supported_line} は"),
        format!("最新の `main` と `{supported_line}` を対象とします。"),
        format!("iroha-zip {supported_line} is"),
        format!("The latest `main` branch and `{supported_line}` are supported."),
    ] {
        assert!(
            security.contains(&expected),
            "SECURITY.md is missing the current support marker: {expected}"
        );
    }

    let readme_ja = include_str!("../README.md");
    let readme_en = include_str!("../README.en.md");
    assert!(readme_ja.contains(&tag), "README.md is missing {tag}");
    assert!(readme_en.contains(&tag), "README.en.md is missing {tag}");
    assert!(readme_ja.contains(&x64_zip));
    assert!(readme_en.contains(&x64_zip));
    for marker in [
        "\n## ",
        "\n### ",
        "\n- ",
        "\n|",
        "```",
        "](",
        "```powershell",
        "```text",
    ] {
        assert_eq!(
            readme_ja.matches(marker).count(),
            readme_en.matches(marker).count(),
            "bilingual README structure differs for marker {marker:?}"
        );
    }

    let release_notes = include_str!("../docs/RELEASE_NOTES.md");
    assert!(release_notes.starts_with(&format!("# iroha-zip {version}\n")));
    assert!(release_notes.contains(&x64_zip));
    assert!(release_notes.contains(&arm64_zip));
    assert_eq!(release_notes.matches("## 日本語").count(), 1);
    assert_eq!(release_notes.matches("## English").count(), 1);

    let verification = include_str!("../docs/RELEASE_VERIFICATION.md");
    assert!(verification.contains(&format!("Version `{version}` contains exactly 11 assets.")));
    assert!(verification.contains(&format!("refs/tags/{tag}")));
    assert!(verification.contains(&arm64_zip));

    let unsigned = include_str!("../docs/UNSIGNED_RELEASE.md");
    assert!(unsigned.contains(&x64_zip));
    assert!(unsigned.contains(&arm64_zip));

    let arm64 = include_str!("../docs/ARM64.md");
    assert!(arm64.contains(&arm64_zip));

    let updater = include_str!("../docs/UPDATER.md");
    assert!(updater.contains(&tag));
}

#[test]
fn bilingual_pages_match_the_crate_version_and_topology() {
    let version = env!("CARGO_PKG_VERSION");
    let tag = format!("v{version}");
    let x64_zip = format!("iroha-zip-{version}-windows-x64.zip");
    let root = include_str!("../site/index.html");
    let japanese_page = include_str!("../site/ja/index.html");
    let english_page = include_str!("../site/en/index.html");
    let not_found_page = include_str!("../site/404.html");
    let site_script = include_str!("../site/assets/site.js");
    let site_styles = include_str!("../site/assets/styles.css");
    let favicon = include_str!("../site/assets/favicon.svg");
    let robots = include_str!("../site/robots.txt");
    let sitemap = include_str!("../site/sitemap.xml");

    for (language, page) in [("ja", japanese_page), ("en", english_page)] {
        assert!(page.contains(&format!("<html lang=\"{language}\">")));
        assert!(page.contains(&tag), "{language} page is missing {tag}");
        assert!(
            page.contains(&x64_zip),
            "{language} page is missing {x64_zip}"
        );
        assert_eq!(page.matches("data-download-url=\"x64\"").count(), 2);
        assert_eq!(page.matches("data-download-url=\"arm64\"").count(), 2);
        assert_eq!(page.matches("data-release-url").count(), 2);
        assert!(page.contains("class=\"skip-link\" href=\"#main\""));
        assert!(page.contains("<main id=\"main\">"));
        assert!(page.contains("http-equiv=\"Content-Security-Policy\""));
        assert!(page.contains("connect-src 'self' https://api.github.com;"));
        assert!(page.contains("style-src 'self'; script-src 'self';"));
        assert!(page.contains("name=\"referrer\" content=\"strict-origin-when-cross-origin\""));
        assert!(page.contains("hreflang=\"x-default\""));
        assert!(page.contains("rel=\"icon\" href=\"../assets/favicon.svg\""));

        for section in ["how", "setup", "formats", "security", "usage", "status"] {
            assert_eq!(
                page.matches(&format!("<section class=\"section\" id=\"{section}\">"))
                    .count(),
                1,
                "{language} page must contain section #{section} exactly once"
            );
        }
    }

    for marker in [
        "<section ",
        "<h2>",
        "<h3>",
        "<article ",
        "<a ",
        "<li>",
        "<code>",
        "data-download-url=",
        "data-release-url",
    ] {
        assert_eq!(
            japanese_page.matches(marker).count(),
            english_page.matches(marker).count(),
            "bilingual Pages structure differs for marker {marker:?}"
        );
    }

    assert!(root.contains("data-page=\"language-gate\""));
    assert!(root.contains("href=\"ja/\" data-language-choice=\"ja\""));
    assert!(root.contains("href=\"en/\" data-language-choice=\"en\""));
    assert!(root.contains("http-equiv=\"Content-Security-Policy\""));
    assert!(root.contains("connect-src 'self' https://api.github.com;"));
    assert!(root.contains("style-src 'self'; script-src 'self';"));
    assert!(root.contains("hreflang=\"x-default\""));
    assert!(root.contains("rel=\"icon\" href=\"assets/favicon.svg\""));
    assert!(japanese_page.contains("rel=\"alternate\" hreflang=\"en\""));
    assert!(english_page.contains("rel=\"alternate\" hreflang=\"ja\""));
    assert!(site_script.contains(&format!("const fallbackVersion = \"{tag}\";")));
    assert!(site_script.contains("/-windows-x64\\.zip$/i"));
    assert!(site_script.contains("/-windows-arm64\\.zip$/i"));
    assert!(site_styles.contains("--on-accent: #ffffff;"));
    assert!(site_styles.contains("--on-accent: #0d131b;"));
    assert!(site_styles.contains("--footer: #17202a;"));
    assert!(site_styles.contains("--footer: #070b10;"));
    assert_eq!(site_styles.matches("color: var(--on-accent);").count(), 3);
    assert_eq!(site_styles.matches("background: var(--footer);").count(), 1);
    assert!(not_found_page.contains("name=\"robots\" content=\"noindex\""));
    for path in [
        "/iroha-zip/assets/favicon.svg",
        "/iroha-zip/assets/styles.css",
        "/iroha-zip/ja/",
        "/iroha-zip/en/",
    ] {
        assert!(not_found_page.contains(path));
    }
    assert!(favicon.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(robots.contains("Sitemap: https://hjosugi.github.io/iroha-zip/sitemap.xml"));
    assert_eq!(sitemap.matches("<url>").count(), 3);
    assert_eq!(sitemap.matches("hreflang=\"ja\"").count(), 3);
    assert_eq!(sitemap.matches("hreflang=\"en\"").count(), 3);
    assert_eq!(sitemap.matches("hreflang=\"x-default\"").count(), 3);
}
