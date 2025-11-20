use clap::Parser;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about = "Read and write binary files in hexadecimal", long_about = None)]
struct Args {
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    read: bool,

    #[arg(short, long)]
    write: Option<String>,

    /// Byte offset (supports decimal or hexadecimal with 0x prefix, e.g., 16 or 0x10)
    #[arg(short, long, default_value_t = 0, value_parser = parse_offset)]
    offset: u64,

    #[arg(short, long)]
    size: Option<usize>,
}

// Custom parsing function: supports both decimal and hexadecimal offsets
fn parse_offset(s: &str) -> Result<u64, String> {
    // Handle hexadecimal (0x prefix)
    let s = s.trim();
    let base = if s.starts_with("0x") {
        16
    } else {
        10 // Default to decimal
    };

    u64::from_str_radix(if base == 16 { &s[2..] } else { s }, base)
        .map_err(|_| format!("Invalid offset: {}", s))
}

fn main() {
    let args = Args::parse();

    let mut file = File::options()
        .read(true)
        .write(true)
        .open(&args.file)
        .expect("Failed to open file");

    if args.read {
        file.seek(SeekFrom::Start(args.offset))
            .expect("Failed to seek to offset");

        let size = args.size.unwrap_or(32);
        let mut buffer = vec![0u8; size];
        let bytes_read = file.read(&mut buffer).expect("Failed to read file");

        print_hex(&buffer[0..bytes_read], args.offset);
    } else if let Some(hex_str) = args.write {
        let bytes: Vec<u8> = hex_str
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16))
            .collect::<Result<_, _>>()
            .expect("Invalid hexadecimal string");

        file.seek(SeekFrom::Start(args.offset))
            .expect("Failed to seek to offset");
        file.write_all(&bytes)
            .expect("Failed to write to file");
    }
}

fn print_hex(data: &[u8], offset: u64) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let chunk_offset = offset + (i * 16) as u64;
        print!("{:08x}: ", chunk_offset);

        for &byte in chunk {
            print!("{:02x} ", byte);
        }
        for _ in 0..(16 - chunk.len()) {
            print!("   ");
        }

        print!("|");
        for &byte in chunk {
            if byte.is_ascii_graphic() || byte == b' ' {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}