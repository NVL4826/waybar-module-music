use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Queries the local MPD daemon over TCP (127.0.0.1:6600) for current volume percentage (0-100).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mpd_volume_graceful_fallback() {
        // Even if MPD is stopped/unavailable, get_mpd_volume should not panic and return Option
        let _ = get_mpd_volume();
    }
}
