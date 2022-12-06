use std::env;

mod png;

use crate::png::PNG;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("You need to provide the path of a png to read.");
    }
    match PNG::from_file_path(&args[1]) {
        Ok(mut image) => image.show(),
        Err(error) => panic!("{}", error)
    }
}
