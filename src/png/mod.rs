use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt;
use std::fmt::Formatter;

use miniz_oxide::inflate::decompress_to_vec_zlib;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
    platform::windows::WindowExtWindows
};
use core::mem::MaybeUninit;
use std::mem::size_of;
use windows_sys::Win32::{
    Graphics::Gdi::{
        BeginPaint,
        CreateCompatibleDC,
        SelectObject, GetObjectA, BITMAP, BitBlt, SRCCOPY, DeleteDC, EndPaint, DeleteObject, CreateBitmap,
    },
};
use windows_sys::Win32::Graphics::Gdi::HBITMAP;

mod decode_error;
use decode_error::DecodeError;
use decode_error::DecodeError::*;

mod chunk;
use chunk::{Chunk, ChunkReader};

#[allow(dead_code)]
#[derive(Debug)]
pub struct ImageMetadata {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8
}

struct LastPixel {
    a: [u8; 4],
    b: [u8; 4],
    c: [u8; 4],
    d: [u8; 4]
}

impl LastPixel {
    fn new() -> Self {
        Self {
            a: [0u8; 4],
            b: [0u8; 4],
            c: [0u8; 4],
            d: [0u8; 4],
        }
    }

    fn from_decoded(decoded_bytes: &[u8], x: usize, y: usize, width: u32) -> Self {
        fn get_pixel(decoded_bytes: &[u8], x: isize, y: isize, width: isize) -> [u8; 4] {
            if x < 0 || y < 0 {
                return [0u8; 4];
            }
            // pixel_index = pixels from current line + all pixels on each line before us
            let pixel_index = (x + (y * width)) as usize;
            // byte_index = pixel_index * 4, each pixel is 4 bytes (BGRA) in our decoded output to Windows
            let byte_index = pixel_index * 4;
            let mut buf = [0u8; 4];
            buf.clone_from_slice(&decoded_bytes[byte_index..byte_index + 4]);
            buf
        }
        let x = x as isize;
        let y = y as isize;
        let width = width as isize;
        let a = get_pixel(decoded_bytes, x - 1, y, width);
        let b = get_pixel(decoded_bytes, x, y - 1, width);
        let c = get_pixel(decoded_bytes, x - 1, y - 1, width);
        let d = get_pixel(decoded_bytes, x + 1, y - 1, width);
        Self { a, b, c, d}
    }

    fn paeth(&self, i: usize) -> u8 {
        let a = self.a[i] as i16;
        let b = self.b[i] as i16;
        let c = self.c[i] as i16;
        let p = a + b - c;
        let p_a = (p - a).abs();
        let p_b = (p - b).abs();
        let p_c = (p - c).abs();
        if p_a <= p_b && p_a <= p_c {
            a as u8
        }
        else if p_b <= p_c  {
            b as u8
        }
        else {
            c as u8
        }
    }
}

pub struct PNG {
    chunks: Vec<Chunk>,
    metadata: ImageMetadata,
    name: String
}

impl fmt::Debug for PNG {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PNG")
            .field("metadata", &self.metadata)
            .field("chunk_length", &self.chunks.len())
            .finish()
    }
}

impl PNG {
    pub fn from_file_path(fp: &str) -> Result<Self, DecodeError> {
        let file_path = Path::new(fp);
        if !file_path.exists() {
            return Err(BadFilePath(fp.to_owned()))
        }
        match File::open(file_path) {
            Ok(mut file) => {
                let mut file_contents: Vec<u8> = Vec::new();
                match file.read_to_end(&mut file_contents) {
                    Ok(_file_size) => (),
                    Err(_e) => return Err(FailedToReadFile(fp.to_owned()))
                }
                let mut reader = ChunkReader::new(file_contents)?;
                let mut chunks: Vec<Chunk> = Vec::new();
                reader.read_into_vec(&mut chunks)?;
                let metadata = reader.read_metadata()?;
                // TODO: I should not be unwrapping
                let name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();
                Ok(Self {chunks, metadata, name })
            }
            Err(_e) => Err(FailedToOpenFile(fp.to_owned()))
        }
    }

    fn filter_decoded_data(&self, unfiltered: Vec<u8>) -> Result<Vec<u8>, DecodeError> {
        let mut finalized_data: Vec<u8> = Vec::new();
        // Each scanline has a length of (1 + width * bytes_per_pixel)
        let bytes_per_pixel = self.get_number_of_channels()? as usize;
        let scanline_width = 1 + self.metadata.width as usize * bytes_per_pixel;

        for y in 0..self.metadata.height as usize {
            // This gets the current scanline as a slice. It runs from the start of the current
            // y to its end
            let scanline: &[u8] = &unfiltered[y * scanline_width..(y + 1) * scanline_width];
            let mut last_pixel = LastPixel::new();
            for x in 0..self.metadata.width as usize {
                last_pixel = LastPixel::from_decoded(&finalized_data, x, y, self.metadata.width);

                // We need to index on scanline[x+1..x+1+bytes_per_pixel] because we need to
                // account for the filter byte at the start of each scanline
                let pixel: &[u8] = &scanline[x * bytes_per_pixel + 1..x * bytes_per_pixel + 1 + bytes_per_pixel];

                // TODO: I am not handling all combinations of what the pixels can be here
                // Ex, for color_type 6 there are 4 pixels in the unfiltered_data but for
                // color_type 2 there are only 3. I need to still handle 1, 2, and palette

                // We want to swap from RGBA to BGRA, thanks Windows
                let mut bgra = [pixel[2], pixel[1], pixel[0], 255];
                match self.metadata.color_type {
                    6 => bgra[3] = pixel[3],
                    _ => ()
                };
                for i in 0..4 {
                    bgra[i] = match scanline[0] {
                        0 => {
                            bgra[i]
                        },
                        1 => {
                            bgra[i].wrapping_add(last_pixel.a[i])
                        },
                        2 => {
                            bgra[i].wrapping_add(last_pixel.b[i])
                        },
                        3 => {
                            bgra[i].wrapping_add(((last_pixel.a[i] as u16 + last_pixel.b[i] as u16) / 2) as u8)
                        },
                        4 => {
                            bgra[i].wrapping_add(last_pixel.paeth(i))
                        },
                        _ => return Err(InvalidScanlineFilter())
                    }
                }
                for i in bgra {
                    finalized_data.push(i);
                }
            }
        }
        Ok(finalized_data)
    }

