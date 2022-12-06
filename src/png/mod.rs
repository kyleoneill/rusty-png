use std::path::Path;
use std::fs::File;
use std::io::Read;

mod decode_error;
use decode_error::DecodeError;
use decode_error::DecodeError::*;

mod chunk;
use chunk::{Chunk, ChunkReader};

pub struct PNG {
    chunks: Vec<Chunk>
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
                Ok(Self {chunks})
            }
            Err(_e) => Err(FailedToOpenFile(fp.to_owned()))
        }
    }

    pub fn show(&self) {
        println!("PNG contains {0} chunk(s)", self.chunks.len());
    }
}
