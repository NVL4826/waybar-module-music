use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "waybar-module-music",
    about = "Lightweight MPD music module for Waybar",
    version
)]
pub struct Args {
    /// MPD server host
    #[arg(short = 'H', long, default_value = "127.0.0.1", env = "MPD_HOST")]
    pub host: String,

    /// MPD server port
    #[arg(short = 'p', long, default_value_t = 6600, env = "MPD_PORT")]
    pub port: u16,

    /// Format template string. Placeholders: %icon%, %title%, %artist%, %album%, %duration%, %volume%
    #[arg(short = 'f', long, default_value = "[ %icon% ] %artist% - %title%")]
    pub format: String,

    /// Play icon
    #[arg(long, default_value = "")]
    pub play_icon: String,

    /// Pause icon
    #[arg(long, default_value = "")]
    pub pause_icon: String,

    /// Stopped icon
    #[arg(long, default_value = "")]
    pub stopped_icon: String,

    /// Text to display when MPD is stopped or offline
    #[arg(short = 's', long, default_value = " mpd")]
    pub stopped_label: String,

    /// Enable debug logging
    #[arg(long, default_value_t = false)]
    pub debug: bool,
}

