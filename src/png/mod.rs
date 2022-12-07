use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt;
use std::fmt::Formatter;

use inflate::inflate_bytes_zlib;
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
            for x in 0..self.metadata.width as usize {
                // We need to index on scanline[x+1..x+1+bytes_per_pixel] because we need to
                // account for the filter byte at the start of each scanline
                let pixel: &[u8] = &scanline[x * bytes_per_pixel + 1..x * bytes_per_pixel + 1 + bytes_per_pixel];
                let red = pixel[0];
                let green = pixel[1];
                let blue = pixel[2];
                let alpha = pixel[3];
                // TODO: Implement filtering
                match scanline[0] {
                    0 => {

                    },
                    1 => {

                    },
                    2 => {

                    },
                    3 => {

                    },
                    4 => {

                    },
                    _ => return Err(InvalidScanlineFilter())
                }
                // We want to swap from RBGA to BGRA, thanks Windows
                finalized_data.push(blue);
                finalized_data.push(green);
                finalized_data.push(red);
                finalized_data.push(alpha);
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
            data.append(&mut chunk.chunk_data);
        }
        // inflate_bytes_zlib_no_checksum(&data[..])
        let decoded_data = match inflate_bytes_zlib(&data[..]) {
            Ok(decoded) => decoded,
            Err(e) => {
                eprintln!("Failed to inflate compressed data with error: {}", e.as_str());
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
        let number_of_channels = self.get_number_of_channels()?;
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
            Ok(CreateBitmap(self.metadata.width as i32, self.metadata.height as i32, 1, 8 * number_of_channels, render_data.as_ptr().cast()))
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
