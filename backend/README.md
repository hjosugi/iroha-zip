# Backend directory

`backend/libarchive/`には、利用者が信頼する`bsdtar.exe`と必要最小限のDLL、および自動生成される`backend-manifest.tsv`を配置します。

ソース配布にはバイナリを含めません。

通常は`iroha-zip 設定`の「bundleを取り込む」または「MSYS2から取り込む」を使用してください。次のスクリプトは自動化用です。

```powershell
.\scripts\export-msys2-backend.ps1 -Msys2Root C:\msys64
```

または、用意済みの最小bundleを取り込みます。

```powershell
.\scripts\install-backend.ps1 -SourceDirectory C:\path\to\bundle
```

iroha-zipはマニフェストにない余分なファイルも拒否します。
