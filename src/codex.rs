//! Run one-shot Codex subprocesses for text replies and image generation.
//! Uses the locally installed Codex executable and its available authentication.

use std::time::Duration;
use tokio::process::Command;

/// Return trimmed stdout from a text request; timeouts, process failures, and empty replies are errors.
pub async fn ask(prompt: String) -> Result<String, String> {
    let mut command = Command::new("codex");
    command
        .args([
            "exec",
            "--ignore-user-config",
            "--sandbox",
            "read-only",
            "--skip-git-repo-check",
            "--ephemeral",
            "--color",
            "never",
            "--",
        ])
        .arg(prompt) // Pass the prompt as one argument after -- so it is not parsed as command options.
        .kill_on_drop(true); // Request termination of the direct child on drop; descendants are not managed here.

    let output = tokio::time::timeout(Duration::from_secs(120), command.output())
        .await
        .map_err(|_| "等待回覆超過兩分鐘，請稍後重試。".to_string())?
        .map_err(|error| format!("無法啟動 Codex，請確認已安裝：{error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let reply = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if reply.is_empty() {
        return Err("Codex 沒有傳回文字。".to_string());
    }
    Ok(reply)
}

/// Generate a PNG in a per-request directory and return its retained file path.
/// Validate PNG decoding before retaining the file; image content is not assessed.
pub async fn generate_image(description: String) -> Result<String, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("generated-images"); // Resolve output from the build-time project directory, not the current working directory.
    std::fs::create_dir_all(&root).map_err(|error| format!("無法建立圖片目錄：{error}"))?;

    // Separate output directories prevent collisions; drop attempts cleanup unless keep() retains the directory.
    let directory = tempfile::Builder::new()
        .prefix("image-")
        .tempdir_in(root)
        .map_err(|error| format!("無法建立本次生成目錄：{error}"))?;

    let prompt = format!(
        "使用原生圖片生成工具生成圖片，描述如下：\n{description}\n\n\
         將實際生成的圖片複製到目前目錄的 generated.png，必須為 PNG 格式。\
         不要以 SVG、HTML 或程式繪圖代替，不要使用 API 腳本，不要委派其他代理。\
         若工具不可用或生成失敗，請直接說明原因，不要建立替代圖片。"
    );
    let mut command = Command::new("codex");
    command
        .current_dir(directory.path())
        .args([
            "exec",
            "--ignore-user-config",
            "--sandbox",
            "workspace-write",
            "--skip-git-repo-check",
            "--ephemeral",
            "--color",
            "never",
            "--",
        ])
        .arg(prompt)
        .kill_on_drop(true); // Request termination of the direct child on drop; descendants are not managed here.

    let output = tokio::time::timeout(Duration::from_secs(300), command.output())
        .await
        .map_err(|_| "生成圖片超過五分鐘，請稍後重試。".to_string())?
        .map_err(|error| format!("無法執行 Codex：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    validate_and_keep_image(directory, &output.stdout)
}

/// Own the request directory until decoding succeeds; errors drop it and attempt cleanup.
/// On success, keep the directory so the returned image path remains usable.
fn validate_and_keep_image(directory: tempfile::TempDir, stdout: &[u8]) -> Result<String, String> {
    let path = directory.path().join("generated.png"); // Must match the filename requested in the generation prompt.
    let bytes = std::fs::read(&path)
        .map_err(|_| format!("沒有取得圖片：{}", String::from_utf8_lossy(stdout).trim()))?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("生成結果不是 PNG 圖片。".to_string());
    }

    // A valid PNG signature can still precede truncated or corrupt pixel data.
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|error| format!("生成的 PNG 圖片無法讀取：{error}"))?;
    let _ = directory.keep(); // Preserve successful output so the UI can load it after this function returns.
    Ok(path.to_string_lossy().into_owned())
}

// Validation tests are local; opt-in smoke tests below send real requests.
#[cfg(test)]
mod tests {
    #[test]
    fn valid_png_is_kept_and_remains_decodable() {
        let parent = tempfile::tempdir().unwrap();
        let directory = tempfile::tempdir_in(parent.path()).unwrap();
        let path = directory.path().join("generated.png");
        let pixels = image::RgbImage::from_pixel(2, 3, image::Rgb([12, 34, 56]));
        pixels.save(&path).unwrap();

        let retained = super::validate_and_keep_image(directory, b"").unwrap();

        assert_eq!(std::path::Path::new(&retained), path);
        let decoded = image::open(&retained).unwrap();
        assert_eq!(decoded.to_rgb8(), pixels);
    }

    #[test]
    fn corrupt_png_is_rejected_and_request_directory_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let request_path = directory.path().to_path_buf();
        std::fs::write(
            request_path.join("generated.png"),
            b"\x89PNG\r\n\x1a\ncorrupt image data",
        )
        .unwrap();

        let error = super::validate_and_keep_image(directory, b"").unwrap_err();

        assert!(error.starts_with("生成的 PNG 圖片無法讀取："));
        assert!(!request_path.exists());
    }

    #[test]
    fn non_png_is_rejected_and_request_directory_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let request_path = directory.path().to_path_buf();
        image::RgbImage::new(2, 3)
            .save_with_format(request_path.join("generated.png"), image::ImageFormat::Jpeg)
            .unwrap();

        let error = super::validate_and_keep_image(directory, b"").unwrap_err();

        assert_eq!(error, "生成結果不是 PNG 圖片。");
        assert!(!request_path.exists());
    }

    #[test]
    fn missing_png_is_rejected_and_request_directory_is_removed() {
        let parent = tempfile::tempdir().unwrap();
        let existing = parent.path().join("existing.png");
        std::fs::write(&existing, b"existing artifact").unwrap();
        let directory = tempfile::tempdir_in(parent.path()).unwrap();
        let request_path = directory.path().to_path_buf();

        let error =
            super::validate_and_keep_image(directory, b" No image was produced. \n").unwrap_err();

        assert_eq!(error, "沒有取得圖片：No image was produced.");
        assert!(!request_path.exists());
        assert_eq!(std::fs::read(existing).unwrap(), b"existing artifact");
    }

    #[test]
    #[ignore = "需要已登入的 Codex 與網路，會發出一次真實請求"]
    fn real_reply() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let reply = runtime
            .block_on(super::ask("只回覆 LOOMA_OK，不要使用工具。".to_string()))
            .expect("應取得真實回覆");
        assert_eq!(reply.trim(), "LOOMA_OK");
    }

    #[test]
    #[ignore = "需要已登入的 Codex 與網路，會真實生成一張圖片"]
    fn real_image() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let path = runtime
            .block_on(super::generate_image(
                "一隻藍色小鳥，簡單插畫，白色背景".to_string(),
            ))
            .expect("應取得真實圖片");
        assert!(std::fs::metadata(&path).unwrap().len() > 8); // Check that output exists beyond the signature, not that it matches the prompt visually.
        println!("生成圖片：{path}");
    }
}
