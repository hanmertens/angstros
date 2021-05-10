#![no_std]
#![no_main]

use core::{mem, panic::PanicInfo, slice};
use os::sys::PixelFormat;
use volatile::Volatile;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C, align(4))]
pub struct Pixel {
    a: u8,
    b: u8,
    c: u8,
}

impl Pixel {
    pub fn new(r: u8, g: u8, b: u8, format: PixelFormat) -> Self {
        match format {
            PixelFormat::Rgb => Self { a: r, b: g, c: b },
            PixelFormat::Bgr => Self { a: b, b: g, c: r },
        }
    }
}

pub struct FrameBuffer {
    buf: Volatile<&'static mut [Pixel]>,
    shape: (usize, usize),
    stride: usize,
    format: PixelFormat,
}

#[no_mangle]
extern "C" fn _start() {
    os::log("Obtaining screen access...");
    let fb = os::frame_buffer();
    if let Some(fb) = fb {
        os::log("Screen access obtained!");
        let buf = unsafe {
            slice::from_raw_parts_mut(fb.ptr as *mut Pixel, fb.size / mem::size_of::<Pixel>())
        };
        let mut fb = FrameBuffer {
            buf: Volatile::new(buf),
            shape: fb.shape,
            stride: fb.stride,
            format: fb.format,
        };
        let (w, h) = fb.shape;
        for y in 0..h {
            for x in 0..w {
                let r = 0xff * x / w;
                let g = 0xff * y / h;
                let b = 0xff;
                fb.buf
                    .index_mut(y * fb.stride + x)
                    .write(Pixel::new(r as u8, g as u8, b, fb.format));
            }
        }
    } else {
        os::log("Screen access not granted");
        os::exit(2);
    }
    os::exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    os::log("panic!");
    os::exit(1);
}
