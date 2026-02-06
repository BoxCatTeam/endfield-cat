use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum GithubMirrorSource {
    GhProxyCf,
    GhProxyFastly,
    GhProxyEdgeone,
    Ghfast,
    Custom,
}

impl Default for GithubMirrorSource {
    fn default() -> Self {
        Self::GhProxyCf
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GithubMirrorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub source: GithubMirrorSource,
    #[serde(default)]
    pub custom_template: Option<String>,
}

impl GithubMirrorConfig {
    /// 根据镜像配置转换 GitHub URL
    pub fn transform_url(&self, original_url: &str) -> String {
        if !self.enabled {
            return original_url.to_string();
        }

        let template = match self.source {
            GithubMirrorSource::GhProxyCf => "https://gh-proxy.org/{url}",
            GithubMirrorSource::GhProxyFastly => "https://cdn.gh-proxy.org/{url}",
            GithubMirrorSource::GhProxyEdgeone => "https://edgeone.gh-proxy.org/{url}",
            GithubMirrorSource::Ghfast => "https://ghfast.top/{url}",
            GithubMirrorSource::Custom => {
                self.custom_template.as_deref().unwrap_or("{url}")
            }
        };

        template.replace("{url}", original_url)
    }
}

/// 从配置文件读取 GitHub 镜像配置
pub fn read_mirror_config(exe_dir: &Path) -> GithubMirrorConfig {
    let config_path = exe_dir.join("data").join("config").join("config.json");
    if !config_path.exists() {
        return GithubMirrorConfig::default();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return GithubMirrorConfig::default(),
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return GithubMirrorConfig::default(),
    };

    json.get("githubMirror")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_url_disabled() {
        let config = GithubMirrorConfig {
            enabled: false,
            source: GithubMirrorSource::GhProxyCf,
            custom_template: None,
        };
        let url = "https://github.com/user/repo/releases/download/v1.0/app.exe";
        assert_eq!(config.transform_url(url), url);
    }

    #[test]
    fn test_transform_url_enabled() {
        let config = GithubMirrorConfig {
            enabled: true,
            source: GithubMirrorSource::GhProxyCf,
            custom_template: None,
        };
        let url = "https://github.com/user/repo/releases/download/v1.0/app.exe";
        let expected = "https://gh-proxy.org/https://github.com/user/repo/releases/download/v1.0/app.exe";
        assert_eq!(config.transform_url(url), expected);
    }

    #[test]
    fn test_transform_url_custom() {
        let config = GithubMirrorConfig {
            enabled: true,
            source: GithubMirrorSource::Custom,
            custom_template: Some("https://my-proxy.com/{url}".to_string()),
        };
        let url = "https://github.com/user/repo/file.zip";
        let expected = "https://my-proxy.com/https://github.com/user/repo/file.zip";
        assert_eq!(config.transform_url(url), expected);
    }
}
