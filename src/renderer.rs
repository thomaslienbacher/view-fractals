use super::fractal::*;
use palette::{LinSrgb, Hsv, Gradient};
use rayon::prelude::*;
use num::complex::Complex64;
use ocl::{ProQue, Buffer, Kernel};
use ocl::prm::Float4;
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
        Hsv::new(0.0, 1.0, 1.0),
        Hsv::new(180.0, 1.0, 1.0),
        Hsv::new(280.0, 1.0, 1.0),
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
                (j as f32) / m as f32
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

        float my_lerp(float a, float b, float w) {{
            return a + w * (b - a);
        }}

        __kernel void julia(__global  unsigned int* input, __global unsigned int* output, float re, float im,
            float4 bounds, __global unsigned int* color_lookup) {{

            unsigned int x = input[get_global_id(0)] % WIDTH;
            unsigned int y = input[get_global_id(0)] / WIDTH;
            float dx = ((float) x) / (float) WIDTH;
            float dy = ((float) y) / (float) HEIGHT;

            float zr = my_lerp(bounds.x, bounds.y, dx);
            float zi = my_lerp(bounds.z, bounds.w, dy);

            output[get_global_id(0)] = 0;

            for (int i = 0; i < color_lookup[0]; i++) {{
                if ((zr * zr + zi * zi) >= 4) {{
                    output[get_global_id(0)] = color_lookup[i + 1];
                    return;
                }}

                float tmpr = zr * zr - zi * zi;
                float tmpi = 2 * zr * zi;

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
        gpu_input_buffer.write(&input_buffer).enq().unwrap();

        let gpu_output_buffer = pro_que.create_buffer::<u32>().unwrap();

        let gpu_color_lookup_buffer = pro_que.create_buffer::<u32>().unwrap();
        let color_lookup_buffer = OpenClRenderer::calculate_color_lookup_buffer(max_iterations as usize);
        gpu_color_lookup_buffer.write(&color_lookup_buffer).enq().unwrap();

        let kernel = pro_que.kernel_builder("julia")
            .arg(&gpu_input_buffer)
            .arg(&gpu_output_buffer)
            .arg(julia.add.re as f32)
            .arg(julia.add.im as f32)
            .arg(&Float4::new(julia.bounds.xbounds.0 as f32, julia.bounds.xbounds.1 as f32,
                              julia.bounds.ybounds.0 as f32, julia.bounds.ybounds.1 as f32))
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

        const GRADIENT_LOOP: usize = 40;

        for (i, c) in v.iter_mut().skip(1).enumerate() {
            let g = {
                if max_iterations < GRADIENT_LOOP {
                    let m = max_iterations - 1;
                    i as f32 / m as f32
                } else {
                    (i % GRADIENT_LOOP) as f32 / GRADIENT_LOOP as f32
                }
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
        unsafe { self.kernel.enq().unwrap(); }
        self.gpu_output_buffer.read(window_buffer).enq().unwrap();
    }

    fn on_add_change(&mut self, add: &Complex64) {
        self.kernel.set_arg(2, add.re as f32).unwrap();
        self.kernel.set_arg(3, add.im as f32).unwrap();
    }

    fn on_bounds_change(&mut self, bounds: &Bounds) {
        self.kernel.set_arg(4, &Float4::new(bounds.xbounds.0 as f32, bounds.xbounds.1 as f32,
                                            bounds.ybounds.0 as f32, bounds.ybounds.1 as f32)).unwrap();
    }

    fn on_max_iterations_change(&mut self, max_iterations: u32) {
        let color_lookup_buffer = OpenClRenderer::calculate_color_lookup_buffer(max_iterations as usize);
        self.gpu_color_lookup_buffer.write(&color_lookup_buffer).enq().unwrap();
    }
}

pub struct PNGSaver {
    internal: OpenClRenderer,
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
        let mut w = &mut BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, Self::WIDTH as u32, Self::HEIGHT as u32); // Width is 2 pixels and height is 1.
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let mut buffer: Vec<u32> = vec![0; Self::WIDTH * Self::HEIGHT];
        self.internal.render(julia, Self::MAX_ITERATIONS, &mut buffer, Self::WIDTH, Self::HEIGHT);

        println!("Preparing data...");
        let data: Vec<u8> = {
            let d: Vec<Vec<u8>> = buffer.iter().map(|u| {
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

