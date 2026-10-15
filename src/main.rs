mod archive;
mod cli;
mod index;

fn main() -> anyhow::Result<()> {
    cli::run()
}
