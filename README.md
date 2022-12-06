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
A PNG is comprised of an 8 byte signature followed by chunks. Chunks have types,
where the mandatory types are:
- `IHDR` (image header)
- `IDAT` (image data)
- `IEND` (image end)

There are other chunk types, but support for them is not mandatory. A PNG must
contain, at minimum, one of each of the mandatory chunks.

A chunk is made of four parts:
- 4 bytes describing the chunk data length
  - This is _not_ the length of the chunk, this is the length of the data section
- 4 bytes describing the chunk type
- The chunk data, of length determined by the first part
- 4 bytes describing the CRC of the chunk

The `IHDR` chunk must always have 13 bytes in its data section, as it describes
the metadata of the file.

This means that a PNG file must be a minimum of 57 bytes
- 8 byte signature
- 25 byte `IHDR`
- At least 12 byte `IDATA`
- 12 byte `IEND`
