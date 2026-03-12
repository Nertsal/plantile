use super::*;

pub fn get_pixel_scale(framebuffer_size: vec2<usize>) -> f32 {
    const TARGET_SIZE: vec2<usize> = vec2(640, 360);
    let size = framebuffer_size.as_f32();
    let ratio = size / TARGET_SIZE.as_f32();
    ratio.x.min(ratio.y)
}

pub struct UiRender {
    context: Context,
    pub util: UtilRender,
}

impl UiRender {
    pub fn new(context: Context) -> Self {
        Self {
            util: UtilRender::new(context.clone()),
            context,
        }
    }

    pub fn draw_texture(
        &self,
        quad: Aabb2<f32>,
        texture: &ugli::Texture,
        color: Color,
        pixel_scale: f32,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_texture_with(
            quad,
            texture,
            color,
            pixel_scale,
            mat3::identity(),
            framebuffer,
        )
    }

    pub fn draw_texture_with(
        &self,
        quad: Aabb2<f32>,
        texture: &ugli::Texture,
        color: Color,
        pixel_scale: f32,
        transform: mat3<f32>,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let pos = geng_utils::pixel::pixel_perfect_aabb(
            quad.center(),
            vec2(0.5, 0.5),
            (texture.size().as_f32() * pixel_scale * get_pixel_scale(framebuffer.size()))
                .map(|x| x as usize),
            &geng::PixelPerfectCamera,
            framebuffer.size().as_f32(),
        );
        self.context.geng.draw2d().draw2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw2d::TexturedQuad::colored(
                Aabb2::ZERO.extend_symmetric(pos.size() / 2.0),
                texture,
                color,
            )
            .transform(mat3::translate(pos.center()) * transform),
        );
    }

    // pub fn draw_subtexture(
    //     &self,
    //     quad: Aabb2<f32>,
    //     texture: &SubTexture,
    //     color: Color,
    //     pixel_scale: f32,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     let size = texture.size().as_f32() * pixel_scale * get_pixel_scale(framebuffer.size());
    //     let pos = crate::ui::layout::align_aabb(size, quad, vec2(0.5, 0.5));
    //     self.context.geng.draw2d().draw2d(
    //         framebuffer,
    //         &geng::PixelPerfectCamera,
    //         &draw2d::TexturedQuad::colored(pos, &*texture.texture, color).sub_texture(texture.uv),
    //     );
    // }

    // pub fn draw_outline(
    //     &self,
    //     quad: Aabb2<f32>,
    //     width: f32,
    //     color: Color,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     let scale = get_pixel_scale(framebuffer.size());
    //     let (texture, real_width) = if width < 2.0 * scale {
    //         (&self.context.assets.sprites.border_thinner, 1.0 * scale)
    //     } else if width < 16.0 * scale {
    //         (&self.context.assets.sprites.border_thin, 2.0 * scale)
    //     } else {
    //         (&self.context.assets.sprites.border, 4.0 * scale)
    //     };
    //     self.util.draw_nine_slice(
    //         quad.extend_uniform(real_width - width),
    //         color,
    //         texture,
    //         scale,
    //         &geng::PixelPerfectCamera,
    //         framebuffer,
    //     );
    // }

    // pub fn fill_quad_width(
    //     &self,
    //     position: Aabb2<f32>,
    //     width: f32,
    //     color: Color,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     let scale = get_pixel_scale(framebuffer.size());
    //     let (texture, real_width) = if width < 2.0 * scale {
    //         (&self.context.assets.sprites.fill_thinner, 1.0 * scale)
    //     } else if width < 16.0 * scale {
    //         (&self.context.assets.sprites.fill_thin, 2.0 * scale)
    //     } else {
    //         (&self.context.assets.sprites.fill, 4.0 * scale)
    //     };
    //     self.util.draw_nine_slice(
    //         position.extend_uniform(real_width - width),
    //         color,
    //         texture,
    //         scale,
    //         &geng::PixelPerfectCamera,
    //         framebuffer,
    //     );
    // }

    // pub fn fill_quad(
    //     &self,
    //     position: Aabb2<f32>,
    //     color: Color,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     let size = position.size();
    //     let size = size.x.min(size.y);

    //     let scale = ui::get_pixel_scale(framebuffer.size());

    //     let texture = if size < 48.0 * scale {
    //         &self.context.assets.sprites.fill_thin
    //     } else {
    //         &self.context.assets.sprites.fill
    //     };
    //     self.util.draw_nine_slice(
    //         position,
    //         color,
    //         texture,
    //         scale,
    //         &geng::PixelPerfectCamera,
    //         framebuffer,
    //     );
    // }

    pub fn draw_quad(
        &self,
        quad: Aabb2<f32>,
        color: Rgba<f32>,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.context.geng.draw2d().draw2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw2d::Quad::new(quad, color),
        );
    }

    // pub fn draw_button(
    //     &self,
    //     button: &ButtonWidget,
    //     theme: Theme,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     let state = &button.text.state;
    //     if !state.visible {
    //         return;
    //     }

    //     let width = button.text.options.size * 0.2;

    //     let position = state.position;
    //     let bg_color = theme.get_color(button.bg_color);

    //     if state.mouse_left.pressed.is_some() {
    //         self.fill_quad(position.extend_uniform(-width), bg_color, framebuffer);
    //     } else if state.hovered {
    //         self.fill_quad(position.extend_uniform(-width * 0.5), bg_color, framebuffer)
    //     } else {
    //         self.fill_quad(position, bg_color, framebuffer)
    //     }
    //     self.draw_text_colored(&button.text, theme.dark, framebuffer);
    // }

    // pub fn draw_icon_button(
    //     &self,
    //     icon: &IconButtonWidget,
    //     theme: Theme,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     if !icon.icon.state.visible {
    //         return;
    //     }
    //     self.draw_icon(&icon.icon, theme, framebuffer);
    // }

    // pub fn draw_icon(&self, icon: &IconWidget, theme: Theme, framebuffer: &mut ugli::Framebuffer) {
    //     if !icon.state.visible {
    //         return;
    //     }

    //     if let Some(bg) = &icon.background {
    //         match bg.kind {
    //             IconBackgroundKind::NineSlice => {
    //                 let texture = //if width < 5.0 {
    //                     &self.context.assets.sprites.fill_thin;
    //                 // } else {
    //                 //     &self.assets.sprites.fill
    //                 // };
    //                 self.util.draw_nine_slice(
    //                     icon.state.position,
    //                     theme.get_color(bg.color),
    //                     texture,
    //                     icon.pixel_scale * get_pixel_scale(framebuffer.size()),
    //                     &geng::PixelPerfectCamera,
    //                     framebuffer,
    //                 );
    //             }
    //             IconBackgroundKind::Circle => {
    //                 self.draw_texture(
    //                     icon.state.position,
    //                     &self.context.assets.sprites.circle,
    //                     theme.get_color(bg.color),
    //                     icon.pixel_scale,
    //                     framebuffer,
    //                 );
    //             }
    //         }
    //     }
    //     self.draw_subtexture(
    //         icon.state.position,
    //         &icon.texture,
    //         theme.get_color(icon.color),
    //         icon.pixel_scale,
    //         framebuffer,
    //     );
    // }

    // // TODO: as text render option
    // pub fn draw_text_wrapped(&self, widget: &TextWidget, framebuffer: &mut ugli::Framebuffer) {
    //     if !widget.state.visible {
    //         return;
    //     }

    //     let main = widget.state.position;
    //     let lines = crate::util::wrap_text(
    //         &self.context.assets.fonts.pixel,
    //         &widget.text,
    //         main.width() / widget.options.size,
    //     );
    //     let row = main.align_aabb(vec2(main.width(), widget.options.size), vec2(0.5, 1.0));
    //     let rows = row.stack(vec2(0.0, -row.height()), lines.len());

    //     for (line, position) in lines.into_iter().zip(rows) {
    //         self.util.draw_text(
    //             line,
    //             position.align_pos(widget.options.align),
    //             widget.options,
    //             &geng::PixelPerfectCamera,
    //             framebuffer,
    //         );
    //     }
    // }

    // pub fn draw_text(&self, widget: &TextWidget, framebuffer: &mut ugli::Framebuffer) {
    //     self.draw_text_colored(widget, widget.options.color, framebuffer)
    // }

    // pub fn draw_text_colored(
    //     &self,
    //     widget: &TextWidget,
    //     color: Color,
    //     framebuffer: &mut ugli::Framebuffer,
    // ) {
    //     if !widget.state.visible {
    //         return;
    //     }

    //     // Fit to area
    //     let mut widget = widget.clone();

    //     let font = &self.context.assets.fonts.pixel;
    //     let measure = font.measure(&widget.text, 1.0);

    //     let size = widget.state.position.size();
    //     let right = vec2(size.x, 0.0).rotate(widget.options.rotation).x;
    //     let left = vec2(0.0, size.y).rotate(widget.options.rotation).x;
    //     let width = if left.signum() != right.signum() {
    //         left.abs() + right.abs()
    //     } else {
    //         left.abs().max(right.abs())
    //     };

    //     let max_height = size.y * 0.9;
    //     let max_width = width * 0.85; // Leave some space TODO: move into a parameter or smth
    //     let max_size = (max_width / measure.width()).min(max_height / measure.height());
    //     let size = widget.options.size.min(max_size).min(max_height);

    //     widget.options.size = size;

    //     self.util.draw_text(
    //         &widget.text,
    //         geng_utils::layout::aabb_pos(widget.state.position, widget.options.align),
    //         widget.options.color(color),
    //         &geng::PixelPerfectCamera,
    //         framebuffer,
    //     );
    //     // self.draw_quad(
    //     //     widget
    //     //         .state
    //     //         .position
    //     //         .align_aabb(measure.size() * size, widget.options.align),
    //     //     Rgba::new(0.7, 0.5, 0.5, 0.75),
    //     //     framebuffer,
    //     // );
    // }
}
