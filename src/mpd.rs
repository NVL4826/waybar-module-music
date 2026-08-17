use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MpdState {
    Playing,
    Paused,
    #[default]
    Stopped,
}

impl MpdState {
    pub fn from_mpd_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "play" => MpdState::Playing,
            "pause" => MpdState::Paused,
            _ => MpdState::Stopped,
        }
    }

    pub fn as_class_str(&self) -> &'static str {
        match self {
            MpdState::Playing => "playing",
            MpdState::Paused => "paused",
            MpdState::Stopped => "stopped",
        }
    }
}

impl fmt::Display for MpdState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_class_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MpdStatus {
    pub state: MpdState,
    pub volume: Option<u8>,
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MpdSong {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub file: Option<String>,
    pub duration_seconds: Option<u64>,
}

impl MpdSong {
    /// Returns a clean display title (Title tag or filename without extension).
    pub fn display_title(&self) -> String {
        if let Some(ref title) = self.title {
            let t = title.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
        if let Some(ref file) = self.file {
            if let Some(name) = std::path::Path::new(file).file_name().and_then(|n| n.to_str()) {
                if let Some(stem) = name.strip_suffix(".mp3")
                    .or_else(|| name.strip_suffix(".flac"))
                    .or_else(|| name.strip_suffix(".m4a"))
                    .or_else(|| name.strip_suffix(".ogg"))
                    .or_else(|| name.strip_suffix(".opus"))
                    .or_else(|| name.strip_suffix(".wav"))
                {
                    return stem.to_string();
                }
                return name.to_string();
            }
        }
        "Không có tiêu đề".to_string()
    }
}

pub struct MpdClient {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl MpdClient {
    /// Connects to the MPD server with a connection timeout.
    pub fn connect(host: &str, port: u16, timeout: Duration) -> std::io::Result<Self> {
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = std::net::ToSocketAddrs::to_socket_addrs(&addr_str)?
            .collect();

        let socket_addr = addrs.first().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Could not resolve host")
        })?;

        let stream = TcpStream::connect_timeout(socket_addr, timeout)?;
        stream.set_nodelay(true)?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);

        // Read the MPD banner: "OK MPD <version>\n"
        let mut banner = String::new();
        reader.read_line(&mut banner)?;
        if !banner.starts_with("OK MPD") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid MPD banner: {banner}"),
            ));
        }

        Ok(Self { stream, reader })
    }

    /// Queries the current MPD playback status and current song in a single atomic batch.
    pub fn query_status_and_song(&mut self) -> std::io::Result<(MpdStatus, MpdSong)> {
        self.stream
            .write_all(b"command_list_begin\nstatus\ncurrentsong\ncommand_list_end\n")?;
        self.stream.flush()?;

        let mut status = MpdStatus::default();
        let mut song = MpdSong::default();

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MPD connection closed unexpectedly",
                ));
            }

            let trimmed = line.trim();
            if trimmed == "OK" {
                break;
            }
            if trimmed.starts_with("ACK") {
                return Err(std::io::Error::other(format!("MPD Error: {trimmed}")));
            }

            if let Some((key, val)) = trimmed.split_once(':') {
                let key = key.trim();
                let val = val.trim();

                match key {
                    "state" => status.state = MpdState::from_mpd_str(val),
                    "volume" => {
                        if let Ok(vol) = val.parse::<i32>() {
                            if vol >= 0 {
                                status.volume = Some(vol.clamp(0, 100) as u8);
                            }
                        }
                    }
                    "duration" => {
                        if let Ok(dur) = val.parse::<f64>() {
                            if dur.is_finite() && dur >= 0.0 {
                                let dur_secs = dur.round() as u64;
                                status.duration_seconds = Some(dur_secs);
                                song.duration_seconds = Some(dur_secs);
                            }
                        }
                    }

                    "time" => {
                        if let Some((_, total_str)) = val.split_once(':') {
                            if let Ok(dur) = total_str.parse::<u64>() {
                                status.duration_seconds = Some(dur);
                                song.duration_seconds = Some(dur);
                            }
                        }
                    }
                    "Title" => song.title = Some(val.to_string()),
                    "Artist" => song.artist = Some(val.to_string()),
                    "Album" => song.album = Some(val.to_string()),
                    "file" => song.file = Some(val.to_string()),
                    "Time" => {
                        if let Ok(dur) = val.parse::<u64>() {
                            if song.duration_seconds.is_none() {
                                song.duration_seconds = Some(dur);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok((status, song))
    }

    /// Blocks on MPD `idle` until a player, mixer, or options event occurs.
    pub fn wait_for_idle(&mut self) -> std::io::Result<()> {
        self.stream.write_all(b"idle player mixer options\n")?;
        self.stream.flush()?;

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MPD connection closed during idle",
                ));
            }

            let trimmed = line.trim();
            if trimmed == "OK" {
                break;
            }
            if trimmed.starts_with("ACK") {
                return Err(std::io::Error::other(format!("MPD idle Error: {trimmed}")));
            }
        }

        Ok(())
    }
}

