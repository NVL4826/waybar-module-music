use log::{debug, info};

use crate::{
    effects::{ellipsis::Ellipsis, marquee::Marquee, text_effect::TextEffect},
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
        Arc, Mutex,
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
    player_rx: Mutex<Option<Receiver<PlayerState>>>,
    last_output: Mutex<String>,
}

impl Display {
    pub fn new(args: Arc<Args>, config: Arc<Config>, player_rx: Receiver<PlayerState>) -> Self {
        Self {
            args,
            config,
            player_rx: Mutex::new(Some(player_rx)),
            last_output: Mutex::new(String::new()),
        }
    }

    fn print_if_changed(&self, output: String) {
        let mut last = self.last_output.lock().unwrap();
        if *last != output {
            *last = output.clone();
            println!("{output}");
        }
    }

    fn init_worker(self: Arc<Self>) {
        self.print_if_changed(self.format_json_output(
            self.get_stopped_label(),
            "Đã dừng",
            "stopped",
        ));

        let player_rx = match self.player_rx.lock().unwrap().take() {
            Some(rx) => rx,
            None => return,
        };

        let (tx, rx) = mpsc::channel();
        let (effect_tx, effect_rx) = mpsc::channel();

        // Forward player state changes
        let tx_state = tx.clone();
        thread::spawn(move || {
            while let Ok(state) = player_rx.recv() {
                if tx_state.send(DisplayMessages::PlayerStateChanged(state)).is_err() {
                    break;
                }
            }
        });

        // Only start effect timer thread if marquee animation is enabled
        if self.args.marquee {
            let tx_anim = tx;
            let effect_speed = self.args.effect_speed as u64;
            thread::spawn(move || {
                Display::text_effect_timer(effect_speed, effect_rx, tx_anim);
            });
        }

        self.listen_for_updates(rx, effect_tx, self.init_fields());
    }

    fn init_fields(&self) -> HashMap<&'static str, TextEffect> {
        let mut fields = HashMap::new();

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
                if tx.send(DisplayMessages::AnimationDue).is_err() {
                    break;
                }
                active_effects = match effect_rx.try_recv() {
                    Ok(msg) => msg,
                    Err(_) => active_effects,
                };
            } else {
                debug!("waiting for effect trigger to continue effect timer");
                active_effects = match effect_rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => break,
                };
            }
        }
    }

    fn set_text_effect_field(fields: &mut HashMap<&'static str, TextEffect>, value: &str, field: &'static str) {
        if let Some(field) = fields.get_mut(field) {
            if field.current_text() != value {
                field.set_effect_text(value.to_string());
                field.override_last_drawn(value.to_string());
            }
        }
    }

    fn should_effects_be_redrawn(&self, fields: &HashMap<&'static str, TextEffect>) -> bool {
        fields.iter().any(|(_, v)| v.has_active_effects())
    }

    fn listen_for_updates(
        &self,
        rx: Receiver<DisplayMessages>,
        effect_tx: Sender<bool>,
        mut fields: HashMap<&'static str, TextEffect>,
    ) {
        let mut player_state: Option<PlayerState> = None;

        while let Ok(msg) = rx.recv() {
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
                    if self.args.marquee {
                        let _ = effect_tx.send(self.should_effects_be_redrawn(&fields));
                    }
                }
                DisplayMessages::AnimationDue => {
                    if self.should_effects_be_redrawn(&fields) {
                        for (_, v) in fields.iter_mut() {
                            v.should_redraw();
                        }
                        self.draw(&player_state, &mut fields);
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
        fields: &mut HashMap<&'static str, TextEffect>,
    ) -> String {
        let title_text = if player_state.title.trim().is_empty() {
            if !player_state.player_name.trim().is_empty() {
                player_state.player_name.clone()
            } else {
                "MPD".to_string()
            }
        } else {
            player_state.title.clone()
        };

        let icon = match player_state.playing.unwrap_or(PlaybackState::Stopped) {
            PlaybackState::Playing => &self.args.play_icon,
            PlaybackState::Paused | PlaybackState::Stopped => &self.args.pause_icon,
        };

        let title = fields.get_mut("title").unwrap().draw(&title_text);
        let artist = fields.get_mut("artist").unwrap().draw(&player_state.artist);
        let album = fields.get_mut("album").unwrap().draw(&player_state.album);
        let player = fields.get_mut("player").unwrap().draw(&player_state.player_name);
        let player_icon = fields
            .get_mut("player-icon")
            .unwrap()
            .draw(self.config.get_player_icon_by_partial_match(&player_state.player_name));
        let length = fields
            .get_mut("length")
            .unwrap()
            .draw(&time::microseconds_to_formatted_time(player_state.length as u128));
        let position = fields
            .get_mut("position")
            .unwrap()
            .draw(&time::microseconds_to_formatted_time(player_state.position));

        self.args
            .format
            .replace("%icon%", icon)
            .replace("%title%", &title)
            .replace("%artist%", &artist)
            .replace("%album%", &album)
            .replace("%player%", &player)
            .replace("%player-icon%", &player_icon)
            .replace("%length%", &length)
            .replace("%position%", &position)
    }

    fn get_stopped_label(&self) -> &str {
        if self.args.stopped_label.trim().is_empty() {
            " mpd"
        } else {
            &self.args.stopped_label
        }
    }

    fn draw(&self, player_state: &Option<PlayerState>, fields: &mut HashMap<&'static str, TextEffect>) {
        let player_state = match player_state {
            Some(state) => state,
            None => {
                let output = self.format_json_output(
                    self.get_stopped_label(),
                    "Đã dừng",
                    "stopped",
                );
                self.print_if_changed(output);
                return;
            }
        };

        if player_state
            .playing
            .is_some_and(|playback| playback == PlaybackState::Stopped)
            || player_state.playing.is_none()
        {
            let output = self.format_json_output(
                self.get_stopped_label(),
                "Đã dừng",
                "stopped",
            );
            self.print_if_changed(output);
            return;
        }

        let title = player_state.title.trim();
        let artist = player_state.artist.trim();
        let song_line = if !title.is_empty() && !artist.is_empty() && artist != "N/A" && artist != "n/a" {
            format!("{title} - {artist}")
        } else if !title.is_empty() {
            title.to_string()
        } else if !artist.is_empty() && artist != "N/A" && artist != "n/a" {
            artist.to_string()
        } else {
            "Không có tiêu đề".to_string()
        };

        let duration_str = if player_state.length > 0 {
            if self.args.format.contains("%position%") {
                format!(
                    "{}/{}",
                    time::microseconds_to_formatted_time(player_state.position),
                    time::microseconds_to_formatted_time(player_state.length as u128)
                )
            } else {
                time::microseconds_to_formatted_time(player_state.length as u128)
            }
        } else {
            String::new()
        };

        let volume_opt = crate::utils::mpd::get_mpd_volume();

        let stats_line = match (duration_str.is_empty(), volume_opt) {
            (false, Some(vol)) => format!("Thời lượng: {duration_str}  •  Âm lượng: {vol}%"),
            (false, None) => format!("Thời lượng: {duration_str}"),
            (true, Some(vol)) => format!("Âm lượng: {vol}%"),
            (true, None) => String::new(),
        };

        let album = player_state.album.trim();
        let mut tooltip_lines = vec![song_line];

        if !album.is_empty() && album != "N/A" && album != "n/a" {
            tooltip_lines.push(format!("Album: {album}"));
        }

        if !stats_line.is_empty() {
            tooltip_lines.push(stats_line);
        }

        let tooltip = tooltip_lines.join("\n");

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

        self.print_if_changed(output);
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
