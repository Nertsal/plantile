use super::*;

use crate::ui::{layout::AreaOps, widget::WidgetState};

pub struct GameUI {
    pub scissors: WidgetState,
}

impl GameUI {
    pub fn new() -> Self {
        Self {
            scissors: WidgetState::new(),
        }
    }

    pub fn layout(&mut self, screen: Aabb2<f32>, context: &mut UiContext) {
        let layout_size = screen.height() * 0.05;

        let left_bar = screen
            .extend_uniform(-layout_size * 1.5)
            .cut_left(layout_size * 2.0);
        let item_size = vec2(1.5, 1.5) * layout_size;
        let items = left_bar.stack_aligned(vec2(0.0, item_size.y), 1, vec2(0.5, 0.5));
        for (pos, widget) in itertools::izip!(items, [&mut self.scissors]) {
            widget.update(pos, context);
        }
    }
}
