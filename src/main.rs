mod fractal;
mod text;
mod renderer;

use minifb::{Key, Window, WindowOptions, MouseButton, MouseMode, Scale, KeyRepeat, CursorStyle};
use crate::fractal::*;
use crate::text::*;
use num::complex::Complex64;
use std::time::{SystemTime};
use crate::renderer::{CpuRenderer, Renderer, OpenClRenderer, PNGSaver};
use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::thread;

const TITLE: &str = "View Julia Fractals - ESC to exit";
const WIDTH: usize = 1900;
const HEIGHT: usize = WIDTH / 16 * 9;
const FONT_SIZE: usize = (WIDTH / 110) + 1;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        TITLE,
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            borderless: false,
            title: true,
            resize: false,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| {
        panic!("Error: {}", e);
    });

    window.set_cursor_style(CursorStyle::Crosshair);

    let mut max_iterations = 20;
    let bounds = Bounds::new(-2.1, 2.1, WIDTH as f64 / HEIGHT as f64);
    let mut julia = JuliaFractal::new(bounds);
    let mut renderer: Box<dyn Renderer> = Box::new(OpenClRenderer::new(WIDTH, HEIGHT, &julia, max_iterations));
    let render_lock = Arc::new(Mutex::new(()));
    let painter = TextPainter::new();
    let mut delta_time: f64;

    println!("{:?}", julia);

    while window.is_open() && !(window.is_key_down(Key::Escape) | window.is_key_down(Key::Q)) {
        let start = SystemTime::now();

        renderer.render(&julia, max_iterations, &mut buffer, WIDTH, HEIGHT);

        painter.paint_string(&mut buffer, WIDTH, 3, 5, &format!("{:?}", julia), FONT_SIZE, 0.7);
        painter.paint_string(&mut buffer, WIDTH, 3, FONT_SIZE + 12, &format!("mode: {}", renderer.name()), FONT_SIZE, 0.7);
        painter.paint_string(&mut buffer, WIDTH, 3, FONT_SIZE * 2 + 17, &format!("iterations: {}", max_iterations), FONT_SIZE, 0.7);

        let time_passed = SystemTime::now().duration_since(start).unwrap();
        delta_time = time_passed.as_secs_f64();
        window.set_title(format!("{} DT: {}", TITLE, delta_time).as_ref());

        if window.is_key_pressed(Key::W, KeyRepeat::Yes) {
            let mut mult = 1;
            if window.is_key_down(Key::LeftShift) {
                mult = 4;
            }

            max_iterations += max((60.0 * delta_time) as u32, 1) * mult;
            renderer.on_max_iterations_change(max_iterations);
        }
        if window.is_key_pressed(Key::S, KeyRepeat::Yes) && max_iterations > 2 {
            let mut mult = 1;
            if window.is_key_down(Key::LeftShift) {
                mult = 4;
            }

            max_iterations -= max((60.0 * delta_time) as u32, 1) * mult;
            renderer.on_max_iterations_change(max_iterations);
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
            if window.is_key_down(Key::U) {
                let ymax = f64::max(julia.bounds.ybounds.0.abs(), julia.bounds.ybounds.1.abs());
                julia.bounds = Bounds::new(-ymax, ymax, WIDTH as f64 / HEIGHT as f64);
                renderer.on_bounds_change(&julia.bounds);
            }
            if window.is_key_down(Key::Key1) {
                renderer = Box::new(CpuRenderer::new());
            }
            if window.is_key_down(Key::Key2) {
                renderer = Box::new(OpenClRenderer::new(WIDTH, HEIGHT, &julia, max_iterations));
            }
            if window.is_key_down(Key::Key3) {
                let j = julia.clone();
                let lock = render_lock.clone();
                thread::spawn(move || {
                    match lock.try_lock() {
                        Ok(_) => PNGSaver::new(&j).save(&j),
                        _ => {}
                    }
                });
            }
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
