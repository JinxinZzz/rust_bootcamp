use clap::Parser;
use std::collections::HashMap;
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(author, version, about = "Count word frequency in text", long_about = None)]
struct Args {
    #[arg(default_value_t = String::new())]
    text: String,

    #[arg(long, default_value_t = 10)]
    top: usize,

    #[arg(long, default_value_t = 1)]
    min_length: usize,

    #[arg(long)]
    ignore_case: bool,
}

fn main() {
    let args = Args::parse();

    let text = if args.text.is_empty() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).expect("Failed to read from stdin");
        buffer
    } else {
        args.text
    };

    let word_counts = count_word_frequency(&text, args.min_length, args.ignore_case);

    let mut sorted_words: Vec<(&String, &u32)> = word_counts.iter().collect();
    sorted_words.sort_by(|a, b| b.1.cmp(a.1));

    println!("Word frequency:");
    for &(word, &count) in sorted_words.iter().take(args.top) {
        println!("{}: {}", word, count);
    }
}

fn count_word_frequency(text: &str, min_length: usize, ignore_case: bool) -> HashMap<String, u32> {
    let mut counts = HashMap::new();

    for raw_word in text.split_whitespace() {
        if raw_word.len() < min_length {
            continue;
        }

        let word = if ignore_case {
            raw_word.to_lowercase()
        } else {
            raw_word.to_string()
        };

        *counts.entry(word).or_insert(0) += 1;
    }

    counts
}