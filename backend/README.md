# Backend directory

`backend/libarchive/`には、利用者が信頼する`bsdtar.exe`と必要最小限のDLL、自動生成される`backend-manifest.tsv`、および`.iroha-zip-evidence/`のprovenance・SPDX SBOM・license inventoryを配置します。

ソース配布にはバイナリを含めません。

通常は`iroha-zip 設定`の「bundleを取り込む」または「MSYS2から取り込む」を使用してください。次のスクリプトは自動化用です。

```powershell
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64
```

または、用意済みの最小bundleを取り込みます。

```powershell
.\scripts\install-backend.ps1 `
  -SourceDirectory C:\path\to\bundle `
  -AllowUnsupportedSource
```

任意bundleは署名と取得元を検証できないため、設定画面では明示警告への確認、スクリプトでは`-AllowUnsupportedSource`が必須です。iroha-zipはmanifest、provenance、SBOM、license inventory間の不一致と、各inventoryにない余分なファイルを拒否します。詳細は[`docs/BACKEND_EVIDENCE.md`](../docs/BACKEND_EVIDENCE.md)を参照してください。

---

## English

`backend/libarchive/` holds a user-trusted `bsdtar.exe`, its minimum DLL set, the generated `backend-manifest.tsv`, and provenance, SPDX SBOM, and license inventory under `.iroha-zip-evidence/`. Source distributions do not include these binaries.

Normally use **Import bundle** or **Import from MSYS2** in iroha-zip Settings. For automation, use:

```powershell
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64
```

To import a prepared minimal bundle instead:

```powershell
.\scripts\install-backend.ps1 `
  -SourceDirectory C:\path\to\bundle `
  -AllowUnsupportedSource
```

An arbitrary bundle has no verified distributor signature or source, so Settings requires explicit warning confirmation and the script requires `-AllowUnsupportedSource`. iroha-zip rejects disagreement among the manifest, provenance, SBOM, license inventory, and actual payload tree. See the [backend evidence contract](../docs/BACKEND_EVIDENCE.md).
