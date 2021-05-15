use std::path::Path;

pub trait Bitmap {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn depth(&self) -> u8;

    fn size(&self) -> usize;

    fn blit(&mut self, x0: i16, y0: i16, src: &Self);

    fn scale_blit(&mut self, x0: i16, y0: i16, w: u16, h: u16, src: &Self);
}

#[repr(C)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, Debug)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

pub trait Gfx {
    type Bitmap: Bitmap;
    type Color;
    type Font;

    fn put_pixel(&mut self, x0: i16, y0: i16, color: Self::Color);

    fn get_pixel(&self, x0: i16, y0: i16) -> Self::Color;

    fn put_char(&mut self, code: char, x0: i16, y0: i16, color: Self::Color, font: &Self::Font) -> u8;

    fn put_text(&mut self, text: impl AsRef<str>, x0: i16, y0: i16, color: Self::Color, font: &Self::Font) -> u16;

    fn get_glyph(&self, code: char, color: Self::Color, bitmap: &mut Self::Bitmap, font: &Self::Font) -> core::result::Result<(), ()>;

    fn blit(&mut self, x0: i16, y0: i16, source: &Self::Bitmap);

    fn scale_blit(&mut self, x0: u16, y0: u16, w: u16, h: u16, source: &Self::Bitmap);

    fn draw_line(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, color: Self::Color);

    fn draw_vline(&mut self, x0: i16, y0: i16, width: u16, color: Self::Color);

    fn draw_hline(&mut self, x0: i16, y0: i16, height: u16, color: Self::Color);

    fn draw_rectangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, color: Self::Color);

    fn fill_rectangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, color: Self::Color);

    fn draw_circle(&mut self, x0: i16, y0: i16, r: i16, color: Self::Color);

    fn fill_circle(&mut self, x0: i16, y0: i16, r: i16, color: Self::Color);

    fn draw_ellipse(&mut self, x0: i16, y0: i16, a: i16, b: i16, color: Self::Color);

    fn fill_ellipse(&mut self, x0: i16, y0: i16, a: i16, b: i16, color: Self::Color);

    fn draw_polygon(&mut self, vertices: &[Point], color: Self::Color);

    fn fill_polygon(&mut self, vertices: &[Point], color: Self::Color);

    fn draw_triangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, x2: i16, y2: i16, color: Self::Color);

    fn fill_triangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, x2: i16, y2: i16, color: Self::Color);

    fn draw_rounded_rectangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, r: i16, color: Self::Color);

    fn fill_rounded_rectangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, r: i16, color: Self::Color);

    fn load_image(&mut self, x0: i16, y0: i16, filename: impl AsRef<Path>);

    fn set_clip_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16);

    fn color(&self, r: u8, g: u8, b: u8) -> Self::Color;

    fn bitmap(width: u16, height: u16, depth: u8) -> Self::Bitmap;

    fn clear_clip_window(&mut self);

    fn clear_screen(&mut self);

    fn flush(&mut self) -> usize;
}
