mod fractal;
mod text;

use minifb::{Key, Window, WindowOptions, MouseButton, MouseMode, Scale, KeyRepeat};
use crate::fractal::*;
use crate::text::*;
use palette::{LinSrgb, Hsv, Gradient};
use rayon::prelude::*;
use num::complex::Complex64;
use std::time::{SystemTime};
use fontdue::FontSettings;
use ocl::ProQue;
use ocl::enums::DeviceInfo;
use ocl::prm::Double4;

const TITLE: &'static str = "View Fractals - ESC to exit";
const WIDTH: usize = 1800;
const HEIGHT: usize = WIDTH / 16 * 9;
const FONT_SIZE: usize = (WIDTH / 110) + 1;

fn get_color_gradient() -> Gradient<Hsv> {
    Gradient::new(vec![
        Hsv::from(LinSrgb::new(0.2 as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(1. as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 1. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 0. as f32, 1. as f32)),
        Hsv::from(LinSrgb::new(1. as f32, 1. as f32, 1. as f32))
    ])
}

fn render_cpu() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        TITLE,
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::FitScreen,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| {
        panic!("Error: {}", e);
    });

    let painter = TextPainter::new();
    let bounds = Bounds::new(-2.1, 2.1, WIDTH as f64 / HEIGHT as f64);
    let mut julia = JuliaFractal::new(bounds);
    let mut max_iterations = 670;
    let mut delta_time = 0.;

    let grad = get_color_gradient();

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

        painter.paint_string(&mut buffer, WIDTH, 3, 3, &format!("{:?}", julia), FONT_SIZE, 0.5);
        painter.paint_string(&mut buffer, WIDTH, 3, FONT_SIZE, &format!("iterations: {}", max_iterations), FONT_SIZE, 0.5);

        let time_passed = SystemTime::now().duration_since(start).unwrap();
        delta_time = time_passed.as_secs_f64();
        window.set_title(format!("{} DT: {}", TITLE, delta_time).as_ref());

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

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

fn calculate_color_lookup_buffer(max_iterations: usize) -> Vec<u32> {
    let grad = get_color_gradient();

    let mut v = vec![0; max_iterations + 1];
    v[0] = max_iterations as u32;
    for (i, c) in (&mut v).into_iter().skip(1).enumerate() {
        let g = {
            let m = max_iterations - 1;
            let p = i as f32 / m as f32;
            p
        };

        if i != max_iterations {
            *c = LinSrgb::from(grad.get(g)).encode();
        } else {
            *c = 0;
        };
    }

    v
}

fn render_ocl() {
    let mut window_buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        TITLE,
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::FitScreen,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| {
        panic!("Error: {}", e);
    });

    let painter = TextPainter::new();
    let bounds = Bounds::new(-2.1, 2.1, WIDTH as f64 / HEIGHT as f64);
    let mut julia = JuliaFractal::new(bounds);
    let mut max_iterations = 670;
    let mut delta_time = 0.;

    let src = format!(r#"
        #define WIDTH {}
        #define HEIGHT {}

        double my_lerp(double a, double b, double w) {{
            return a + w * (b - a);
        }}

        __kernel void julia(__global __read_only  unsigned int* input, __global unsigned int* output, double re, double im,
            double4 bounds, __global __read_only unsigned int* color_lookup) {{

            unsigned int x = input[get_global_id(0)] % WIDTH;
            unsigned int y = input[get_global_id(0)] / WIDTH;
            double dx = ((double) x) / (double) WIDTH;
            double dy = ((double) y) / (double) HEIGHT;

            double zr = my_lerp(bounds.x, bounds.y, dx);
            double zi = my_lerp(bounds.z, bounds.w, dy);

            output[get_global_id(0)] = 0;

            for (int i = 0; i < color_lookup[0]; i++) {{
                if ((zr * zr + zi * zi) >= 4) {{
                    output[get_global_id(0)] = color_lookup[i + 1];
                    return;
                }}

                double tmpr = zr * zr - zi * zi;
                double tmpi = 2 * zr * zi;

                zr = tmpr + re;
                zi = tmpi + im;
            }}
        }}
    "#, WIDTH, HEIGHT);

    let pro_que = ProQue::builder()
        .src(src)
        .dims(WIDTH * HEIGHT)
        .build().unwrap();

    let mut gpu_input_buffer = pro_que.create_buffer::<u32>().unwrap();
    let input_buffer = {
        let mut v = vec![0; WIDTH * HEIGHT];

        let mut m = 0;
        for i in &mut v {
            *i = m;
            m += 1u32;
        }

        v
    };
    gpu_input_buffer.write(&input_buffer).enq();

    let gpu_output_buffer = pro_que.create_buffer::<u32>().unwrap();

    let gpu_color_lookup_buffer = pro_que.create_buffer::<u32>().unwrap();
    let mut color_lookup_buffer = calculate_color_lookup_buffer(max_iterations);
    gpu_color_lookup_buffer.write(&color_lookup_buffer).enq();

    let mut kernel = pro_que.kernel_builder("julia")
        .arg(&gpu_input_buffer)
        .arg(&gpu_output_buffer)
        .arg(julia.add.re)
        .arg(julia.add.im)
        .arg(&Double4::new(julia.bounds.xbounds.0, julia.bounds.xbounds.1, julia.bounds.ybounds.0, julia.bounds.ybounds.1))
        .arg(&gpu_color_lookup_buffer)
        .build().unwrap();

    println!("{:?}", julia);

    while window.is_open() && !window.is_key_released(Key::Escape) {
        let start = SystemTime::now();

        unsafe { kernel.enq(); }
        gpu_output_buffer.read(&mut window_buffer).enq();

        kernel.set_arg(4, &Double4::new(julia.bounds.xbounds.0, julia.bounds.xbounds.1, julia.bounds.ybounds.0, julia.bounds.ybounds.1));

        painter.paint_string(&mut window_buffer, WIDTH, 3, 3, &format!("{:?}", julia), FONT_SIZE, 0.5);
        painter.paint_string(&mut window_buffer, WIDTH, 3, FONT_SIZE + 10, &format!("iterations: {}", max_iterations), FONT_SIZE, 0.5);

        let time_passed = SystemTime::now().duration_since(start).unwrap();
        delta_time = time_passed.as_secs_f64();
        window.set_title(format!("{} DT: {}", TITLE, delta_time).as_ref());

        if window.is_key_pressed(Key::W, KeyRepeat::Yes) {
            max_iterations += 1;
            color_lookup_buffer = calculate_color_lookup_buffer(max_iterations);
            gpu_color_lookup_buffer.write(&color_lookup_buffer).enq();
        }
        if window.is_key_pressed(Key::S, KeyRepeat::Yes) {
            if max_iterations > 2 {
                max_iterations -= 1;
                color_lookup_buffer = calculate_color_lookup_buffer(max_iterations);
                gpu_color_lookup_buffer.write(&color_lookup_buffer).enq();
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
                kernel.set_arg(2, julia.add.re);
                kernel.set_arg(3, julia.add.im);
            }
        }

        window.update_with_buffer(&window_buffer, WIDTH, HEIGHT).unwrap();
    }
}

fn main() {
    render_ocl();
}
