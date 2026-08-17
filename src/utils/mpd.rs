use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// Direct query to the local MPD daemon over TCP (127.0.0.1:6600) for current volume percentage (0-100).
pub fn get_mpd_volume() -> Option<u8> {
    let addr: SocketAddr = "127.0.0.1:6600".parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(50)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(50))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(50))).ok()?;

    let mut buf = [0u8; 256];
    let _ = stream.read(&mut buf).ok()?; // read MPD banner
    stream.write_all(b"status\nclose\n").ok()?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response).ok()?;

    for line in response.lines() {
        if let Some(vol_str) = line.strip_prefix("volume: ") {
            if let Ok(vol) = vol_str.trim().parse::<u8>() {
                return Some(vol);
            }
        }
    }

    None
}

/// Cached MPD volume provider that throttles TCP socket queries.
#[derive(Debug)]
pub struct MpdVolumeCache {
    cached_volume: Option<u8>,
    last_checked: Option<Instant>,
    ttl: Duration,
}

impl Default for MpdVolumeCache {
    fn default() -> Self {
        Self::new(Duration::from_millis(1500))
    }
}

impl MpdVolumeCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cached_volume: None,
            last_checked: None,
            ttl,
        }
    }

    pub fn get_volume(&mut self) -> Option<u8> {
        let now = Instant::now();
        if let Some(last) = self.last_checked {
            if now.duration_since(last) < self.ttl {
                return self.cached_volume;
            }
        }

        self.last_checked = Some(now);
        self.cached_volume = get_mpd_volume();
        self.cached_volume
    }

    pub fn invalidate(&mut self) {
        self.last_checked = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mpd_volume_graceful_fallback() {
        // Even if MPD is stopped/unavailable, get_mpd_volume should not panic and return Option
        let _ = get_mpd_volume();
    }

    #[test]
    fn test_mpd_volume_cache_caching() {
        let mut cache = MpdVolumeCache::new(Duration::from_secs(10));
        assert!(cache.last_checked.is_none());

        let _ = cache.get_volume();
        assert!(cache.last_checked.is_some());

        let last_time = cache.last_checked;
        // Immediate second call should reuse cached result without changing last_checked
        let _ = cache.get_volume();
        assert_eq!(cache.last_checked, last_time);

        // Invalidate should reset cache timestamp
        cache.invalidate();
        assert!(cache.last_checked.is_none());
    }
}

