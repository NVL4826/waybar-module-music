use bincode::config;
use log::{debug, error, info, warn};

use crate::{
    effects::{ellipsis::Ellipsis, marquee::Marquee, text_effect::TextEffect},
    event_bus::{EventBusHandle, EventType},
    models::{
        args::Args, config::Config, playback_state::PlaybackState, player_state::PlayerState,
    },
    utils::time,
};

use super::runnable::Runnable;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

#[derive(Debug)]
enum DisplayMessages {
    PlayerStateChanged(PlayerState),
    AnimationDue,
}

#[derive(serde::Serialize)]
struct WaybarOutput<'a> {
    text: &'a str,
    tooltip: &'a str,
    class: &'a str,
    alt: &'a str,
}

pub struct Display {
    args: Arc<Args>,
    config: Arc<Config>,
    event_bus: EventBusHandle,
}

impl Display {
    pub fn new(args: Arc<Args>, config: Arc<Config>, event_bus: EventBusHandle) -> Self {
        Self {
            args,
            config,
            event_bus,
        }
    }

    fn init_worker(self: Arc<Self>) {
        println!(
            "{}",
            self.format_json_output(
                self.get_stopped_label(),
                "trạng thái: đã dừng\nchuột trái: chuyển playlist",
                "stopped"
            )
        );

        let (tx, rx) = mpsc::channel();
        let (effect_tx, effect_rx) = mpsc::channel();

        if let Some(rx) = self.event_bus.subscribe(EventType::PlayerStateChanged) {
            let tx = tx.clone();
            thread::spawn(move || {
                Display::listen_player_state(rx, tx);
            });
        } else {
            error!("failed to subscribe to PlayerStateChanged listener");
        }

        {
            let tx = tx.clone();
            let effect_speed = self.args.effect_speed as u64;
            thread::spawn(move || {
                Display::text_effect_timer(effect_speed, effect_rx, tx);
            });
        }

        self.listen_for_updates(rx, effect_tx, self.init_fields());
    }

    fn init_fields(&self) -> HashMap<&str, TextEffect> {
        let mut fields = HashMap::new();

        // FIXME: I'm sure this could be done better
        if self.args.marquee {
            fields.insert(
                "title",
                TextEffect::new().with_effect(Box::new(Marquee::new(
                    self.args.title_width,
                    self.args.delay_marquee as u16,
                ))),
            );

            fields.insert(
                "artist",
                TextEffect::new().with_effect(Box::new(Marquee::new(
                    self.args.artist_width,
                    self.args.delay_marquee as u16,
                ))),
            );
        } else if self.args.ellipsis {
            fields.insert(
                "title",
                TextEffect::new().with_effect(Box::new(Ellipsis::new(self.args.title_width))),
            );

            fields.insert(
                "artist",
                TextEffect::new().with_effect(Box::new(Ellipsis::new(self.args.artist_width))),
            );
        } else {
            fields.insert("title", TextEffect::new());
            fields.insert("artist", TextEffect::new());
        }

        fields.insert("album", TextEffect::new());
        fields.insert("player", TextEffect::new());
        fields.insert("player-icon", TextEffect::new());
        fields.insert("position", TextEffect::new());
        fields.insert("length", TextEffect::new());

        fields
    }

    fn text_effect_timer(interval_ms: u64, effect_rx: Receiver<bool>, tx: Sender<DisplayMessages>) {
        let mut active_effects = false;
        loop {
            if active_effects {
                thread::sleep(Duration::from_millis(interval_ms));
                if let Err(err) = tx.send(DisplayMessages::AnimationDue) {
                    warn!("failed to send AnimationDue message: {err}");
                }
                active_effects = match effect_rx.try_recv() {
                    Ok(msg) => msg,
                    Err(_) => active_effects,
                };
            } else {
                debug!("waiting for effect trigger to continue effect timer");
                active_effects = match effect_rx.recv() {
                    Ok(msg) => msg,
                    Err(err) => {
                        error!("failed to receieve effect message: {err}");
                        false
                    }
                };
                debug!("got effect trigger message: {active_effects}");
            }
        }
    }

