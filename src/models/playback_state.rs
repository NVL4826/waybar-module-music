use std::fmt;
use std::str::FromStr;

use bincode::{Decode, Encode};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackState {
    pub fn from_string(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl FromStr for PlaybackState {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_lowercase().as_str() {
            "playing" => Ok(PlaybackState::Playing),
            "paused" => Ok(PlaybackState::Paused),
            "stopped" => Ok(PlaybackState::Stopped),
            _ => Err(()),
        }
    }
}

impl fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaybackState::Playing => write!(f, "playing"),
            PlaybackState::Paused => write!(f, "paused"),
            PlaybackState::Stopped => write!(f, "stopped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_state_from_str() {
        assert_eq!("Playing".parse::<PlaybackState>(), Ok(PlaybackState::Playing));
        assert_eq!("PAUSED".parse::<PlaybackState>(), Ok(PlaybackState::Paused));
        assert_eq!("stopped".parse::<PlaybackState>(), Ok(PlaybackState::Stopped));
        assert!("unknown".parse::<PlaybackState>().is_err());
    }

    #[test]
    fn test_playback_state_display() {
        assert_eq!(PlaybackState::Playing.to_string(), "playing");
        assert_eq!(PlaybackState::Paused.to_string(), "paused");
        assert_eq!(PlaybackState::Stopped.to_string(), "stopped");
    }
}
