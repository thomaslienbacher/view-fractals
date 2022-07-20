# Fractal Viewer

This program written in [Rust](https://www.rust-lang.org/) can be used to look at the
[Julia set](https://en.wikipedia.org/wiki/Julia_set) of fractals.
Compile using the `--release` flag for increased performance.

Licensed under the GNU General Public License v3.0 [GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html).

## Controls

| Action | Input |
| --- | --- |
| Move | Mouse Middle click | 
| Zoom in | Mouse Left click | 
| Zoom out | Mouse Right click | 
| Change iterations | W / S keys | 
| Change add parameter | Hold Space and move mouse | 
| Center view on 0,0 | U key | 
| Change to CPU renderer | 1 key | 
| Change to OpenCL (GPU) renderer | 2 key | 
| Save current view to image | 3 key | 
| Exit | Escape key | 

## OpenCL Renderer
The OpenCL renderer uses floats instead of doubles because my laptop doesn't support `cl_khr_fp64`.
This means it is faster but has reduced accuracy.