    fn listen_player_state(rx: Receiver<Vec<u8>>, tx: Sender<DisplayMessages>) {
        loop {
            let msg = rx.recv();
            let (state, _): (PlayerState, usize) = match msg {
                Ok(encoded) => {
                    bincode::decode_from_slice(&encoded[..], config::standard()).unwrap()
                }
                Err(err) => {
                    warn!("failed to decode message in Display: {err}");
                    continue;
                }
            };

            if let Err(err) = tx.send(DisplayMessages::PlayerStateChanged(state)) {
                warn!("failed to send DisplayMessages: {err}");
            }
        }
    }

    fn set_text_effect_field(fields: &mut HashMap<&str, TextEffect>, value: &str, field: &str) {
        match fields.get_mut(field) {
            Some(field) => {
                if field.current_text() != value {
                    field.set_effect_text(value.to_string());
                    field.override_last_drawn(value.to_string());
                }
            }
            None => error!("failed to get '{field}' field"),
        }
    }

    fn should_effects_be_redrawn(&self, fields: &HashMap<&str, TextEffect>) -> bool {
        fields.iter().any(|(_, v)| v.has_active_effects())
    }

    fn listen_for_updates(
        &self,
        rx: Receiver<DisplayMessages>,
        effect_tx: Sender<bool>,
        mut fields: HashMap<&str, TextEffect>,
    ) {
        let mut player_state: Option<PlayerState> = None;

        loop {
            let msg = match rx.recv() {
                Ok(msg) => msg,
                Err(err) => {
                    warn!("failed to recieve message: {err}");
                    continue;
                }
            };

            debug!("msg receieved: {:?}", msg);

            match msg {
                DisplayMessages::PlayerStateChanged(state) => {
                    Display::set_text_effect_field(&mut fields, &state.title, "title");
                    Display::set_text_effect_field(&mut fields, &state.artist, "artist");
                    Display::set_text_effect_field(&mut fields, &state.album, "album");
                    Display::set_text_effect_field(&mut fields, &state.player_name, "player");
                    Display::set_text_effect_field(
                        &mut fields,
                        &time::microseconds_to_formatted_time(state.length as u128),
                        "length",
                    );
                    Display::set_text_effect_field(
                        &mut fields,
                        &time::microseconds_to_formatted_time(state.position),
                        "position",
                    );
                    Display::set_text_effect_field(
                        &mut fields,
                        self.config
                            .get_player_icon_by_partial_match(&state.player_name),
                        "player-icon",
                    );
                    player_state = Some(state);
                    self.draw(&player_state, &mut fields);
                    if let Err(err) = effect_tx.send(self.should_effects_be_redrawn(&fields)) {
                        error!("failed to notify effects thread: {err}");
                    }
                }
                DisplayMessages::AnimationDue => {
                    if self.should_effects_be_redrawn(&fields) {
                        fields.iter_mut().for_each(|(_, v)| {
                            v.should_redraw();
                        });
                        self.draw(&player_state, &mut fields)
                    }
                }
            }
        }
    }

    fn get_class(&self, state: &PlayerState) -> String {
        if let Some(playing) = state.playing {
            playing.to_string()
        } else {
            String::from("stopped")
        }
    }

    /// Create the final output JSON, in the format that Waybar expects
    fn format_json_output(&self, text: &str, tooltip: &str, class: &str) -> String {
        let output = WaybarOutput {
            text,
            tooltip,
            class,
            alt: "",
        };
        serde_json::to_string(&output).unwrap_or_else(|_| {
            format!(
                "{{\"text\": \"{}\", \"tooltip\": \"{}\", \"class\": \"{}\", \"alt\": \"\"}}",
                text, tooltip, class
            )
        })
    }

