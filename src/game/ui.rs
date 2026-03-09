use super::*;

use crate::{
    render::ui::get_pixel_scale,
    ui::{layout::AreaOps, widget::WidgetState},
};

pub struct GameUI {
    pub inventory: WidgetState,
    pub inventory_items: Vec<WidgetState>,

    pub shop: WidgetState,
    pub shop_hover_t: f32,
    pub shop_items: Vec<(WidgetState, Tile)>,

    pub gold: WidgetState,
}

impl GameUI {
    pub fn new(context: &Context) -> Self {
        let shop = &context.assets.config.shop;
        Self {
            inventory: WidgetState::new(),
            inventory_items: vec![WidgetState::new(); 6],

            shop: WidgetState::new(),
            shop_hover_t: 0.0,
            shop_items: shop
                .iter()
                .map(|item| (WidgetState::new(), item.tile.clone()))
                .collect(),

            gold: WidgetState::new(),
        }
    }

    pub fn layout(&mut self, screen: Aabb2<f32>, context: &mut UiContext) {
        // let layout_size = screen.height() * 0.05;
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
        let t = crate::util::smoothstep(self.shop_hover_t);
        let size = vec2(110.0, 200.0) * pixel_scale;
        let shop = screen
            .align_aabb(size, vec2(1.0, 0.75))
            .translate(vec2(size.x * 0.5 * (1.0 - t), 0.0))
            .map(|x| x.round());
        self.shop.update(shop, context);
        if self.shop.hovered {
            self.shop_hover_t += context.delta_time / 0.25;
        } else {
            self.shop_hover_t -= context.delta_time / 0.25;
        }
        self.shop_hover_t = self.shop_hover_t.clamp(0.0, 1.0);

        // Items
        let items = shop.extend_uniform(-6.0 * pixel_scale);
        let rows = items
            .clone()
            .cut_top(45.0 * pixel_scale)
            .stack(vec2(0.0, -45.0 * pixel_scale), 4);
        for (i, row) in rows.into_iter().enumerate() {
            let item = row.align_aabb(vec2(28.0, 34.0) * pixel_scale, vec2(0.0, 0.5));
            let items = item.stack(vec2(34.0, 0.0) * pixel_scale, 3);
            for (j, pos) in items.into_iter().enumerate() {
                let Some((widget, _)) = self.shop_items.get_mut(i * 3 + j) else {
                    break;
                };
                widget.update(pos, context);
            }
        }

        // Gold
        let gold = inventory
            .align_aabb(vec2(31.0, 20.0) * pixel_scale, vec2(1.0, 1.0))
            .translate(vec2(-7.0, 30.0) * pixel_scale);
        self.gold.update(gold, context);
    }
}
