use clap::Parser;

/// Mawaku CLI entry point.
///
/// Mawaku will translate natural language scene descriptions into
/// prompts for background generators such as Google Imagen or OpenAI's DALL-E.
#[derive(Parser, Debug)]
#[command(
    name = "mawaku",
    author,
    version,
    about = "Generate video-call backgrounds by describing a place.",
    long_about = None
)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    println!("Imagine a workspace with immersive backgrounds! (Use --help for options.)");
}
