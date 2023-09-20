use color_eyre::eyre::Result;
use text_translator::translation::translator;
use text_translator::neu;

#[tokio::main]
async fn main() -> Result<()> {
    //color_eyre::install()?;
    //env_logger::init();
    //translator::translate().await.unwrap();

    neu::main().await?;
    Ok(())
}
