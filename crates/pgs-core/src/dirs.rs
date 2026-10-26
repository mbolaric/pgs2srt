//! Per-user folders of the app on each platform.

use std::path::PathBuf;

const APP: &str = "pgs2srt";

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn xdg(variable: &str, fallback: &str) -> Option<PathBuf> {
    env_dir(variable).or_else(|| home_dir().map(|home| home.join(fallback)))
}

pub fn home_dir() -> Option<PathBuf> {
    let home = env_dir("HOME");
    #[cfg(windows)]
    let home = home.or_else(|| env_dir("USERPROFILE"));
    home
}

pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = env_dir("APPDATA").or_else(|| env_dir("LOCALAPPDATA"));
    #[cfg(target_os = "macos")]
    let base = home_dir().map(|home| home.join("Library/Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = xdg("XDG_CONFIG_HOME", ".config");
    base.map(|base| base.join(APP))
}

pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = env_dir("LOCALAPPDATA");
    #[cfg(target_os = "macos")]
    let base = home_dir().map(|home| home.join("Library/Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = xdg("XDG_DATA_HOME", ".local/share");
    base.map(|base| base.join(APP))
}

pub fn log_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let dir = env_dir("LOCALAPPDATA").map(|base| base.join(APP).join("logs"));
    #[cfg(target_os = "macos")]
    let dir = home_dir().map(|home| home.join("Library/Logs").join(APP));
    #[cfg(not(any(windows, target_os = "macos")))]
    let dir = xdg("XDG_STATE_HOME", ".local/state").map(|base| base.join(APP).join("logs"));
    dir
}
