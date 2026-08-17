use log::info;

use crate::{
    interfaces::dbus_client::DBusClient,
    models::{
        mpris_event::MprisEvent, mpris_metadata::MprisMetadata, mpris_playback::MprisPlayback,
        mpris_seeked::MprisSeeked, player_client::PlayerClient, player_state::PlayerState,
    },
    services::runnable::Runnable,
};
use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct PlayerManager {
    dbus_client: Arc<DBusClient>,
    event_rx: Mutex<Option<Receiver<MprisEvent>>>,
    display_tx: Sender<PlayerState>,
}

impl PlayerManager {
    pub fn new(
        event_rx: Receiver<MprisEvent>,
        display_tx: Sender<PlayerState>,
        dbus_client: Arc<DBusClient>,
    ) -> Self {
        Self {
            dbus_client,
            event_rx: Mutex::new(Some(event_rx)),
            display_tx,
        }
    }

    fn init_worker(&self) {
        let event_rx = match self.event_rx.lock().unwrap().take() {
            Some(rx) => rx,
            None => return,
        };

        let mut players: HashMap<String, PlayerClient> = HashMap::new();

        // Discover active players on startup
        if let Ok(player_ids) = self.dbus_client.get_players() {
            for id in player_ids {
                let metadata = self
                    .dbus_client
                    .query_metadata(&id)
                    .unwrap_or_else(|_| MprisMetadata::new(id.clone()));
                let identity = self
                    .dbus_client
                    .query_mediaplayer_identity(&id)
                    .unwrap_or_else(|_| "MPD".to_string());
                let mut client = PlayerClient::new(identity, metadata);

                if let Ok(playback) = self.dbus_client.query_playback_status(&id) {
                    client.update_playback_state(playback);
                }
                players.insert(id, client);
            }

            if let Some(active) = self.get_most_active_player(&players) {
                self.publish_player_state(&active);
            }
        }

        loop {
            let is_any_playing = players.values().any(|p| p.playing());
            let msg = if is_any_playing {
                match event_rx.recv_timeout(Duration::from_millis(1000)) {
                    Ok(msg) => Some(msg),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        for player in players.values_mut().filter(|p| p.playing()) {
                            player.update_position(player.position() + 1_000_000);
                        }
                        if let Some(active) = self.get_most_active_player(&players) {
                            self.publish_player_state(&active);
                        }
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match event_rx.recv() {
                    Ok(msg) => Some(msg),
                    Err(_) => break,
                }
            };

            if let Some(msg) = msg {
                match msg {
                    MprisEvent::Metadata(metadata) => {
                        self.handle_metadata_event(&mut players, metadata);
                    }
                    MprisEvent::Playback(playback) => {
                        self.handle_playback_event(&mut players, playback);
                    }
                    MprisEvent::Seeked(seeked) => {
                        self.handle_seeked_event(&mut players, seeked);
                    }
                }
            }
        }
    }

    fn handle_metadata_event(
        &self,
        players: &mut HashMap<String, PlayerClient>,
        mpris_metadata: MprisMetadata,
    ) {
        let id = mpris_metadata.player_id.clone();
        if id.is_empty() {
            return;
        }

        if let Some(player) = players.get_mut(&id) {
            player.update_metadata(mpris_metadata);
        } else {
            let identity = self
                .dbus_client
                .query_mediaplayer_identity(&id)
                .unwrap_or_else(|_| "MPD".to_string());
            players.insert(id.clone(), PlayerClient::new(identity, mpris_metadata));
        }

        if let Some(player) = players.get(&id) {
            self.publish_player_state(player);
        }
    }

    fn handle_playback_event(
        &self,
        players: &mut HashMap<String, PlayerClient>,
        mpris_playback: MprisPlayback,
    ) {
        let id = mpris_playback.player_id.clone();
        self.query_player_if_not_exists(players, &id);

        if let Some(player) = players.get_mut(&id) {
            if let Ok(metadata) = self.dbus_client.query_metadata(&id) {
                player.update_metadata(metadata);
            }
            player.update_playback_state(mpris_playback);
        }

        if let Some(active) = self.get_most_active_player(players) {
            self.publish_player_state(&active);
        }
    }

    fn handle_seeked_event(
        &self,
        players: &mut HashMap<String, PlayerClient>,
        mpris_seeked: MprisSeeked,
    ) {
        let id = &mpris_seeked.player_id;
        self.query_player_if_not_exists(players, id);

        if let Some(player) = players.get_mut(id) {
            player.update_position(mpris_seeked.position);
        }

        if let Some(player) = players.get(id) {
            self.publish_player_state(player);
        }
    }

    fn query_player_if_not_exists(&self, players: &mut HashMap<String, PlayerClient>, id: &str) {
        if !players.contains_key(id) {
            let metadata = self
                .dbus_client
                .query_metadata(id)
                .unwrap_or_else(|_| MprisMetadata::new(id.to_string()));
            let identity = self
                .dbus_client
                .query_mediaplayer_identity(id)
                .unwrap_or_else(|_| "MPD".to_string());
            players.insert(id.to_owned(), PlayerClient::new(identity, metadata));
        }
    }

    fn get_most_active_player(&self, players: &HashMap<String, PlayerClient>) -> Option<PlayerClient> {
        players
            .values()
            .filter(|p| p.playing())
            .max_by_key(|p| p.last_updated)
            .cloned()
            .or_else(|| players.values().max_by_key(|p| p.last_updated).cloned())
    }

    pub fn publish_player_state(&self, player: &PlayerClient) {
        if let Some(state) = PlayerState::from_mpris_data(
            player.name(),
            player.metadata(),
            player.playback_state(),
            player.position(),
        ) {
            let _ = self.display_tx.send(state);
        }
    }
}


impl Runnable for PlayerManager {
    fn run(self: Arc<Self>) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("starting PlayerManager thread");
            self.init_worker();
            info!("PlayerManager thread is stopping");
        })
    }
}
