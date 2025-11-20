use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "A simple greeting program")]
struct Args {
    /// Name to greet [default: World]
    name: Option<String>,

    /// Convert to uppercase
    #[arg(long)]
    upper: bool,

    /// Repeat greeting N times [default: 1]
    #[arg(long, default_value_t = 1)]
    repeat: u8,
}

fn main() {
    let args = Args::parse();
    let name = args.name.unwrap_or_else(|| "World".to_string());
    let greeting = if args.upper {
        format!("HELLO, {}!", name.to_uppercase())
    } else {
        format!("Hello, {}!", name)
    };

    for _ in 0..args.repeat {
        println!("{}", greeting);
    }
}