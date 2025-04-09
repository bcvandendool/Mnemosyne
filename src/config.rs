use arc_swap::{ArcSwap, Cache};
use directories::ProjectDirs;
use figment::providers::{Format, Serialized, Toml};
use figment::Figment;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct UIConfig {
    start_last_game_on_launch: bool,
    pub(crate) game_folder: String,
}

impl Default for UIConfig {
    fn default() -> Self {
        UIConfig {
            start_last_game_on_launch: false,
            game_folder: "./src/roms/".to_string(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub(crate) struct GameBoyConfig {
    pub(crate) rom_path: String,
    pub(crate) save_path: String,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) ui_config: UIConfig,
    pub(crate) gameboy_config: GameBoyConfig,
}

static GLOBAL_CONFIG: LazyLock<ArcSwap<Config>> = LazyLock::new(|| {
    let project_dirs = ProjectDirs::from("", "", "Mnemosyne").unwrap();
    let mut config_path = PathBuf::new();
    config_path.push(project_dirs.config_dir());
    fs::create_dir_all(&config_path).expect("Failed to create config directory");
    config_path.push("config.toml");

    ArcSwap::from_pointee(
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file(config_path))
            .extract()
            .unwrap(),
    )
});

thread_local! {
    pub static THREAD_LOCAL_CONFIG: RefCell<Cache<&'static ArcSwap<Config>, Arc<Config>>> = RefCell::new(Cache::from(GLOBAL_CONFIG.deref()));
}

pub(crate) fn save_config() {
    let config = THREAD_LOCAL_CONFIG.with(|c| c.borrow_mut().load().clone());

    let project_dirs = ProjectDirs::from("", "", "Mnemosyne").unwrap();
    let mut config_path = PathBuf::new();
    config_path.push(project_dirs.config_dir());
    fs::create_dir_all(&config_path).expect("Failed to create config directory");
    config_path.push("config.toml");

    let toml_string = toml::to_string_pretty(&config).unwrap();
    fs::write(config_path, &toml_string).unwrap();
}
