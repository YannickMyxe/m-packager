mod archive;
mod cli;
mod index;
mod paths;

fn main() -> anyhow::Result<()> {
    cli::run()
}
