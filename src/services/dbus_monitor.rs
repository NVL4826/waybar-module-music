use std::{
    sync::{mpsc::Sender, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};

use dbus::{arg::RefArg, blocking::Connection, message::MatchRule, Message};
use log::{debug, error, info, warn};

use crate::{
    interfaces::dbus_client::DBusClient,
    models::{
        args::Args, mpris_event::MprisEvent, mpris_metadata::MprisMetadata,
        mpris_playback::MprisPlayback, mpris_seeked::MprisSeeked,
    },
};

use super::runnable::Runnable;

pub struct DBusMonitor {
    args: Arc<Args>,
    event_tx: Sender<MprisEvent>,
    dbus_client: Arc<DBusClient>,
}

impl DBusMonitor {
    pub fn new(args: Arc<Args>, event_tx: Sender<MprisEvent>, dbus_client: Arc<DBusClient>) -> Self {
        Self {
            args,
            event_tx,
            dbus_client,
        }
    }

    fn get_signal_property_keys(msg: &Message) -> Vec<String> {
        let mut result = vec![];
        for elem in msg.iter_init() {
            if let Some(args) = elem.as_iter() {
                for arg in args {
                    if let Some(arg_str) = arg.as_str() {
                        result.push(String::from(arg_str));
                    }
                }
            };
        }
        result
    }

    fn should_handle_sender(args: &Args, dbus_client: &DBusClient, msg: &Message) -> bool {
        if args.whitelist.is_empty() {
            return true;
        }

        let sender = match msg.sender() {
            Some(sender) => sender.to_string(),
            None => return true,
        };

        match dbus_client.query_mediaplayer_identity(&sender) {
            Ok(identity) => args
                .whitelist
                .iter()
                .any(|w| identity.to_lowercase().contains(&w.to_lowercase())),
            Err(_) => true,
        }
    }

    fn handle_on_match(
        args: &Args,
        dbus_client: &DBusClient,
        msg: &Message,
        event_tx: &Sender<MprisEvent>,
    ) -> bool {
        if !DBusMonitor::should_handle_sender(args, dbus_client, msg) {
            debug!("ignoring sender, not in whitelist");
            return true;
        }

        debug!("dbus_monitor msg: {:?}", msg);
        let mut property_keys = DBusMonitor::get_signal_property_keys(msg);
        if let Some(member) = msg.member() {
            property_keys.push(member.to_string());
        }

        for key in property_keys {
            let event = match key.to_lowercase().as_str() {
                "metadata" => Some(MprisEvent::Metadata(MprisMetadata::from_dbus_message(msg))),
                "playbackstatus" => {
                    Some(MprisEvent::Playback(MprisPlayback::from_dbus_message(msg)))
                }
                "seeked" => Some(MprisEvent::Seeked(MprisSeeked::from_dbus_message(msg))),
                _ => None,
            };

            if let Some(event) = event {
                let _ = event_tx.send(event);
            }
        }

        true
    }

    pub fn begin_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::new_session()?;

        let rules: Vec<MatchRule> = vec![
            MatchRule::new()
                .with_type(dbus::MessageType::Signal)
                .with_path("/org/mpris/MediaPlayer2")
                .with_interface("org.freedesktop.DBus.Properties")
                .with_member("PropertiesChanged"),
            MatchRule::new()
                .with_type(dbus::MessageType::Signal)
                .with_path("/org/mpris/MediaPlayer2")
                .with_interface("org.mpris.MediaPlayer2.Player")
                .with_member("Seeked"),
        ];

        for rule in rules {
            let event_tx = self.event_tx.clone();
            let dbus_client = self.dbus_client.clone();
            let args = self.args.clone();
            match conn.add_match(rule, move |_: (), _, msg| {
                DBusMonitor::handle_on_match(&args, &dbus_client, msg, &event_tx)
            }) {
                Ok(token) => token,
                Err(err) => {
                    error!("DBusMonitor was unable to monitor MPRIS players: {err}");
                    return Err(err.into());
                }
            };
        }

        loop {
            if let Err(err) = conn.process(Duration::from_millis(1000)) {
                warn!("failed to process DBus connection: {err}");
            }
        }
    }
}

impl Runnable for DBusMonitor {
    fn run(self: Arc<Self>) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("starting DBusMonitor thread");
            if let Err(err) = self.begin_monitoring() {
                error!("DBusMonitor thread failed: {err}");
            }
            info!("DBusMonitor thread is stopping");
        })
    }
}
