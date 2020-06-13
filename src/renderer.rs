use super::fractal::*;
use palette::{LinSrgb, Hsv, Gradient};
use rayon::prelude::*;
use num::complex::Complex64;
use ocl::{ProQue, Buffer, Kernel};
use ocl::prm::Double4;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

pub trait Renderer {
    fn name(&self) -> String;
    fn render(&mut self, julia: &JuliaFractal, max_iterations: u32, window_buffer: &mut Vec<u32>, width: usize, height: usize);
    fn on_add_change(&mut self, add: &Complex64);
    fn on_bounds_change(&mut self, bounds: &Bounds);
    fn on_max_iterations_change(&mut self, max_iterations: u32);
}

fn get_color_gradient() -> Gradient<Hsv> {
    Gradient::new(vec![
        Hsv::from(LinSrgb::new(0.2 as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(1. as f32, 0. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 1. as f32, 0. as f32)),
        Hsv::from(LinSrgb::new(0. as f32, 0. as f32, 1. as f32)),
        Hsv::from(LinSrgb::new(1. as f32, 1. as f32, 1. as f32))
    ])
}

pub struct CpuRenderer {}

impl CpuRenderer {
    pub fn new() -> CpuRenderer {
        CpuRenderer {}
    }
}

impl Renderer for CpuRenderer {
    fn name(&self) -> String {
        "CPU Multi threaded".into()
    }

    fn render(&mut self, julia: &JuliaFractal, max_iterations: u32, window_buffer: &mut Vec<u32>, width: usize, height: usize) {
        let grad = get_color_gradient();

        window_buffer.par_iter_mut().enumerate().for_each(|(i, v)| {
            let x = (i % width) as f64 / width as f64;
            let y = (i / width) as f64 / height as f64;
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
    }

    fn on_add_change(&mut self, _add: &Complex64) {}

    fn on_bounds_change(&mut self, _bounds: &Bounds) {}

    fn on_max_iterations_change(&mut self, _max_iterations: u32) {}
}

pub struct OpenClRenderer {
    kernel: Kernel,
    gpu_output_buffer: Buffer<u32>,
    gpu_color_lookup_buffer: Buffer<u32>,
}

impl OpenClRenderer {
    pub fn new(width: usize, height: usize, julia: &JuliaFractal, max_iterations: u32) -> OpenClRenderer {
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
    "#, width, height);

        let pro_que = ProQue::builder()
            .src(src)
            .dims(width * height)
            .build().unwrap();

        let gpu_input_buffer = pro_que.create_buffer::<u32>().unwrap();
        let input_buffer = {
            let mut v = vec![0; width * height];

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
        let color_lookup_buffer = OpenClRenderer::calculate_color_lookup_buffer(max_iterations as usize);
        gpu_color_lookup_buffer.write(&color_lookup_buffer).enq();

        let kernel = pro_que.kernel_builder("julia")
            .arg(&gpu_input_buffer)
            .arg(&gpu_output_buffer)
            .arg(julia.add.re)
            .arg(julia.add.im)
            .arg(&Double4::new(julia.bounds.xbounds.0, julia.bounds.xbounds.1, julia.bounds.ybounds.0, julia.bounds.ybounds.1))
            .arg(&gpu_color_lookup_buffer)
            .build().unwrap();

        OpenClRenderer {
            kernel,
            gpu_color_lookup_buffer,
            gpu_output_buffer,
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
}

impl Renderer for OpenClRenderer {
    fn name(&self) -> String {
        "OpenCL Frame".into()
    }

    fn render(&mut self, _julia: &JuliaFractal, _max_iterations: u32, window_buffer: &mut Vec<u32>, _width: usize, _height: usize) {
        unsafe { self.kernel.enq(); }
        self.gpu_output_buffer.read(window_buffer).enq();
    }

    fn on_add_change(&mut self, add: &Complex64) {
        self.kernel.set_arg(2, add.re);
        self.kernel.set_arg(3, add.im);
    }

    fn on_bounds_change(&mut self, bounds: &Bounds) {
        self.kernel.set_arg(4, &Double4::new(bounds.xbounds.0, bounds.xbounds.1, bounds.ybounds.0, bounds.ybounds.1));
    }

    fn on_max_iterations_change(&mut self, max_iterations: u32) {
        let color_lookup_buffer = OpenClRenderer::calculate_color_lookup_buffer(max_iterations as usize);
        self.gpu_color_lookup_buffer.write(&color_lookup_buffer).enq();
    }
}

pub struct PNGSaver {
    internal: OpenClRenderer
}

impl PNGSaver {
    const WIDTH: usize = crate::WIDTH * 9;
    const HEIGHT: usize = crate::HEIGHT * 9;
    const MAX_ITERATIONS: u32 = 1000;

    pub fn new(julia: &JuliaFractal) -> PNGSaver {
        PNGSaver {
            internal: OpenClRenderer::new(Self::WIDTH, Self::HEIGHT, julia, Self::MAX_ITERATIONS)
        }
    }

    pub fn save(&mut self, julia: &JuliaFractal) {
        println!("Starting to save PNG...");
        let path = Path::new("render.png");
        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, Self::WIDTH as u32, Self::HEIGHT as u32); // Width is 2 pixels and height is 1.
        encoder.set_color(png::ColorType::RGB);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let mut buffer: Vec<u32> = vec![0; Self::WIDTH * Self::HEIGHT];
        self.internal.render(julia, Self::MAX_ITERATIONS, &mut buffer, Self::WIDTH, Self::HEIGHT);

        println!("Preparing data...");
        let data: Vec<u8> = {
            let mut d: Vec<Vec<u8>> = buffer.iter().map(|u| {
                let r = (*u & 0x00FF0000) >> 16;
                let g = (*u & 0x0000FF00) >> 8;
                let b = *u & 0x000000FF;

                vec![r as u8, g as u8, b as u8]
            }).collect();

            d.into_iter().flatten().collect()
        };

        println!("Writing data...");
        writer.write_image_data(&data[..]).unwrap();
        println!("Finished writing to PNG!");
    }
}

