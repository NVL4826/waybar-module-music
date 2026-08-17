use crate::models::{
    mpris_metadata::MprisMetadata, mpris_playback::MprisPlayback, mpris_seeked::MprisSeeked,
};

#[derive(Debug, Clone)]
pub enum MprisEvent {
    Metadata(MprisMetadata),
    Playback(MprisPlayback),
    Seeked(MprisSeeked),
}
