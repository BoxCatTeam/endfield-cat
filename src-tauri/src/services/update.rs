use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize)]
pub struct UpdateProgress {
    pub stage: String,
    pub progress: u32,
}

pub struct UpdatePaths {
    pub temp_dir: PathBuf,
    pub new_exe: PathBuf,
    pub batch_path: PathBuf,
}

pub fn prepare_paths(exe_name: &std::ffi::OsStr) -> Result<UpdatePaths, String> {
    let temp_dir = std::env::temp_dir().join("endfield-cat-update");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let new_exe = temp_dir.join(exe_name);
    let batch_path = temp_dir.join("updater.bat");

    Ok(UpdatePaths {
        temp_dir,
        new_exe,
        batch_path,
    })
}

pub async fn download_new_exe<F>(
    client: &reqwest::Client,
    download_url: &str,
    dest: &Path,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(u32),
{
    use futures_util::StreamExt;
    use std::io::Write;

    let resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            on_progress(progress);
        }
    }

    Ok(())
}

pub fn build_updater_batch(
    exe_name: &str,
    new_exe: &Path,
    current_exe: &Path,
    temp_dir: &Path,
) -> String {
    let batch_content = format!(
        r#"@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
echo 正在更新 endfield-cat...
echo Updating endfield-cat...

:wait_loop
tasklist /FI "IMAGENAME eq {exe_name}" 2>NUL | find /I "{exe_name}" >NUL
if "%ERRORLEVEL%"=="0" (
    timeout /t 1 /nobreak >nul
    goto wait_loop
)

echo 正在替换文件...
copy /Y "{new_exe}" "{current_exe}" >nul
if errorlevel 1 (
    echo 更新失败，请手动替换文件
    pause
    exit /b 1
)

echo 启动新版本...
start "" /min "{current_exe}"

echo 清理临时文件...
start "" /min powershell -NoProfile -ExecutionPolicy Bypass -Command "param([string]$p) Start-Sleep -Seconds 3; if (Test-Path -LiteralPath $p) {{ Remove-Item -LiteralPath $p -Recurse -Force }}" "{temp_dir}"

exit /b 0
"#,
        exe_name = exe_name,
        new_exe = new_exe.to_string_lossy(),
        current_exe = current_exe.to_string_lossy(),
        temp_dir = temp_dir.to_string_lossy()
    );

    batch_content.replace('\n', "\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_updater_batch_uses_powershell_literalpath_cleanup() {
        let content = build_updater_batch(
            "endfield-cat.exe",
            Path::new("C:\\Temp\\endfield-cat-update\\new.exe"),
            Path::new("C:\\Program Files\\EndCat\\endfield-cat.exe"),
            Path::new("C:\\Temp\\endfield-cat-update"),
        );

        // In cmd.exe, `\"` is not an escape; it can break parsing and lead to paths like `\\`.
        assert!(!content.contains("\\\""));
        assert!(content.contains(r#"powershell -NoProfile -ExecutionPolicy Bypass -Command "param([string]$p) Start-Sleep -Seconds 3; if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force }""#));
        assert!(content.contains(r#""C:\Temp\endfield-cat-update""#));
    }

    #[test]
    fn build_updater_batch_cleanup_quotes_ampersand_path() {
        let content = build_updater_batch(
            "endfield-cat.exe",
            Path::new("C:\\Temp\\endfield-cat-update\\new.exe"),
            Path::new("C:\\Program Files\\EndCat\\endfield-cat.exe"),
            Path::new("C:\\Temp\\A&B\\endfield-cat-update"),
        );

        assert!(content.contains(r#""C:\Temp\A&B\endfield-cat-update""#));
    }
}
