use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::fmt;
use std::fmt::Formatter;

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
    metadata: ImageMetadata
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
                Ok(Self {chunks, metadata})
            }
            Err(_e) => Err(FailedToOpenFile(fp.to_owned()))
        }
    }

    pub fn show(&self) {
        // println!("PNG contains {0} chunk(s)", self.chunks.len());
        // println!("Resolution is {0}x{1}", self.metadata.width, self.metadata.height);
        println!("{:?}", self);
    }
}
