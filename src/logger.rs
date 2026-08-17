use std::{fs, path::PathBuf};
use simplelog::{CombinedLogger, Config as LogConfig, LevelFilter, WriteLogger};

/// Gets PathBuf from a callback (e.g. `dirs::cache_dir`) and ensures the subfolder exists recursively.
pub fn get_and_create_dir<F>(callback: F) -> Result<PathBuf, std::io::Error>
where
    F: Fn() -> Option<PathBuf>,
{
    let directory = callback()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "could not get directory"))?
        .join("waybar-module-music");

    fs::create_dir_all(&directory)?;
    Ok(directory)
}

/// Initializes file logger in the cache directory.
pub fn init_logger(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = get_and_create_dir(dirs::cache_dir)?;
    let log_path = cache_dir.join("app.log");

    CombinedLogger::init(vec![WriteLogger::new(
        if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        LogConfig::default(),
        fs::File::create(log_path)?,
    )])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_and_create_dir_success() {
        let temp_base = env::temp_dir().join("test_waybar_music_log_dir");
        let result = get_and_create_dir(|| Some(temp_base.clone()));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.ends_with("waybar-module-music"));
        let _ = fs::remove_dir_all(temp_base);
    }

    #[test]
    fn test_get_and_create_dir_not_found() {
        let result = get_and_create_dir(|| None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}
