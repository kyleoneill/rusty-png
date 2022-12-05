use crate::png::decode_error::DecodeError;
use crate::png::decode_error::DecodeError::*;

pub struct ChunkReader {
    position: usize,
    bytes: Vec<u8>
}

impl ChunkReader {
    fn signature_is_valid(signature: &[u8]) -> bool {
        let valid_signature: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
        return valid_signature == *signature
    }

    pub fn new(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        // A valid PNG is a minimum of 44 bytes. This covers the signature, which is 8 bytes,
        // and 3 chunks. A chunk is a minimum of 12 bytes (length, type, CRC at 4 bytes each. The
        // data section can be empty). A PNG must have a minimum of 3 chunks, an IHDR, an IDAT, and
        // an IEND
        if bytes.len() < 44 {
            return Err(InvalidStructure())
        }
        if !Self::signature_is_valid(&bytes[0..8]) {
            return Err(InvalidSignature())
        }
        Ok(ChunkReader { position: 8, bytes })
    }

    fn read_four_bytes(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        buf.clone_from_slice(&self.bytes[self.position..self.position + 4]);
        u32::from_be_bytes(buf)
    }

    pub fn read_into_vec(&mut self, chunks: &mut Vec<Chunk>) -> Result<(), DecodeError> {
        // The first chunk must be an IHDR
        // The final chunk must be an IEND
        // If we run out of bytes without hitting an IEND, it is an error
        while self.position < self.bytes.len() {
            // Length is big-endian
            let length = self.read_four_bytes();
            break;
            // create chunk
            // add chunk to chunk vec
        }
        Ok(())
    }
}

/// IHDR, IDAT, and IEND are the only supported chunk types. These represent the mandatory types,
/// the rest are optional.
pub enum ChunkType {
    IHDR,
    IDAT,
    IEND
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
