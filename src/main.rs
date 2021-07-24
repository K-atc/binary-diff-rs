// #![no_std]
// Cannot apply `no_std` since BufReader is std::io::BufReader

pub mod binary_diff;

// extern crate alloc;
extern crate bcmp;
extern crate clap;
extern crate log;

use crate::binary_diff::binary_diff;
use clap::{App, Arg};
use std::io::BufReader;

fn main() {
    let matches = App::new("Binary diff tool")
        .version("1.0")
        .author("Nao Tomori (@K_atc)")
        .about("Show changes between two binaries. Each of value is hex (16 digit) value")
        .arg(
            Arg::with_name("FILE1")
                .help("Original file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("FILE2")
                .help("Patched file")
                .required(true)
                .index(2),
        )
        .get_matches();

    let diff_chunks = match (matches.value_of("FILE1"), matches.value_of("FILE2")) {
        (Some(file_path_1), Some(file_path_2)) => {
            let file_1 = std::fs::File::open(file_path_1).unwrap();
            let file_2 = std::fs::File::open(file_path_2).unwrap();
            binary_diff(&mut BufReader::new(file_1), &mut BufReader::new(file_2)).unwrap()
        }
        _ => {
            panic!("[!] Parameter FILE1 or FILE2 is not specified");
        }
    };

    for chunk in diff_chunks {
        println!("{}", chunk);
    }
}
