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
            // TODO: I should actually do something with the crc, like verify the chunk with it
            let crc = self.read_four_bytes_into_u32();
            let chunk = Chunk { length, chunk_type, chunk_data, crc };
            chunks.push(chunk);
        }
        Ok(())
    }
}

/// IHDR, IDAT, and IEND are the only supported chunk types. These represent the mandatory types,
/// the rest are optional. The first chunk must be an IHDR, there must be at least one IDAT, and
/// the final chunk must be an IEND.
pub enum ChunkType {
    IHDR,
    IDAT,
    IEND,
    Unknown
}

impl ChunkType {
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        match &bytes {
            b"IHDR" => Self::IHDR,
            b"IDAT" => Self::IDAT,
            b"IEND" => Self::IEND,
            // If a chunk type is unknown, we want to ignore the type. We don't need to error but we
            // still need to read the chunk so our file read continues correctly
            _ => Self::Unknown
        }
        // match as_num {
        //     1229472850 => Ok(Self::IHDR),
        //     1229209940 => Ok(Self::IDAT),
        //     1229278788 => Ok(Self::IEND),
        //     _ => Err(UnsupportedChunkType(as_num))
        // }
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
