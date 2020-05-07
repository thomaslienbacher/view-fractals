pub mod color {

    pub struct Color {
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    }

    impl Color {
        pub const fn new() -> Color {
            Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            }
        }

        pub const fn rgb(r: f64, g: f64, b: f64) -> Color {
            Color {
                r,
                g,
                b,
                a: 1.,
            }
        }
    }

    impl std::convert::From<Color> for u32 {
        fn from(c: Color) -> Self {
            let r = num::clamp((c.r * 255.0) as u8, 0, 255);
            let g = num::clamp((c.g * 255.0) as u8, 0, 255);
            let b = num::clamp((c.b * 255.0) as u8, 0, 255);
            let a = num::clamp((c.a * 255.0) as u8, 0, 255);

            ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        }
    }
}

pub mod julia {
    use num::complex::Complex64;
    use interpolation::lerp;

    #[derive(Debug)]
    pub struct JuliaFractal {
        pub xbounds: (f64, f64),
        pub ybounds: (f64, f64),
        pub add: Complex64,
        pub max_iterations: u32,
    }

    impl JuliaFractal {
        pub fn new() -> JuliaFractal {
            JuliaFractal {
                xbounds: (-1.7, 1.7),
                ybounds: (-1.7, 1.7),
                add: Complex64::new(0., 0.),
                max_iterations: 60,
            }
        }

        fn f(&self, z: Complex64) -> Complex64 {
            z.powu(2) + self.add
        }

        pub fn get(&self, x: f64, y: f64) -> f64 {
            let mut z = Complex64::new(
                lerp(&self.xbounds.0, &self.xbounds.1, &x),
                lerp(&self.ybounds.0, &self.ybounds.1, &y));

            for iteration in 0..=self.max_iterations {
                if (z.re * z.re) + (z.im * z.im) >= 4. {
                    return iteration as f64;
                }

                z = self.f(z);
            }

            return self.max_iterations as f64;
        }

        pub fn zoom(&mut self, mut x: f64, mut y: f64, scale: f64) {
            const ZOOM: f64 = 0.07;
            x = 1. - x;
            y = 1. - y;

            let scale_x = (self.xbounds.1 - self.xbounds.0) * scale;
            let scale_y = (self.ybounds.1 - self.ybounds.0) * scale;

            self.xbounds.0 += (1. - x) * ZOOM * scale_x;
            self.xbounds.1 -= x * ZOOM * scale_x;
            self.ybounds.0 += (1. - y) * ZOOM * scale_y;
            self.ybounds.1 -= y * ZOOM * scale_y;
        }

        pub fn translate(&mut self, mut x: f64, mut y: f64) {
            const SCALE: f64 = 0.1;
            let mut scale = (self.xbounds.1 - self.xbounds.0) + (self.ybounds.1 - self.ybounds.0);
            scale *= 0.9;
            x -= 0.5;
            y -= 0.5;
            self.xbounds.0 += x * SCALE * scale;
            self.xbounds.1 += x * SCALE * scale;
            self.ybounds.0 += y * SCALE * scale;
            self.ybounds.1 += y * SCALE * scale;
        }
    }
}