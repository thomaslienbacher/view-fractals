use palette::{LinSrgb};
use num::complex::Complex64;
use interpolation::lerp;

pub trait ColorEncode {
    fn encode(&self) -> u32;
}

impl ColorEncode for LinSrgb {
    fn encode(&self) -> u32 {
        let r = (self.red * 255.0) as u8;
        let g = (self.green * 255.0) as u8;
        let b = (self.blue * 255.0) as u8;

        (255 << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
    }
}

#[derive(Debug)]
pub struct Bounds {
    xbounds: (f64, f64),
    ybounds: (f64, f64),
}

impl Bounds {
    pub fn new(lower: f64, upper: f64, aspect_ratio_xy: f64) -> Bounds {
        Bounds {
            xbounds: (lower * aspect_ratio_xy, upper * aspect_ratio_xy),
            ybounds: (lower, upper),
        }
    }

    pub fn zoom(&mut self, mut x: f64, mut y: f64, delta_time: f64) {
        x = 1. - x;
        y = 1. - y;

        let scale_x = (self.xbounds.1 - self.xbounds.0) * delta_time;
        let scale_y = (self.ybounds.1 - self.ybounds.0) * delta_time;

        self.xbounds.0 += (1. - x) * scale_x;
        self.xbounds.1 -= x * scale_x;
        self.ybounds.0 += (1. - y) * scale_y;
        self.ybounds.1 -= y * scale_y;
    }

    pub fn translate(&mut self, mut x: f64, mut y: f64, delta_time: f64) {
        let scale = ((self.xbounds.1 - self.xbounds.0) + (self.ybounds.1 - self.ybounds.0))
            * delta_time;

        x -= 0.5;
        y -= 0.5;
        x *= scale;
        y *= scale;

        self.xbounds.0 += x;
        self.xbounds.1 += x;
        self.ybounds.0 += y;
        self.ybounds.1 += y;
    }
}

#[derive(Debug)]
pub struct JuliaFractal {
    pub bounds: Bounds,
    pub add: Complex64,
}

impl JuliaFractal {
    pub const fn new(bounds: Bounds) -> JuliaFractal {
        JuliaFractal {
            bounds,
            add: Complex64::new(-0., 0.),
        }
    }

    fn f(&self, z: Complex64) -> Complex64 {
        z.powu(2) + self.add
    }

    pub fn get(&self, x: f64, y: f64, max_iterations: u32) -> f64 {
        let mut z = Complex64::new(
            lerp(&self.bounds.xbounds.0, &self.bounds.xbounds.1, &x),
            lerp(&self.bounds.ybounds.0, &self.bounds.ybounds.1, &y));

        for iteration in 0..=max_iterations {
            if (z.re * z.re) + (z.im * z.im) >= 4. {
                return iteration as f64;
            }

            z = self.f(z);
        }

        return max_iterations as f64;
    }
}
