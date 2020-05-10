use fontdue::*;
use palette::{LinSrgb};
use super::fractal::ColorEncode;
use std::cmp::min;

pub struct TextPainter {
    font: fontdue::Font
}

impl TextPainter {
    pub fn new() -> TextPainter {
        let file = include_bytes!("../resources/SourceSansPro-Regular.ttf") as &[u8];

        TextPainter {
            font: fontdue::Font::from_bytes(file, FontSettings::default()).unwrap()
        }
    }

    pub fn paint_string(&self, buffer: &mut Vec<u32>, buffer_stride: usize, x: usize, y: usize, str: &str, size: usize, alpha: f32) {
        let mut sum_x = 0;
        let mut max_y = 0;
        let mut min_y = 0;

        // calculate size of black box
        for c in str.chars() {
            let metrics = self.font.metrics(c, size as f32);

            if metrics.bounds.ymax > max_y as f32 {
                max_y = metrics.bounds.ymax.ceil() as usize;
            }
            if metrics.bounds.ymin < min_y as f32 {
                min_y = metrics.bounds.ymin.floor() as i32;
            }
            sum_x += metrics.width + 2;
        }

        // draw black box
        for x in (x - 2)..(sum_x + x + 2) {
            for y in (y - 2)..(y + max_y + min_y.abs() as usize + 2) {
                let c = LinSrgb::new(0., 0., 0.) +
                    LinSrgb::from_u32(buffer[x + y * buffer_stride]) * (1. - alpha);
                buffer[x + y * buffer_stride] = c.encode();
            }
        }

        let mut local_x = x;

        // draw characters
        for c in str.chars() {
            let (metrics, bitmap) = self.font.rasterize(c, size as f32);

            for (i, b) in bitmap.iter().enumerate() {
                let x = (i % metrics.width) + local_x;
                let mut y: i32 = (i as i32 / metrics.width as i32) + y as i32 + (max_y as i32 - metrics.bounds.ymax as i32);
                let c = *b as f32 / 255.;
                let a = LinSrgb::from((c, c, c));
                let b = LinSrgb::from_u32(buffer[x + y as usize * buffer_stride]);
                let mut c = a * alpha + b;
                c.red = num::clamp(c.red, 0., 1.);
                c.green = num::clamp(c.green, 0., 1.);
                c.blue = num::clamp(c.blue, 0., 1.);
                buffer[x + y as usize * buffer_stride] = c.encode();
            }

            local_x += metrics.width + 2;
        }
    }
}