extern crate binary_diff;
extern crate clap;

use binary_diff::{BinaryDiff, BinaryDiffAnalyzer, BinaryDiffChunk};
use clap::{App, Arg};
use std::io::BufReader;
use std::path::Path;

fn main() {
    env_logger::init();

    let matches = App::new("Binary diff tool")
        .version("1.0")
        .author("Nao Tomori (@K_atc)")
        .about("Show changes between two binaries. Each of value is hex (16 digit) value")
        .arg(
            Arg::with_name("same")
                .long("same")
                .help("Enables to output Same chunks")
                .takes_value(false),
        )
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
        .arg(
            Arg::with_name("OFFSET")
                .long("offset")
                .help("Analyzes given offset of patched file derives from which diff chunk")
                .takes_value(true),
        )
        .get_matches();

    let diff = match (matches.value_of("FILE1"), matches.value_of("FILE2")) {
        (Some(file_path_1), Some(file_path_2)) => {
            let file_1 = std::fs::File::open(file_path_1).unwrap();
            let file_2 = std::fs::File::open(file_path_2).unwrap();
            BinaryDiff::new(&mut BufReader::new(file_1), &mut BufReader::new(file_2)).unwrap()
        }
        _ => {
            panic!("[!] Parameter FILE1 or FILE2 is not specified");
        }
    };

    if matches.is_present("OFFSET") {
        let offset = usize::from_str_radix(matches.value_of("OFFSET").unwrap(), 16).unwrap();
        let patched_file =
            std::fs::File::open(Path::new(matches.value_of("FILE2").unwrap())).unwrap();

        let enhanced_diff = diff.enhance();
        let mut analyzer = BinaryDiffAnalyzer::new(&enhanced_diff, &patched_file);
        match analyzer.derives_from(offset).unwrap() {
            Some(derives_from) => println!("{}", derives_from.chunk),
            None => eprintln!("[!] offset={:#x} does not derive from no chunks", offset),
        }
    } else {
        let print_same_chunks = matches.is_present("same");
        for chunk in diff.enhance().chunks() {
            let print = match chunk {
                BinaryDiffChunk::Same(_, _) => print_same_chunks,
                _ => true,
            };
            if print {
                println!("{}", chunk);
            }
        }
    }
}
