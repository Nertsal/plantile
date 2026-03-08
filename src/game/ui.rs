use super::*;

use crate::{
    render::ui::get_pixel_scale,
    ui::{layout::AreaOps, widget::WidgetState},
};

pub struct GameUI {
    pub inventory: WidgetState,
    pub inventory_items: Vec<WidgetState>,

    pub shop: WidgetState,
    pub shop_items: Vec<(WidgetState, Tile)>,

    pub gold: WidgetState,
}

impl GameUI {
    pub fn new() -> Self {
        Self {
            inventory: WidgetState::new(),
            inventory_items: vec![WidgetState::new(); 6],

            shop: WidgetState::new(),
            shop_items: vec![(WidgetState::new(), Tile::Light)],

            gold: WidgetState::new(),
        }
    }

    pub fn layout(&mut self, screen: Aabb2<f32>, context: &mut UiContext) {
        let layout_size = screen.height() * 0.05;
        let pixel_scale = get_pixel_scale(screen.size().map(|x| x as usize));

        // Inventory
        let inventory = screen
            .align_aabb(vec2(342.0, 45.0) * pixel_scale, vec2(0.5, 0.0))
            .map(|x| x.round());
        self.inventory
            .update(inventory.extend_down(pixel_scale * 3.0), context);

        // Items
        let items = inventory.extend_uniform(-6.0 * pixel_scale);
        let item = items.align_aabb(vec2(28.0, 34.0) * pixel_scale, vec2(0.0, 0.5));
        let items = item.stack(vec2(34.0, 0.0) * pixel_scale, self.inventory_items.len());
        for (widget, pos) in self.inventory_items.iter_mut().zip(items) {
            widget.update(pos, context);
        }

        // Shop
        let shop = inventory
            .extend_uniform(-4.0 * pixel_scale)
            .align_aabb(vec2(137.0, 37.0) * pixel_scale, vec2(1.0, 0.5))
            .map(|x| x.round());
        self.shop.update(shop, context);

        // Items
        let items = shop.extend_uniform(-3.0 * pixel_scale);
        let item = items.align_aabb(vec2(28.0, 34.0) * pixel_scale, vec2(1.0, 0.5));
        let items = item.stack(vec2(-34.0, 0.0) * pixel_scale, self.shop_items.len());
        for ((widget, _), pos) in self.shop_items.iter_mut().zip(items) {
            widget.update(pos, context);
        }

        // Gold
        let gold = inventory
            .align_aabb(vec2(31.0, 20.0) * pixel_scale, vec2(1.0, 1.0))
            .translate(vec2(-7.0, 30.0) * pixel_scale);
        self.gold.update(gold, context);
    }
}
