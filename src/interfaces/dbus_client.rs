use std::error::Error;
use std::sync::Mutex;
use std::time::Duration;

use dbus::{
    arg::PropMap,
    blocking::{stdintf::org_freedesktop_dbus::Properties, Connection},
};

use crate::models::{
    mpris_metadata::MprisMetadata, mpris_playback::MprisPlayback, playback_state::PlaybackState,
};

pub struct DBusClient {
    conn: Mutex<Connection>,
}

impl Default for DBusClient {
    fn default() -> Self {
        Self {
            conn: Mutex::new(
                Connection::new_session().expect("failed to create DBus connection"),
            ),
        }
    }
}

impl DBusClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_players(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(5000));

        let (names,): (Vec<String>,) =
            proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;

        let players: Vec<String> = names
            .into_iter()
            .filter(|name| name.contains("org.mpris.MediaPlayer2"))
            .collect();

        Ok(players)
    }

    pub fn query_playback_status(&self, player_id: &str) -> Result<MprisPlayback, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let proxy = conn.with_proxy(
            player_id,
            "/org/mpris/MediaPlayer2",
            Duration::from_millis(5000),
        );
        let result: String = proxy.get("org.mpris.MediaPlayer2.Player", "PlaybackStatus")?;
        Ok(MprisPlayback::new_with_playing(
            player_id.to_string(),
            PlaybackState::from_string(&result),
        ))
    }

    pub fn query_metadata(&self, player_id: &str) -> Result<MprisMetadata, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let proxy = conn.with_proxy(
            player_id,
            "/org/mpris/MediaPlayer2",
            Duration::from_millis(5000),
        );
        let result: PropMap = proxy.get("org.mpris.MediaPlayer2.Player", "Metadata")?;

        Ok(MprisMetadata::from_dbus_propmap(
            player_id.to_string(),
            result,
        ))
    }

    pub fn query_mediaplayer_identity(&self, player_id: &str) -> Result<String, Box<dyn Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let proxy = conn.with_proxy(
            player_id,
            "/org/mpris/MediaPlayer2",
            Duration::from_millis(5000),
        );
        let identity: String = proxy.get("org.mpris.MediaPlayer2", "Identity")?;

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_dbus_client_is_send_and_sync() {
        assert_send_sync::<DBusClient>();
    }
}

