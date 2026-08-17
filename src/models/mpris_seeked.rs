use dbus::Message;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MprisSeeked {
    pub player_id: String,
    pub position: u128,
}

impl MprisSeeked {
    pub fn new(player_id: String) -> Self {
        Self {
            player_id,
            position: 0,
        }
    }

    pub fn from_dbus_message(msg: &Message) -> Self {
        let sender = msg.sender().map(|s| s.to_string()).unwrap_or_default();
        let mut result = MprisSeeked::new(sender);

        if let Some(position) = msg.get1::<i64>() {
            result.position = position as u128;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seeked_from_message_without_sender_does_not_panic() {
        let msg = Message::new_signal(
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            "Seeked",
        )
        .unwrap();

        let seeked = MprisSeeked::from_dbus_message(&msg);
        assert_eq!(seeked.player_id, "");
        assert_eq!(seeked.position, 0);
    }
}

