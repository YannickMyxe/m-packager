mod archive;
mod cli;

fn main() -> anyhow::Result<()> {
    cli::run()
}
