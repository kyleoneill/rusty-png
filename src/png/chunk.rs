use crate::png::decode_error::DecodeError;
use crate::png::decode_error::DecodeError::*;
use crate::png::ImageMetadata;
use crc32fast::{hash, Hasher};
use winit::event::VirtualKeyCode::P;

pub struct ChunkReader {
    position: usize,
    bytes: Vec<u8>
}

impl ChunkReader {
    fn validate_file(bytes: &Vec<u8>) -> Result<(), DecodeError> {
        // A valid PNG is a minimum of 57 bytes. This covers the signature, which is 8 bytes,
        // an IHDR header chunk, which is 25 bytes (13 bytes of data), at least one IDAT chunk, and
        // an IEND chunk. Chunks are a minimum of 12 bytes; 4 for data length, 4 for type, and
        // 4 for a CRC. The data section can be empty)
        if bytes.len() < 57 {
            return Err(InvalidStructure())
        }
        let valid_signature: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
        if valid_signature != bytes[0..8] {
            return Err(InvalidSignature())
        }
        // The first chunk of every PNG must be the header. The header's first 8 bytes must
        // display that the data section is 13 bytes long and that the header type is b"IHDR"
        let valid_length: [u8; 4] = [0, 0, 0, 13];
        let valid_first_chunk_type = b"IHDR";
        if valid_length != bytes[8..12] || *valid_first_chunk_type != bytes[12..16] {
            return Err(InvalidHeader())
        }
        Ok(())
    }

    pub fn new(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        Self::validate_file(&bytes)?;
        // Initialize to position 33, the first byte after the signature and IHDR chunk
        Ok(ChunkReader { position: 33, bytes })
    }

    fn read_four_bytes_into_u32(&mut self) -> u32 {
        let arr = self.read_four_bytes_into_array();
        // PNG files are big endian (network ordering)
        u32::from_be_bytes(arr)
    }

    fn read_four_bytes_into_array(&mut self) -> [u8; 4] {
        let mut buf = [0u8; 4];
        buf.clone_from_slice(&self.bytes[self.position..self.position + 4]);
        self.position += 4;
        buf
    }

    fn read_chunk_data(&mut self, bytes_to_read: &u32) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();
        for b in 0..*bytes_to_read as usize {
            res.push(self.bytes[self.position + b].clone())
        }
        self.position += *bytes_to_read as usize;
        res
    }

    pub fn read_into_vec(&mut self, chunks: &mut Vec<Chunk>) -> Result<(), DecodeError> {
        while self.position < self.bytes.len() {
            let length = self.read_four_bytes_into_u32();
            let chunk_type = ChunkType::from_bytes(self.read_four_bytes_into_array());
            let chunk_data = self.read_chunk_data(&length);
            let crc = self.read_four_bytes_into_u32();
            let chunk = Chunk { length, chunk_type, chunk_data, crc };
            // TODO: Need to support PLTE chunks
            if chunk.chunk_type == ChunkType::PLTE {
                return Err(UnsupportedFeature("PLTE chunks are not yet supported".to_owned()));
            }
            if !chunk.crc_is_valid() {
                return Err(FailedChecksum());
            }
            if chunk.chunk_type != ChunkType::IEND {
                chunks.push(chunk);
            }
        }
        Ok(())
    }

    pub fn read_metadata(&self) -> ImageMetadata {
        // IHDR begin at position 8 and end at 33 (non-inclusive)
        // This means that IHDR's data begins at 16 and ends at 29 (non-inclusive)
        let mut buf = [0u8; 4];
        buf.clone_from_slice(&self.bytes[16..20]);
        let width = u32::from_be_bytes(buf);
        buf.clone_from_slice(&self.bytes[20..24]);
        let height = u32::from_be_bytes(buf);
        ImageMetadata {
            width,
            height,
            bit_depth: self.bytes[24].clone(),
            color_type: self.bytes[25].clone(),
            compression_method: self.bytes[26].clone(),
            filter_method: self.bytes[27].clone(),
            interlace_method: self.bytes[28].clone()
        }
    }
}

/// IHDR, IDAT, and IEND are the only supported chunk types. These represent the mandatory types,
/// the rest are optional. The first chunk must be an IHDR, there must be at least one IDAT, and
/// the final chunk must be an IEND.
#[derive(Debug, PartialEq)]
pub enum ChunkType {
    IHDR,
    IDAT,
    IEND,
    PLTE,
    Unknown
}

impl ChunkType {
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        match &bytes {
            b"IHDR" => Self::IHDR,
            b"IDAT" => Self::IDAT,
            b"IEND" => Self::IEND,
            b"PLTE" => Self::PLTE,
            // If a chunk type is unknown, we want to ignore the type. We don't need to error but we
            // still need to read the chunk so our file read continues correctly
            _ => Self::Unknown
        }
    }
    pub fn to_bytes(&self) -> &[u8; 4] {
        match self {
            Self::IHDR => b"IHDR",
            Self::IDAT => b"IDAT",
            Self::IEND => b"IEND",
            Self::PLTE => b"PLTE",
            _ => b"aaaa"
        }
    }
}

/// A chunk is made of 4 constituent parts. It begins with an unsigned 4 byte length (the length
/// of the data section, not the entire chunks length), then the chunk's type, the chunk data, and
/// a CRC.
pub struct Chunk {
    pub length: u32,
    pub chunk_type: ChunkType,
    pub chunk_data: Vec<u8>,
    pub crc: u32
}

// pub struct Chunk<'a> {
//     pub length: u32,
//     pub chunk_type: ChunkType,
//     pub chunk_data:Cow<'a, [u8]>, // &'a [u8] when borrowed, Vec<u8> when owned
//     pub crc: u32,
// }

impl Chunk {
    fn crc_is_valid(&self) -> bool {
        // CRC is calculated on the chunk type and chunk data but NOT the length field
        // This could be made more efficient, like verifying the CRC while constructing the chunk.
        // I shouldn't be converting [u8; 4] into a ChunkType and then back into a [u8; 4]
        let mut hasher = Hasher::new();
        let chunk_type = self.chunk_type.to_bytes().clone();
        //let data_to_checksum = chunk_type + self.chunk_data.clone();
        hasher.update(&chunk_type);
        hasher.update(&self.chunk_data[..]);
        let checksum = hasher.finalize();
        checksum == self.crc
    }
}