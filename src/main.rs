mod fractal;

use minifb::{Key, Window, WindowOptions, MouseButton, MouseMode, Scale, KeyRepeat};
use crate::fractal::*;
use palette::{LinSrgb, Hsv, Gradient};
use rayon::prelude::*;
use num::complex::Complex64;
use std::time::{SystemTime};

const WIDTH: usize = 1200;
const HEIGHT: usize = WIDTH / 16 * 9;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let title = "View Fractals - ESC to exit";
    let mut window = Window::new(
        title,
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| {
        panic!("Error: {}", e);
    });

    let bounds = Bounds::new(-2.1, 2.1, WIDTH as f64 / HEIGHT as f64);
    let mut julia = JuliaFractal::new(bounds);
    let mut max_iterations = 50;
    let mut delta_time = 0.;

    let grad = Gradient::new(vec![
        Hsv::from(LinSrgb::new(1. as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 1. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 0. as f32, 1. as f32)),
        Hsv::from(LinSrgb::new(1. as f32, 1. as f32, 1. as f32))
    ]);

    println!("{:?}", julia);

    while window.is_open() && !window.is_key_released(Key::Escape) {
        let start = SystemTime::now();
        buffer.par_iter_mut().enumerate().for_each(|(i, v)| {
            let x = ((i % WIDTH) as f64 / WIDTH as f64);
            let y = ((i / WIDTH) as f64 / HEIGHT as f64);
            let j = julia.get(x, y, max_iterations);

            let g = {
                let m = max_iterations - 1;
                let p = (j as f32) / m as f32;
                p
            };

            if j != max_iterations as f64 {
                *v = LinSrgb::from(grad.get(g)).encode();
            } else {
                *v = 0;
            };
        });

        let time_passed = SystemTime::now().duration_since(start).unwrap();
        delta_time = time_passed.as_secs_f64();
        window.set_title(format!("{} DT: {}", title, delta_time).as_ref());

        if window.is_key_pressed(Key::W, KeyRepeat::Yes) {
            max_iterations += 1;
        }
        if window.is_key_pressed(Key::S, KeyRepeat::Yes) {
            if max_iterations > 2 {
                max_iterations -= 1;
            }
        }

        if let Some((a, b)) = window.get_mouse_pos(MouseMode::Discard) {
            if window.get_mouse_down(MouseButton::Left) {
                julia.bounds.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, delta_time);
            }
            if window.get_mouse_down(MouseButton::Right) {
                julia.bounds.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, -delta_time);
            }
            if window.get_mouse_down(MouseButton::Middle) {
                julia.bounds.translate(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, delta_time);
            }
            if window.is_key_down(Key::Space) {
                julia.add = Complex64::new((a as f64 / WIDTH as f64) - 0.5, (b as f64 / HEIGHT as f64) - 0.5);
                julia.add.re *= 2. * 2.;
                julia.add.im *= 2. * 2.;
            }
        }

        println!("{:?}", julia);

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
