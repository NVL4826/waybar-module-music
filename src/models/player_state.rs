use bincode::{Decode, Encode};

use crate::models::playback_state::PlaybackState;

use super::{mpris_metadata::MprisMetadata, mpris_playback::MprisPlayback};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub struct PlayerState {
    pub player_id: String,
    pub player_name: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub playing: Option<PlaybackState>,
    pub length: u64,
    pub position: u128,
}

impl PlayerState {
    pub fn from_mpris_data(
        player_name: String,
        metadata: MprisMetadata,
        playback: Option<MprisPlayback>,
        position: u128,
    ) -> Option<Self> {
        let player_id = metadata.player_id;
        if player_id.is_empty() {
            return None;
        }

        let artist = metadata.artist.first().cloned().unwrap_or_default();
        let album = metadata.album.unwrap_or_default();
        let title = metadata.title.unwrap_or_else(|| {
            if !player_name.is_empty() {
                player_name.clone()
            } else {
                "mpd".to_string()
            }
        });
        let playing = playback.unwrap_or_default().playing;
        let length = metadata.length.unwrap_or(0);

        Some(PlayerState {
            player_id,
            player_name,
            artist,
            album,
            title,
            playing,
            length,
            position,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_state_from_mpris_data() {
        let mut metadata = MprisMetadata::new("org.mpris.MediaPlayer2.mpd".to_string());
        metadata.artist = vec!["Test Artist".to_string()];
        metadata.album = Some("Test Album".to_string());
        metadata.title = Some("Test Title".to_string());
        metadata.length = Some(180_000_000);

        let playback = Some(MprisPlayback {
            player_id: "org.mpris.MediaPlayer2.mpd".to_string(),
            playing: Some(PlaybackState::Playing),
        });

        let state = PlayerState::from_mpris_data(
            "mpd".to_string(),
            metadata,
            playback,
            50_000_000,
        );

        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.player_name, "mpd");
        assert_eq!(state.artist, "Test Artist");
        assert_eq!(state.album, "Test Album");
        assert_eq!(state.title, "Test Title");
        assert_eq!(state.playing, Some(PlaybackState::Playing));
        assert_eq!(state.length, 180_000_000);
        assert_eq!(state.position, 50_000_000);
    }

    #[test]
    fn test_player_state_missing_fields() {
        let metadata = MprisMetadata::new("org.mpris.MediaPlayer2.mpd".to_string());
        let state = PlayerState::from_mpris_data("mpd".to_string(), metadata, None, 0);
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.player_name, "mpd");
        assert_eq!(state.title, "mpd");
        assert_eq!(state.artist, "");
        assert_eq!(state.album, "");

        let empty_id_meta = MprisMetadata::new(String::new());
        assert!(PlayerState::from_mpris_data("mpd".to_string(), empty_id_meta, None, 0).is_none());
    }
}
