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

fn merge_values(base: &mut toml::Value, override_v: toml::Value) {
    match (base, override_v) {
        (toml::Value::Table(base_t), toml::Value::Table(ov_t)) => {
            for (k, v) in ov_t {
                match base_t.get_mut(&k) {
                    Some(existing) => merge_values(existing, v),
                    None => {
                        base_t.insert(k, v);
                    }
                }
            }
        }
        (base_slot, ov) => {
            *base_slot = ov;
        }
    }
}

/// Apply user/project/local config layers onto an already-loaded base config.
///
/// Later files override earlier ones: `user -> project -> local`.
///
/// Important: layers are merged from raw TOML values rather than deserializing
/// each layer into a full `Config`, which would inject defaults and accidentally
/// overwrite unrelated base settings.
pub async fn apply_layered_files(mut cfg: Config) -> Result<Config> {
    let workspace_dir = cfg.workspace_dir.clone();
    let original_config_path = cfg.config_path.clone();
    let paths = default_layered_paths(&workspace_dir);

    let mut base_v = toml::Value::try_from(cfg.clone())
        .unwrap_or(toml::Value::Table(toml::map::Map::new()));

    for (layer_name, path) in [
        ("user", paths.user),
        ("project", paths.project),
        ("local", paths.local),
    ]
    .into_iter()
    .filter_map(|(n, p)| p.map(|pp| (n, pp)))
    {
        if !path.exists() {
            continue;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(text) => match toml::from_str::<toml::Value>(&text) {
                Ok(layer_v) => {
                    merge_values(&mut base_v, layer_v);
                    cfg.config_path = path.clone();
                    tracing::info!(layer = layer_name, path = %path.display(), "applied config layer");
                }
                Err(error) => {
                    tracing::warn!(layer = layer_name, path = %path.display(), %error, "failed to parse config layer");
                }
            },
            Err(error) => {
                tracing::warn!(layer = layer_name, path = %path.display(), %error, "failed to read config layer");
            }
        }
    }

    if let Ok(mut merged) = base_v.try_into::<Config>() {
        merged.workspace_dir = workspace_dir;
        merged.config_path = if cfg.config_path.as_os_str().is_empty() {
            original_config_path
        } else {
            cfg.config_path.clone()
        };
        cfg = merged;
    }

    Ok(cfg)
}

/// Backward-compatible helper that loads the base config first, then applies layers.
pub async fn load_layered_or_init(workspace_dir: &Path) -> Result<Config> {
    let mut cfg = Config::load_or_init().await?;
    cfg.workspace_dir = workspace_dir.to_path_buf();
    apply_layered_files(cfg).await
}