    fn populate_using_placeholders(
        &self,
        player_state: &PlayerState,
        fields: &mut HashMap<&str, TextEffect>,
    ) -> String {
        let replacements: HashMap<&str, String> = [
            (
                "icon",
                match player_state
                    .playing
                    .unwrap_or(PlaybackState::Stopped)
                {
                    PlaybackState::Playing => self.args.play_icon.clone(),
                    PlaybackState::Paused => self.args.pause_icon.clone(),
                    PlaybackState::Stopped => self.args.pause_icon.clone(),
                },
            ),
            (
                "title",
                {
                    let title_text = if player_state.title.trim().is_empty() {
                        if !player_state.player_name.trim().is_empty() {
                            player_state.player_name.clone()
                        } else {
                            "mpd".to_string()
                        }
                    } else {
                        player_state.title.clone()
                    };
                    fields.get_mut("title").unwrap().draw(&title_text)
                },
            ),
            (
                "artist",
                fields.get_mut("artist").unwrap().draw(&player_state.artist),
            ),
            (
                "album",
                fields.get_mut("album").unwrap().draw(&player_state.album),
            ),
            (
                "player",
                fields
                    .get_mut("player")
                    .unwrap()
                    .draw(&player_state.player_name),
            ),
            (
                "player-icon",
                fields.get_mut("player-icon").unwrap().draw(
                    self.config
                        .get_player_icon_by_partial_match(&player_state.player_name),
                ),
            ),
            (
                "length",
                fields
                    .get_mut("length")
                    .unwrap()
                    .draw(&time::microseconds_to_formatted_time(
                        player_state.length as u128,
                    )),
            ),
            (
                "position",
                fields
                    .get_mut("position")
                    .unwrap()
                    .draw(&time::microseconds_to_formatted_time(player_state.position)),
            ),
        ]
        .into_iter()
        .collect();

        replacements
            .iter()
            .fold(self.args.format.clone(), |acc, (key, value)| {
                acc.replace(&format!("%{key}%"), value)
            })
    }

    fn get_stopped_label(&self) -> &str {
        if self.args.stopped_label.trim().is_empty() {
            " mpd"
        } else {
            &self.args.stopped_label
        }
    }

    fn draw(&self, player_state: &Option<PlayerState>, fields: &mut HashMap<&str, TextEffect>) {
        let player_state = match player_state {
            Some(state) => state,
            None => {
                println!(
                    "{}",
                    self.format_json_output(
                        self.get_stopped_label(),
                        "trạng thái: đã dừng\nchuột trái: chuyển playlist",
                        "stopped"
                    )
                );
                return;
            }
        };

        if player_state
            .playing
            .is_some_and(|playback| playback == PlaybackState::Stopped)
            || player_state.playing.is_none()
        {
            println!(
                "{}",
                self.format_json_output(
                    self.get_stopped_label(),
                    "trạng thái: đã dừng\nchuột trái: chuyển playlist",
                    "stopped"
                )
            );
            return;
        }

        let state_str = match player_state.playing {
            Some(PlaybackState::Playing) => "đang phát",
            Some(PlaybackState::Paused) => "tạm dừng",
            _ => "đã dừng",
        };
        let duration_str = if player_state.length > 0 {
            format!(
                "{}/{}",
                time::microseconds_to_formatted_time(player_state.position),
                time::microseconds_to_formatted_time(player_state.length as u128)
            )
        } else {
            "n/a".to_string()
        };
        let tooltip = format!(
            "bài hát: {}\nnghệ sĩ: {}\nalbum: {}\ntrình phát: {}\nthời lượng: {}\ntrạng thái: {}\nchuột trái: chuyển playlist",
            if player_state.title.is_empty() { "n/a" } else { &player_state.title },
            if player_state.artist.is_empty() { "n/a" } else { &player_state.artist },
            if player_state.album.is_empty() { "n/a" } else { &player_state.album },
            if player_state.player_name.is_empty() { "mpd" } else { &player_state.player_name },
            duration_str,
            state_str
        );

        let populated = self.populate_using_placeholders(player_state, fields);
        let display_text = if populated.trim().is_empty() {
            self.get_stopped_label()
        } else {
            populated.trim()
        };

        let output = self.format_json_output(
            display_text,
            &tooltip,
            &self.get_class(player_state),
        );

        println!("{}", output)
    }
}

impl Runnable for Display {
    fn run(self: Arc<Self>) -> std::thread::JoinHandle<()> {
        thread::spawn(move || {
            info!("starting Display thread");
            self.init_worker();
            info!("Display thread is stopping");
        })
    }
}
