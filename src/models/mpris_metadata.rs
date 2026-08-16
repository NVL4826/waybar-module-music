use bincode::{Decode, Encode};
use dbus::{
    arg::{PropMap, RefArg, Variant},
    Message,
};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
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
        let mut result = MprisMetadata::new(msg.sender().unwrap().to_string());

        // FIXME: this is ugly...
        for msg in msg.iter_init() {
            if let Some(dict) = msg.as_iter() {
                for chunk in dict.collect::<Vec<_>>().chunks(2) {
                    // only handle key-value pairs
                    if chunk.len() != 2 {
                        continue;
                    }

                    if let (Some(key), value) = (chunk[0].as_str(), &chunk[1]) {
                        if key != "Metadata" {
                            continue;
                        }

                        if let Some(metadata_dict) = value.as_iter() {
                            for m in metadata_dict.collect::<Vec<_>>().iter() {
                                if let Some(metadata_item) = m.as_iter() {
                                    for metadata_item_chunk in
                                        metadata_item.collect::<Vec<_>>().chunks(2)
                                    {
                                        if metadata_item_chunk.len() != 2 {
                                            continue;
                                        }

                                        if let Some(meta_key) = metadata_item_chunk[0].as_str() {
                                            result.set_field(
                                                meta_key,
                                                Variant(metadata_item_chunk[1].box_clone()),
                                            );
                                        }
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
