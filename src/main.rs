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
struct Cli {
    /// Describe the workspace background you want to generate.
    #[arg(long, value_name = "TEXT")]
    prompt: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let prompt = cli
        .prompt
        .as_deref()
        .unwrap_or("Imagine a workspace with immersive backgrounds! (Use --help for options.)");

    println!("{prompt}");
}
