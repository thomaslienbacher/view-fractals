mod fractal;

use minifb::{Key, Window, WindowOptions, MouseButton, MouseMode, Scale, KeyRepeat};
use crate::fractal::color::*;
use crate::fractal::julia::JuliaFractal;
use palette::{LinSrgb, Hsv, Gradient};
use rayon::prelude::*;
use num::complex::Complex64;
use std::time::{SystemTime};

const WIDTH: usize = 1920;
const HEIGHT: usize = WIDTH / 16 * 9;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut options = WindowOptions::default();
    options.scale = Scale::FitScreen;
    options.borderless = true;
    options.resize = false;

    let mut window = Window::new(
        "View Fractals - ESC to exit",
        WIDTH,
        HEIGHT,
        options,
    ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut julia = JuliaFractal::new();
    julia.xbounds.0 *= (WIDTH as f64 / HEIGHT as f64);
    julia.xbounds.1 *= (WIDTH as f64 / HEIGHT as f64);
    let mut first = true;

    let grad = Gradient::new(vec![
        Hsv::from(LinSrgb::new(1. as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 1. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 0. as f32, 1. as f32))
    ]);

    println!("{:?}", julia);

    while window.is_open() && !window.is_key_released(Key::Escape) {
        if first {
            let start = SystemTime::now();
            buffer.par_iter_mut().enumerate().for_each(|(i, v)| {
                let x = ((i % WIDTH) as f64 / WIDTH as f64);
                let y = ((i / WIDTH) as f64 / HEIGHT as f64);
                let j = julia.get(x, y);

                let g = {
                    let m = julia.max_iterations - 1;
                    let p = (j as f32) / m as f32;
                    p
                };

                let c: LinSrgb = if j != julia.max_iterations as f64 {
                    LinSrgb::from(grad.get(g))
                } else {
                    LinSrgb::new(0., 0., 0.)
                };

                *v = Color::rgb(c.red.into(), c.green.into(), c.blue.into()).into();
            });

            let time_passed = SystemTime::now().duration_since(start).unwrap();
            println!("Frame time: {}", time_passed.as_secs_f32());

            //first = false;
        }

        if window.is_key_pressed(Key::W, KeyRepeat::Yes) {
            julia.max_iterations += 1;
        }
        if window.is_key_pressed(Key::S, KeyRepeat::Yes) {
            if julia.max_iterations > 2 {
                julia.max_iterations -= 1;
            }
        }

        if let Some((a, b)) = window.get_mouse_pos(MouseMode::Discard) {
            if window.get_mouse_down(MouseButton::Left) {
                julia.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, 1.0);
            }
            if window.get_mouse_down(MouseButton::Right) {
                julia.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, -1.0);
            }
            if window.get_mouse_down(MouseButton::Middle) {
                julia.translate(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64);
            }
            if window.is_key_down(Key::Space) {
                julia.add = Complex64::new((a as f64 / WIDTH as f64) - 0.5, (b as f64 / HEIGHT as f64) - 0.5);
                julia.add.re *= 2. * 2.;
                julia.add.im *= 2. * 2.;
            }
        }

        //println!("{:?}", julia);

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
