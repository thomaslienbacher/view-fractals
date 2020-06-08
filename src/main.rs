mod fractal;
mod text;
mod renderer;

use minifb::{Key, Window, WindowOptions, MouseButton, MouseMode, Scale, KeyRepeat};
use crate::fractal::*;
use crate::text::*;
use num::complex::Complex64;
use std::time::{SystemTime};
use crate::renderer::{CpuRenderer, Renderer, OpenClRenderer};
use std::cmp::max;

const TITLE: &'static str = "View Julia Fractals - ESC to exit";
const WIDTH: usize = 1600;
const HEIGHT: usize = WIDTH / 16 * 9;
const FONT_SIZE: usize = (WIDTH / 110) + 1;

fn main() {
    let mut renderer: Box<dyn Renderer> = Box::new(CpuRenderer::new());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        TITLE,
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| {
        panic!("Error: {}", e);
    });

    let painter = TextPainter::new();
    let bounds = Bounds::new(-2.1, 2.1, WIDTH as f64 / HEIGHT as f64);
    let mut julia = JuliaFractal::new(bounds);
    let mut max_iterations = 670;
    let mut delta_time: f64;

    println!("{:?}", julia);

    while window.is_open() && !window.is_key_released(Key::Escape) {
        let start = SystemTime::now();

        renderer.render(&julia, max_iterations, &mut buffer, WIDTH, HEIGHT);

        painter.paint_string(&mut buffer, WIDTH, 3, 5, &format!("{:?}", julia), FONT_SIZE, 0.7);
        painter.paint_string(&mut buffer, WIDTH, 3, FONT_SIZE + 12, &format!("mode: {}", renderer.name()), FONT_SIZE, 0.7);
        painter.paint_string(&mut buffer, WIDTH, 3, FONT_SIZE * 2 + 17, &format!("iterations: {}", max_iterations), FONT_SIZE, 0.7);

        let time_passed = SystemTime::now().duration_since(start).unwrap();
        delta_time = time_passed.as_secs_f64();
        window.set_title(format!("{} DT: {}", TITLE, delta_time).as_ref());

        if window.is_key_pressed(Key::W, KeyRepeat::Yes) {
            max_iterations += max((20.0 * delta_time) as u32, 1);
            renderer.on_max_iterations_change(max_iterations);
        }
        if window.is_key_pressed(Key::S, KeyRepeat::Yes) {
            if max_iterations > 2 {
                max_iterations -= max((20.0 * delta_time) as u32, 1);
                renderer.on_max_iterations_change(max_iterations);
            }
        }

        if let Some((a, b)) = window.get_mouse_pos(MouseMode::Discard) {
            if window.get_mouse_down(MouseButton::Left) {
                julia.bounds.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, delta_time);
                renderer.on_bounds_change(&julia.bounds);
            }
            if window.get_mouse_down(MouseButton::Right) {
                julia.bounds.zoom(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, -delta_time);
                renderer.on_bounds_change(&julia.bounds);
            }
            if window.get_mouse_down(MouseButton::Middle) {
                julia.bounds.translate(a as f64 / WIDTH as f64, b as f64 / HEIGHT as f64, delta_time);
                renderer.on_bounds_change(&julia.bounds);
            }
            if window.is_key_down(Key::Space) {
                julia.add = Complex64::new((a as f64 / WIDTH as f64) - 0.5, (b as f64 / HEIGHT as f64) - 0.5);
                julia.add.re *= 2. * 2.;
                julia.add.im *= 2. * 2.;
                renderer.on_add_change(&julia.add);
            }
            if window.is_key_down(Key::Key1) {
                renderer = Box::new(CpuRenderer::new());
            }
            if window.is_key_down(Key::Key2) {
                renderer = Box::new(OpenClRenderer::new(WIDTH, HEIGHT, &julia, max_iterations));
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
