use super::*;

pub struct SfxManager {
    geng: Geng,
    options: Rc<RefCell<Options>>,
}

impl SfxManager {
    pub fn new(geng: Geng, options: Rc<RefCell<Options>>) -> Self {
        Self { geng, options }
    }

    pub fn play(&self, sfx: &geng::Sound) {
        self.play_volume(sfx, 1.0);
    }

    pub fn play_volume(&self, sfx: &geng::Sound, volume: f32) {
        let options = self.options.borrow();
        let mut effect = sfx.effect(self.geng.audio().default_type());
        effect.set_volume(options.volume_sfx() * volume);
        effect.play();
    }
}
