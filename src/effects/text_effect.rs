use crate::effects::effect::Effect;

#[derive(Default)]
pub struct TextEffect {
    text: String,
    last_drawn: String,
    effects: Vec<Box<dyn Effect>>,
    update_tick: bool,
}

impl TextEffect {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_effect_text(&mut self, text: String) {
        self.text = text.clone();
        self.update_tick = true;
        for effect in &mut self.effects {
            effect.set_text(text.clone());
            effect.apply(text.clone());
        }
    }

    pub fn has_active_effects(&self) -> bool {
        self.effects.iter().any(|elem| elem.is_active())
    }

    pub fn current_text(&self) -> &str {
        &self.text
    }

    pub fn should_redraw(&mut self) {
        self.update_tick = true;
    }

    pub fn with_effect(mut self, effect: Box<dyn Effect>) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn override_last_drawn(&mut self, text: String) {
        self.last_drawn = text;
    }

    pub fn draw(&mut self, text: &str) -> String {
        if self.last_drawn.is_empty() {
            self.last_drawn = text.to_string();
        }

        for effect in &mut self.effects {
            effect.set_text(text.to_string());
        }

        if !self.update_tick {
            return self.last_drawn.clone();
        }

        self.update_tick = false;

        let mut result = text.to_owned();
        for effect in &mut self.effects {
            result = effect.apply(result);
        }
        if !result.is_empty() {
            self.last_drawn = result.clone();
        }
        result
    }
}

