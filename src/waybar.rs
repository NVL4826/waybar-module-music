use serde::Serialize;

use crate::cli::Args;
use crate::mpd::{MpdSong, MpdState, MpdStatus};

/// Formats seconds into "mm:ss" or "hh:mm:ss" if duration >= 1 hour.
pub fn seconds_to_formatted_time(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Serialize)]
struct WaybarOutput<'a> {
    text: &'a str,
    tooltip: &'a str,
    class: &'a str,
    alt: &'a str,
}

pub struct WaybarDisplay {
    last_output: std::sync::Mutex<String>,
}

impl Default for WaybarDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl WaybarDisplay {
    pub fn new() -> Self {
        Self {
            last_output: std::sync::Mutex::new(String::new()),
        }
    }

    /// Prints output to stdout if it changed from the previous output.
    pub fn print_if_changed(&self, output: &str) {
        let mut last = self.last_output.lock().unwrap();
        if *last != output {
            *last = output.to_string();
            println!("{output}");
        }
    }

    /// Formats the current MPD status and song into Waybar JSON string.
    pub fn format_output(&self, args: &Args, status: &MpdStatus, song: &MpdSong) -> String {
        if status.state == MpdState::Stopped {
            return self.format_stopped(args);
        }

        let icon = match status.state {
            MpdState::Playing => &args.play_icon,
            MpdState::Paused => &args.pause_icon,
            MpdState::Stopped => &args.stopped_icon,
        };

        let title = song.display_title();
        let artist = song.artist.as_deref().unwrap_or("").trim();
        let album = song.album.as_deref().unwrap_or("").trim();
        let duration_str = song
            .duration_seconds
            .or(status.duration_seconds)
            .map(seconds_to_formatted_time)
            .unwrap_or_default();
        let volume_str = status.volume.map(|v| format!("{v}%")).unwrap_or_default();

        let display_text = args
            .format
            .replace("%icon%", icon)
            .replace("%title%", &title)
            .replace("%artist%", artist)
            .replace("%album%", album)
            .replace("%duration%", &duration_str)
            .replace("%volume%", &volume_str);

        let display_text = display_text.trim();
        let final_text = if display_text.is_empty() {
            &args.stopped_label
        } else {
            display_text
        };

        // Build rich tooltip
        let song_line = if !artist.is_empty() && artist != "N/A" && artist != "n/a" {
            format!("{title} - {artist}")
        } else {
            title
        };

        let mut tooltip_lines = vec![song_line];
        if !album.is_empty() && album != "N/A" && album != "n/a" {
            tooltip_lines.push(format!("Album: {album}"));
        }

        match (!duration_str.is_empty(), status.volume) {
            (true, Some(vol)) => tooltip_lines.push(format!("Thời lượng: {duration_str}  •  Âm lượng: {vol}%")),
            (true, None) => tooltip_lines.push(format!("Thời lượng: {duration_str}")),
            (false, Some(vol)) => tooltip_lines.push(format!("Âm lượng: {vol}%")),
            (false, None) => {}
        }

        let tooltip = tooltip_lines.join("\n");
        let class = status.state.as_class_str();

        let output = WaybarOutput {
            text: final_text,
            tooltip: &tooltip,
            class,
            alt: "",
        };

        serde_json::to_string(&output).unwrap_or_else(|_| {
            format!(
                "{{\"text\": \"{}\", \"tooltip\": \"{}\", \"class\": \"{}\", \"alt\": \"\"}}",
                final_text, tooltip, class
            )
        })
    }

    /// Formats the stopped / offline output string.
    pub fn format_stopped(&self, args: &Args) -> String {
        let output = WaybarOutput {
            text: &args.stopped_label,
            tooltip: "Đã dừng",
            class: "stopped",
            alt: "",
        };
        serde_json::to_string(&output).unwrap_or_else(|_| {
            format!(
                "{{\"text\": \"{}\", \"tooltip\": \"Đã dừng\", \"class\": \"stopped\", \"alt\": \"\"}}",
                args.stopped_label
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_to_formatted_time() {
        assert_eq!(seconds_to_formatted_time(0), "00:00");
        assert_eq!(seconds_to_formatted_time(65), "01:05");
        assert_eq!(seconds_to_formatted_time(3599), "59:59");
        assert_eq!(seconds_to_formatted_time(3600), "01:00:00");
        assert_eq!(seconds_to_formatted_time(3665), "01:01:05");
        assert_eq!(seconds_to_formatted_time(7325), "02:02:05");
    }

    #[test]
    fn test_display_format_stopped() {
        let display = WaybarDisplay::new();
        let args = Args {
            host: "127.0.0.1".into(),
            port: 6600,
            format: "%artist% - %title%".into(),
            play_icon: "play".into(),
            pause_icon: "pause".into(),
            stopped_icon: "stop".into(),
            stopped_label: " mpd".into(),
            debug: false,
        };

        let status = MpdStatus {
            state: MpdState::Stopped,
            volume: None,
            duration_seconds: None,
        };
        let song = MpdSong::default();

        let json_str = display.format_output(&args, &status, &song);
        assert!(json_str.contains(" mpd"));
        assert!(json_str.contains("\"class\":\"stopped\""));
    }

    #[test]
    fn test_display_format_playing() {
        let display = WaybarDisplay::new();
        let args = Args {
            host: "127.0.0.1".into(),
            port: 6600,
            format: "[ %icon% ] %artist% - %title%".into(),
            play_icon: "".into(),
            pause_icon: "".into(),
            stopped_icon: "".into(),
            stopped_label: " mpd".into(),
            debug: false,
        };

        let status = MpdStatus {
            state: MpdState::Playing,
            volume: Some(85),
            duration_seconds: Some(215),
        };
        let song = MpdSong {
            title: Some("Song Title".into()),
            artist: Some("Artist Name".into()),
            album: Some("Album Name".into()),
            file: None,
            duration_seconds: Some(215),
        };

        let json_str = display.format_output(&args, &status, &song);
        assert!(json_str.contains("Artist Name - Song Title"));
        assert!(json_str.contains("\"class\":\"playing\""));
        assert!(json_str.contains("85%"));
    }
}
