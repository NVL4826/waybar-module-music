use std::io::Write;
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

/// Truncates text by character count, optionally appending ellipsis (...).
pub fn truncate_text(text: &str, max_width: usize, use_ellipsis: bool) -> String {
    if max_width == 0 {
        return text.to_string();
    }
    let char_count = text.chars().count();
    if char_count > max_width {
        if use_ellipsis && max_width > 3 {
            let truncated: String = text.chars().take(max_width - 3).collect();
            format!("{truncated}...")
        } else {
            text.chars().take(max_width).collect()
        }
    } else {
        text.to_string()
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

    /// Prints output to stdout if it changed from the previous output, immediately flushing stdout.
    pub fn print_if_changed(&self, output: &str) {
        let mut last = self
            .last_output
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *last != output {
            *last = output.to_string();
            println!("{output}");
            let _ = std::io::stdout().flush();
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

        let raw_title = song.display_title();
        let title = if args.title_width > 0 {
            truncate_text(&raw_title, args.title_width, args.ellipsis)
        } else {
            raw_title.clone()
        };

        let raw_artist = song.artist.as_deref().unwrap_or("").trim();
        let artist = if args.artist_width > 0 {
            truncate_text(raw_artist, args.artist_width, args.ellipsis)
        } else {
            raw_artist.to_string()
        };

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
            .replace("%artist%", &artist)
            .replace("%album%", album)
            .replace("%duration%", &duration_str)
            .replace("%volume%", &volume_str);

        // Sanitize text field: no newlines permitted on Waybar taskbar text
        let display_text = display_text.replace('\n', " ").replace('\r', "");
        let display_text = display_text.trim();

        let final_text = if display_text.is_empty() {
            args.stopped_label.clone()
        } else if args.max_length > 0 {
            truncate_text(display_text, args.max_length, args.ellipsis)
        } else {
            display_text.to_string()
        };

        // Build rich tooltip
        let song_line = if !raw_artist.is_empty() && raw_artist != "N/A" && raw_artist != "n/a" {
            format!("{raw_title} - {raw_artist}")
        } else {
            raw_title
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
            text: &final_text,
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
        let clean_label = args.stopped_label.replace('\n', " ").replace('\r', "");
        let output = WaybarOutput {
            text: &clean_label,
            tooltip: "Đã dừng",
            class: "stopped",
            alt: "",
        };
        serde_json::to_string(&output).unwrap_or_else(|_| {
            format!(
                "{{\"text\": \"{}\", \"tooltip\": \"Đã dừng\", \"class\": \"stopped\", \"alt\": \"\"}}",
                clean_label
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
    fn test_truncate_text() {
        assert_eq!(truncate_text("Hello World", 0, false), "Hello World");
        assert_eq!(truncate_text("Hello World", 5, false), "Hello");
        assert_eq!(truncate_text("Hello World", 8, true), "Hello...");
        assert_eq!(truncate_text("Tiếng Việt Có Dấu", 12, true), "Tiếng Việ...");
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
            title_width: 0,
            artist_width: 0,
            max_length: 0,
            ellipsis: false,
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
    fn test_display_format_playing_with_ellipsis() {
        let display = WaybarDisplay::new();
        let args = Args {
            host: "127.0.0.1".into(),
            port: 6600,
            format: " %title%".into(),
            play_icon: "".into(),
            pause_icon: "".into(),
            stopped_icon: "".into(),
            stopped_label: " mpd".into(),
            title_width: 15,
            artist_width: 0,
            max_length: 0,
            ellipsis: true,
            debug: false,
        };

        let status = MpdStatus {
            state: MpdState::Playing,
            volume: Some(85),
            duration_seconds: Some(215),
        };
        let song = MpdSong {
            title: Some("Very Long Song Title That Exceeds Width".into()),
            artist: Some("Artist Name".into()),
            album: Some("Album Name".into()),
            file: None,
            duration_seconds: Some(215),
        };

        let json_str = display.format_output(&args, &status, &song);
        assert!(json_str.contains("Very Long So..."));
        assert!(json_str.contains("\"class\":\"playing\""));
    }
}

