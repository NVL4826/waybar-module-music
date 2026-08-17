use dbus::{
    arg::{PropMap, RefArg, Variant},
    Message,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MprisMetadata {
    pub player_id: String,
    pub artist: Vec<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub length: Option<u64>,
}

impl MprisMetadata {
    pub fn new(sender: String) -> Self {
        Self {
            player_id: sender,
            artist: vec![],
            title: None,
            album: None,
            length: None,
        }
    }

    fn refarg_to_vec_string(value: Variant<Box<dyn RefArg>>) -> Vec<String> {
        let mut result = vec![];

        if let Some(iter) = value.as_iter() {
            for e in iter {
                if let Some(s) = e.as_str() {
                    result.push(s.to_string());
                } else if let Some(inner) = e.as_iter() {
                    for inner_e in inner {
                        if let Some(s) = inner_e.as_str() {
                            result.push(s.to_string());
                        }
                    }
                }
            }
        }

        result
    }

    fn refarg_to_string(value: Variant<Box<dyn RefArg>>) -> Option<String> {
        value.as_str().map(|elem| elem.to_string())
    }

    fn set_field(&mut self, key: &str, value: Variant<Box<dyn RefArg>>) {
        match key {
            "xesam:title" => self.title = MprisMetadata::refarg_to_string(value),
            "xesam:artist" => self.artist = MprisMetadata::refarg_to_vec_string(value),
            "xesam:album" => self.album = MprisMetadata::refarg_to_string(value),
            "mpris:length" => self.length = value.as_i64().map(|elem| elem as u64),
            _ => (),
        }
    }

    pub fn from_dbus_message(msg: &Message) -> Self {
        let sender = msg.sender().map(|s| s.to_string()).unwrap_or_default();
        let mut result = MprisMetadata::new(sender);

        for msg_arg in msg.iter_init() {
            if let Some(dict) = msg_arg.as_iter() {
                let mut iter = dict;
                while let Some(key_arg) = iter.next() {
                    let val_arg = match iter.next() {
                        Some(v) => v,
                        None => break,
                    };
                    if key_arg.as_str() != Some("Metadata") {
                        continue;
                    }

                    if let Some(metadata_dict) = val_arg.as_iter() {
                        for m in metadata_dict {
                            if let Some(entry_iter) = m.as_iter() {
                                let mut entry = entry_iter;
                                if let (Some(meta_k), Some(meta_v)) = (entry.next(), entry.next()) {
                                    if let Some(meta_key) = meta_k.as_str() {
                                        result.set_field(
                                            meta_key,
                                            Variant(meta_v.box_clone()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    pub fn from_dbus_propmap(player_id: String, map: PropMap) -> Self {
        let mut result = MprisMetadata::new(player_id);
        for (key, value) in map {
            result.set_field(&key, Variant(value.box_clone()));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_from_message_without_sender_does_not_panic() {
        let msg = Message::new_signal(
            "/org/mpris/MediaPlayer2",
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
        )
        .unwrap();

        // sender() is None
        let meta = MprisMetadata::from_dbus_message(&msg);
        assert_eq!(meta.player_id, "");
        assert_eq!(meta.title, None);
    }
}

