use std::{fs, path::PathBuf};

/// Gets PathBuf depending on given callback, then creates a new directory recursively within that with the module's name
/// Takes a function that returns a directory, for example, `dirs::cache_dir()`
pub fn get_and_create_dir<F>(callback: F) -> Result<PathBuf, std::io::Error>
where
    F: Fn() -> Option<PathBuf>,
{
    let directory = callback()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "could not get directory")
        })?
        .join("waybar-module-music");

    fs::create_dir_all(&directory)?;

    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_and_create_dir_success() {
        let temp_base = env::temp_dir().join("test_waybar_music_dir");
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
