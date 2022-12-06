use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt;
use std::fmt::Formatter;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod decode_error;
use decode_error::DecodeError;
use decode_error::DecodeError::*;

mod chunk;
use chunk::{Chunk, ChunkReader};

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
                let metadata = reader.read_metadata();
                // TODO: I should not be unwrapping
                let name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();
                Ok(Self {chunks, metadata, name })
            }
            Err(_e) => Err(FailedToOpenFile(fp.to_owned()))
        }
    }

    fn get_chunk_data(&mut self) -> Vec<u8> {
        // TODO: I think that I should be storing the IDAT data in the PNG struct rather than chunks
        // Chunks might only be meant to be used to read data during transfer/decoding, not as a
        // storage mechanism. Will have to see how other chunks, like PLTE, affect the rendering
        // or reading of IDAT data.

        // Using this method either doubles the memory size of the PNG (data is doubled, we are
        // storing the compressed data-stream twice and then the compressed data-stream once and
        // the uncompressed once) if we copy chunk.chunk_data or empties all chunk chunk_data fields
        let mut data: Vec<u8> = Vec::new();
        for mut chunk in &mut self.chunks {
            data.append(&mut chunk.chunk_data);
        }
        data
    }

    pub fn show(&mut self) {
        println!("Displaying image with data:\n{:?}", self);
        let render_data = self.get_chunk_data();
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(&self.name)
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(self.metadata.width, self.metadata.height))
            .build(&event_loop)
            .expect("Failed to create window");

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        });
    }
}
