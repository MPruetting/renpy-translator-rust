use color_eyre::eyre::Result;
use text_translator::translation::translator;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();
    // text_deepl().await.unwrap();
    translator::translate().await.unwrap();
    Ok(())
}
