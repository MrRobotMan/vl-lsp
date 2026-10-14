#![allow(dead_code)]
use std::path::PathBuf;

mod document;
pub mod server;

pub fn log_dir() -> PathBuf {
    let base = {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|home| home.join("Library/Logs"))
        }

        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir()
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            dirs::state_dir()
        }
    };
    let app = "vl_lsp";
    base.map_or_else(|| std::env::temp_dir().join(app), |d| d.join(app))
}
