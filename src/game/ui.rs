use super::*;

use crate::ui::{layout::AreaOps, widget::WidgetState};

pub struct GameUI {
    pub coins: WidgetState,
    // pub scissors: WidgetState,
    // pub seed: WidgetState,
}

impl GameUI {
    pub fn new() -> Self {
        Self {
            coins: WidgetState::new(),
            // scissors: WidgetState::new(),
            // seed: WidgetState::new(),
        }
    }

    pub fn layout(&mut self, screen: Aabb2<f32>, context: &mut UiContext) {
        let layout_size = screen.height() * 0.05;

        let left_bar = screen
            .extend_uniform(-layout_size * 0.5)
            .cut_left(layout_size * 2.0);

        // Coins
        let coins = left_bar.align_aabb(vec2(7.0, 2.0) * layout_size, vec2(0.0, 1.0));
        self.coins.update(coins, context);

        // Items
        // let item_size = vec2(1.5, 1.5) * layout_size;
        // let widgets = [&mut self.scissors, &mut self.seed];
        // let items = left_bar
        //     .align_aabb(item_size, vec2(0.5, 0.5))
        //     .stack_aligned(
        //         vec2(0.0, item_size.y + layout_size),
        //         widgets.len(),
        //         vec2(0.5, 0.5),
        //     );
        // for (pos, widget) in itertools::izip!(items, widgets) {
        //     widget.update(pos, context);
        // }
    }
}