/// Continuous event monitor that handles connection lifecycle, automatic retries, and idle event dispatch.
pub struct MpdMonitor {
    host: String,
    port: u16,
    reconnect_interval: Duration,
}

impl MpdMonitor {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            reconnect_interval: Duration::from_secs(2),
        }
    }

    #[allow(dead_code)]
    pub fn with_reconnect_interval(mut self, interval: Duration) -> Self {
        self.reconnect_interval = interval;
        self
    }


    /// Continuously monitors MPD, invoking `on_update` with `Some((status, song))` on state changes,
    /// or `None` when MPD is disconnected or offline.
    pub fn run<F>(&self, mut on_update: F)
    where
        F: FnMut(Option<(&MpdStatus, &MpdSong)>),
    {
        // Initial disconnected / stopped state
        on_update(None);

        loop {
            log::debug!("Connecting to MPD at {}:{}", self.host, self.port);
            match MpdClient::connect(&self.host, self.port, Duration::from_secs(2)) {
                Ok(mut client) => {
                    log::info!("Connected to MPD successfully");
                    loop {
                        match client.query_status_and_song() {
                            Ok((status, song)) => {
                                log::debug!("MPD state: {:?}, song: {:?}", status.state, song.title);
                                on_update(Some((&status, &song)));
                            }
                            Err(err) => {
                                log::warn!("Failed to query MPD status: {err}");
                                break;
                            }
                        }

                        // Block on MPD idle events (0% CPU, 0ms latency on changes)
                        if let Err(err) = client.wait_for_idle() {
                            log::warn!("MPD idle connection closed: {err}");
                            break;
                        }
                    }
                }
                Err(err) => {
                    log::debug!("Unable to connect to MPD: {err}");
                }
            }

            // Connection lost / unavailable
            on_update(None);
            std::thread::sleep(self.reconnect_interval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpd_state_parsing() {
        assert_eq!(MpdState::from_mpd_str("play"), MpdState::Playing);
        assert_eq!(MpdState::from_mpd_str("pause"), MpdState::Paused);
        assert_eq!(MpdState::from_mpd_str("stop"), MpdState::Stopped);
        assert_eq!(MpdState::from_mpd_str("other"), MpdState::Stopped);
    }

    #[test]
    fn test_mpd_song_display_title() {
        let song_with_title = MpdSong {
            title: Some("Test Track".to_string()),
            ..Default::default()
        };
        assert_eq!(song_with_title.display_title(), "Test Track");

        let song_with_file = MpdSong {
            file: Some("/path/to/my_favorite_song.mp3".to_string()),
            ..Default::default()
        };
        assert_eq!(song_with_file.display_title(), "my_favorite_song");

        let empty = MpdSong::default();
        assert_eq!(empty.display_title(), "Không có tiêu đề");
    }

    #[test]
    fn test_mpd_monitor_builder() {
        let monitor = MpdMonitor::new("127.0.0.1", 6600)
            .with_reconnect_interval(Duration::from_millis(500));
        assert_eq!(monitor.host, "127.0.0.1");
        assert_eq!(monitor.port, 6600);
        assert_eq!(monitor.reconnect_interval, Duration::from_millis(500));
    }
}

