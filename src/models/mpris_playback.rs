use dbus::Message;

use crate::models::playback_state::PlaybackState;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MprisPlayback {
    pub player_id: String,
    pub playing: Option<PlaybackState>,
}

impl MprisPlayback {
    pub fn new(player_id: String) -> Self {
        Self {
            player_id,
            playing: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing == Some(PlaybackState::Playing)
    }

    pub fn new_with_playing(player_id: String, playing: Option<PlaybackState>) -> Self {
        Self { player_id, playing }
    }

    pub fn from_dbus_message(msg: &Message) -> Self {
        let sender = msg.sender().map(|s| s.to_string()).unwrap_or_default();
        let mut result = MprisPlayback::new(sender);

        for elem in msg.iter_init() {
            if let Some(dict) = elem.as_iter() {
                let mut iter = dict;
                while let Some(k) = iter.next() {
                    let v = match iter.next() {
                        Some(v) => v,
                        None => break,
                    };
                    if let (Some(key), Some(value)) = (k.as_str(), v.as_str()) {
                        if key == "PlaybackStatus" {
                            result.playing = PlaybackState::from_string(value);
                            return result;
                        }
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_from_message_without_sender_does_not_panic() {
        let msg = Message::new_signal(
            "/org/mpris/MediaPlayer2",
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
        )
        .unwrap();

        let playback = MprisPlayback::from_dbus_message(&msg);
        assert_eq!(playback.player_id, "");
        assert_eq!(playback.playing, None);
    }
}

