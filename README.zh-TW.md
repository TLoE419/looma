# Looma · 織象

讓想法，逐漸成形。透過人工智慧生成圖片，將想法放進可以自由排列的桌面工作空間。

[English](README.md) · [貢獻指南](CONTRIBUTING.md) · [安全政策](SECURITY.md)

目前是早期原型，介面為繁體中文，僅提供原始碼，尚無可下載的桌面安裝版本。

## 目前原始碼已實作

- 透過原生選檔視窗匯入 PNG、JPEG、WebP，支援一次選取多張。
- 選取、排列與拖曳圖片區塊；平移、縮放、顯示全部圖片。
- 使用本機已登入的 Codex CLI，依文字描述生成圖片。
- 在同一輸入區請求一次性的文字回覆。
- 每次成功生成的圖片保留於獨立目錄。

長期方向是讓軟體工程師透過語音與圖片溝通想法。**語音輸入／輸出、工作桌保存、
圖片編輯、圖片歷史與連續對話記憶尚未實作。** SQLite 是已選定但尚未接入的保存方案，
詳見 [資料保存決策](STORAGE.md)。

## 從原始碼啟動

目前開發基準為 macOS、Rust 1.90.0 與 Xcode Command Line Tools。
Linux 與 Windows 尚未驗證。建議使用 rustup 管理工具鏈；專案的
`rust-toolchain.toml` 固定版本並指定格式與靜態檢查元件。若使用 Homebrew 安裝，
需自行確認編譯器與元件版本一致。

在專案目錄執行：

```sh
cargo run --locked
```

匯入、排列圖片不需要登入 Codex。文字與生成圖片功能需另外安裝 Codex CLI，
執行 `codex login`，且啟動 Looma 的環境必須找得到 `codex`。
本次準備文件時，本機 CLI 為 0.162.1；其他版本相容性尚未驗證。
帳號需有對應模型與工具權限，使用限制與費用依帳號及服務規則；Looma 不附憑證或使用額度。

也可以在啟動時載入既有圖片：

```sh
cargo run --locked -- /absolute/path/to/image.png
```

## 資料與限制

- 輸入文字會交給 Codex 及其設定的服務處理，圖片生成需要連線。
  請勿在提示文字或公開回報中放入憑證與機密。
- 選取圖片不代表圖片會送給人工智慧；目前請求只有輸入的文字，每次互相獨立。
- 工作桌排列與對話歷史不會保存；匯入圖片仍引用原始檔案，關閉後排列會消失。
- 生成圖片保留於**編譯時原始碼目錄**下的 `generated-images/image-*/generated.png`。
  該目錄必須可寫入；正式散布獨立執行檔前需要調整保存位置。
- 文字最多等待兩分鐘，圖片最多五分鐘。逾時會要求終止直接啟動的 CLI 子程序，
  但不保證所有衍生程序都會被終止。
- 呼叫使用暫時工作階段並略過使用者的主要設定檔，登入沿用本機憑證。
  這不代表外部服務完全不保留資料，也不代表所有其他設定來源都被停用。

## 已知相依套件警告

2026-10-17 的全鎖定檔 RustSec 比對回報 2 項安全性不健全與 3 項停止維護警告，
嚴格檢查尚未通過。目前 macOS 相依路徑包含已停止維護的 `paste`；其餘警告路徑
涉及 Linux 與 Android。沒有忽略任何通報，詳見 [公開檢查清單](docs/PUBLICATION_CHECKLIST.md#unresolved-dependency-decisions-before-public-exposure)。
此原型不能視為已完成安全稽核的正式產品。

## 開發驗證

```sh
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

預設測試不發出真實服務請求；兩項略過的測試須另外執行，可能消耗帳號額度：

```sh
cargo test --locked real_reply -- --ignored
cargo test --locked real_image -- --ignored --nocapture
```

自動測試涵蓋工作桌狀態、幾何運算與生成圖片驗證，不能證明桌面互動正常。
實際操作請依 [人工檢查清單](docs/SMOKE_TEST.md) 記錄結果。

## 參與及授權

歡迎小而明確的貢獻；較大功能請先依 [貢獻指南](CONTRIBUTING.md) 討論。
規劃方向不代表交付承諾。Looma 不代表 OpenAI 官方產品或合作關係。

原始碼採 [MIT 授權](LICENSE)；第三方元件、生成與匯入內容仍依各自適用條款。

## 英文對照

- **CLI** — 命令列介面，此處是 Looma 在背景呼叫的 Codex 工具。
- **Rust** — 本專案使用的程式語言。
- **SQLite** — 尚未實作的本機內嵌資料庫選擇。
- **rustup** — 安裝與切換 Rust 工具鏈的管理工具。
- **Homebrew** — macOS 套件管理工具。
- **MIT** — 允許使用、修改與散布，要求保留授權及著作權聲明的開源授權。
- **PNG / JPEG / WebP** — 目前可匯入的圖片格式。
- **Xcode Command Line Tools** — macOS 編譯所需的命令列開發工具。