    fn get_decoded_chunk_data(&mut self) -> Result<Vec<u8>, DecodeError> {
        // TODO: I think that I should be storing the IDAT data in the PNG struct rather than chunks
        // Chunks might only be meant to be used to read data during transfer/decoding, not as a
        // storage mechanism. Will have to see how other chunks, like PLTE, affect the rendering
        // or reading of IDAT data.

        // Using this method either doubles the memory size of the PNG (data is doubled, we are
        // storing the compressed data-stream twice and then the compressed data-stream once and
        // the uncompressed once) if we copy chunk.chunk_data or empties all chunk chunk_data fields
        let mut data: Vec<u8> = Vec::new();
        for chunk in &mut self.chunks {
            if chunk.chunk_type == b"IDAT".to_owned() {
                data.append(&mut chunk.chunk_data);
            }
            // TODO: What do I do with non IDAT chunks?
        }
        let decoded_data = match decompress_to_vec_zlib(&data[..]) {
            Ok(decoded) => decoded,
            Err(e) => {
                eprintln!("Failed to inflate compressed data with error: {}", e);
                return Err(FailedDecoding());
            }
        };
        let filtered_data = self.filter_decoded_data(decoded_data)?;
        Ok(filtered_data)
    }

    fn get_number_of_channels(&self) -> Result<u32, DecodeError> {
        match self.metadata.color_type {
            0 => Ok(1 as u32),
            2 => {
                if self.metadata.bit_depth == 8 || self.metadata.bit_depth == 16 {
                    Ok(3 as u32)
                }
                else {
                    return Err(InvalidStructure())
                }
            },
            3 => Err(UnsupportedFeature("PLTE chunks are not yet supported".to_owned())),
            4 => {
                if self.metadata.bit_depth == 8 || self.metadata.bit_depth == 16 {
                    Ok(2 as u32)
                }
                else {
                    return Err(InvalidStructure())
                }
            }
            6 => {
                if self.metadata.bit_depth == 8 || self.metadata.bit_depth == 16 {
                    Ok(4 as u32)
                }
                else {
                    return Err(InvalidStructure())
                }
            },
            _ => return Err(InvalidStructure())
        }
    }

    fn create_bitmap(&mut self) -> Result<HBITMAP, DecodeError> {
        let render_data = self.get_decoded_chunk_data()?;
        // let mut image_data = vec![128u8; 256 * 256 * 4];
        // for x in 0..256 {
        //     for y in 0..256 {
        //         let index = x + y * 256;
        //         // B
        //         image_data[index * number_of_channels as usize + 0] = x as u8;
        //         // G
        //         image_data[index * number_of_channels as usize + 1] = y as u8;
        //         // R
        //         image_data[index * number_of_channels as usize + 2] = 128;
        //         // A
        //         image_data[index * number_of_channels as usize + 3] = 255;
        //     }
        // }
        unsafe {
            // nbitcount is 8 * 4 rather than 8 * number_of_channels because we always decode back to
            // BGRA, even if the PNG is just B/W or RGB
            Ok(CreateBitmap(self.metadata.width as i32, self.metadata.height as i32, 1, 8 * 4, render_data.as_ptr().cast()))
        }
    }

    pub fn show(&mut self) -> Result<(), DecodeError> {
        eprintln!("Displaying image with data:\n{:?}", self);
        let h_bitmap = self.create_bitmap()?;

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(format!("{}.png", &self.name))
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(self.metadata.width, self.metadata.height))
            .build(&event_loop)
            .expect("Failed to create window");

        event_loop.run(move |event, _, control_flow| {
            control_flow.set_poll();

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    unsafe {
                        DeleteObject(h_bitmap);
                    }
                    control_flow.set_exit();
                }
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    unsafe {
                        let h_wnd = window.hwnd();

                        let mut ps = MaybeUninit::zeroed();
                        let hdc = BeginPaint(h_wnd, ps.as_mut_ptr());
                        let ps = ps.assume_init();

                        let hdc_mem = CreateCompatibleDC(hdc);
                        let old_bitmap = SelectObject(hdc_mem, h_bitmap);

                        let mut bitmap = MaybeUninit::<BITMAP>::zeroed();
                        GetObjectA(h_bitmap, size_of::<BITMAP>() as i32, bitmap.as_mut_ptr().cast());
                        let bitmap = bitmap.assume_init();
                        BitBlt(hdc, 0, 0, bitmap.bmWidth, bitmap.bmHeight, hdc_mem, 0, 0, SRCCOPY);

                        SelectObject(hdc_mem, old_bitmap);
                        DeleteDC(hdc_mem);

                        EndPaint(h_wnd, &ps);
                    }
                }
                _ => (),
            }
        });
    }
}
