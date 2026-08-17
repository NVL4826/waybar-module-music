use unicode_segmentation::UnicodeSegmentation;

use super::effect::Effect;

pub struct Ellipsis {
    text: String,
    max_width: u16,
    active: bool,
}

impl Ellipsis {
    pub fn new(max_width: u16) -> Self {
        Self {
            max_width,
            active: false,
            text: String::new(),
        }
    }
}

impl Effect for Ellipsis {
    fn apply(&mut self, text: String) -> String {
        let text_graphemes = text.graphemes(true).collect::<Vec<&str>>();
        if text_graphemes.len() <= self.max_width as usize || self.max_width == 0 {
            return text;
        }

        self.active = false;
        format!(
            "{}...",
            // we gotta join here, since we have a Vec<&str>, not a string.
            // and join just looks nicer than .into_iter().collect<String>() but is functionally identical
            text_graphemes.split_at(self.max_width as usize).0.join("")
        )
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn update_active(&mut self) {
        self.active =
            self.text.graphemes(true).count() > self.max_width as usize && self.max_width > 0;
    }

    fn set_text(&mut self, text: String) {
        if self.text != text {
            self.text = text;
            self.update_active();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipsis_ascii() {
        let mut effect = Ellipsis::new(10);
        effect.set_text("hello".to_string());
        assert!(!effect.is_active());
        assert_eq!(effect.apply("hello".to_string()), "hello");

        effect.set_text("hello world from rust".to_string());
        assert!(effect.is_active());
        assert_eq!(effect.apply("hello world from rust".to_string()), "hello worl...");
    }

    #[test]
    fn test_ellipsis_unicode_vietnamese_within_bounds() {
        let mut effect = Ellipsis::new(16);
        // "Tiêu đề bài hát" has 15 graphemes, but 19 UTF-8 bytes
        let text = "Tiêu đề bài hát".to_string();
        effect.set_text(text.clone());
        assert!(!effect.is_active(), "Should not be active because 15 graphemes <= 16 max_width");
        assert_eq!(effect.apply(text.clone()), text);
    }

    #[test]
    fn test_ellipsis_emojis() {
        let mut effect = Ellipsis::new(5);
        // 4 emoji graphemes, 16 bytes
        let text = "🎵🎶🎧🎤".to_string();
        effect.set_text(text.clone());
        assert!(!effect.is_active(), "4 emojis <= 5 max_width");
        assert_eq!(effect.apply(text), "🎵🎶🎧🎤");

        // 5 emoji graphemes with max_width 3 -> active and truncated
        let mut effect_trunc = Ellipsis::new(3);
        let long_emojis = "🎵🎶🎧🎤🎸".to_string();
        effect_trunc.set_text(long_emojis.clone());
        assert!(effect_trunc.is_active());
        assert_eq!(effect_trunc.apply(long_emojis), "🎵🎶🎧...");
    }
}

