use color_eyre::eyre::Result;
use reqwest::Url;
use text_translator::translation::translator;
use text_translator::parser;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();
    // text_deepl().await.unwrap();
    // translator::translate().await.unwrap();
    test_parser().unwrap();
    Ok(())
}

fn test_parser() -> Result<()>{
    let ret = parser::parse_renpy_tag("{w}").unwrap();
    dbg!(ret);
    Ok(())
}

async fn text_deepl() -> Result<()> {
    let client = reqwest::Client::new();
    let url = Url::parse("https://api-free.deepl.com/v2/translate")
        .unwrap()
        .query_pairs_mut()
        .append_pair("orig", "hello")
        .finish()
        .to_string();
    let params = [("text", "hello"), ("target_lang", "DE")];
    let res = client
        .post(url)
        .form(&params)
        .header(
            "Authorization",
            "DeepL-Auth-Key 1e5512af-570f-97e5-5d99-863f7af21e2c:fx",
        )
        .send()
        .await?;

    let url = res.url().clone();
    let orig_text = url
        .query_pairs()
        .find(|pair| pair.0 == "orig")
        .unwrap()
        .1;
    let body = res.text().await?;
    println!("body {}", body);
    println!("orig_text {}", orig_text);

    Ok(())
}
