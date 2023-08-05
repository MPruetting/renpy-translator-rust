use text_translator::translation::translator;
use color_eyre::eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();
    translator::translate().await.unwrap();
    Ok(())
}
