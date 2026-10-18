# 資料保存方案

狀態：第一版選 SQLite，尚未實作；後續可依需求調整。

## 已決定

- 使用 SQLite，資料保存在使用者本機，由應用管理。
- 使用者不需另外安裝或啟動資料庫服務。
- 使用 rusqlite，啟用 `bundled`，將 SQLite 一起編譯進應用。
- 未來若需要多人共享服務，再評估伺服器端 PostgreSQL；目前不實作同步或遷移。

依據：[SQLite 介紹](https://www.sqlite.org/about.html)、[適用情境](https://www.sqlite.org/whentouse.html)。

## 待決定

- 資料結構與資料表版本升級方式。
- 對話、圖片版本與一般設定的保存分工。
- 圖片原檔的保存位置、備份與還原方式。
- 憑證管理：之後決定。

## 英文對照

- **SQLite** — 第一版採用的本機內嵌資料庫。
- **PostgreSQL** — 未來伺服器端資料庫的候選方案。
- **Rust** — 本專案採用的主要程式語言。
- **rusqlite** — Rust 操作 SQLite 的函式庫。
- **bundled** — 將 SQLite 一起編譯、連結進應用的功能選項。
