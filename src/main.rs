use clap::Parser;
use gh_monthly::cli::Cli;

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();
}
