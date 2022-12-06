# Rusty PNG
This extremely creatively named repo is my implementation of a PNG decoder,
made as a hackathon project.

It is written in Rust and can be ran with:
```sh
cargo run {path_to_png_file}
```

## How does it work
The PNG library decodes a PNG file and then displays it.

### Decoding a PNG
A PNG is composed of an 8 byte signature followed by chunks. All integers are
big endian (network byte order), so the highest bit value of a byte (128) is
bit 7 while the lowest bit (1) is bit 0. Integers are unsigned unless noted
and signed values are represented in two's complement.

A chunk is made of four parts:
- 4 bytes describing the chunk data length
  - This is _not_ the length of the chunk, this is the length of the data section
- 4 bytes describing the chunk type
- The chunk data, of length determined by the first part. This can be empty (0 bytes)
- 4 bytes describing the CRC of the chunk

#### Mandatory chunks
The mandatory types are:
- `IHDR` (image header)
- `IDAT` (image data)
- `IEND` (image end)

Every PNG must contain at least one of each of these chunks. `IHDR` is the
header and describes metadata about the image that is necessary to display it.
`IDAT` is a data chunk and `IEND` signifies the end of the file. The first byte
is upper-case for mandatory chunks.

The `IHDR` chunk must always have 13 bytes in its data section, as it describes
the metadata of the file.

This means that a PNG file must be a minimum of 57 bytes
- 8 byte signature
- 25 byte `IHDR`
- At least 12 byte `IDATA`
- 12 byte `IEND`

#### Optional (ancillary) chunks
Optional chunks are signified by a lower-case first byte. the `tIME` chunk is
optional.
