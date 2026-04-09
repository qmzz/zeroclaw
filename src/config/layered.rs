use anyhow::Result;
use std::path::{Path, PathBuf};

use super::schema::Config;

#[derive(Debug, Clone)]
pub struct LayeredConfigPaths {
    pub user: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub local: Option<PathBuf>,
}

pub fn default_layered_paths(workspace_dir: &Path) -> LayeredConfigPaths {
    let user = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".zeroclaw/config.toml"));
    let project = Some(workspace_dir.join(".zeroclaw/config.toml"));
    let local = Some(workspace_dir.join(".zeroclaw/config.local.toml"));

    LayeredConfigPaths {
        user,
        project,
        local,
    }
}

/// Load config with layered precedence (later overrides earlier):
/// user -> project -> local -> env (handled by underlying loader)
pub async fn load_layered_or_init(workspace_dir: &Path) -> Result<Config> {
    // Base loader keeps existing behavior and env handling
    let mut cfg = Config::load_or_init().await?;

    let paths = default_layered_paths(workspace_dir);

    for (layer_name, path) in [
        ("user", paths.user),
        ("project", paths.project),
        ("local", paths.local),
    ]
    .into_iter()
    .filter_map(|(n, p)| p.map(|pp| (n, pp)))
    {
        if path.exists() {
            if let Ok(text) = tokio::fs::read_to_string(&path).await {
                if let Ok(layer_cfg) = toml::from_str::<Config>(&text) {
                    cfg.apply_layer(layer_cfg);
                    cfg.config_path = path.clone();
                    tracing::info!(layer = layer_name, path = %path.display(), "applied config layer");
                } else {
                    tracing::warn!(layer = layer_name, path = %path.display(), "failed to parse config layer");
                }
            }
        }
    }

    Ok(cfg)
}
