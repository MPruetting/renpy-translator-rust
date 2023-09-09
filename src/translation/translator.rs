use std::{collections::HashMap, fs};

use super::cli;
use color_eyre::eyre::Result;
use itertools::Itertools;
use reqwest::Url;

use futures::future::join_all;

const SPECIAL_CHARS_MAPPING: [SpecialCharsMapping; 2] = [
    SpecialCharsMapping {
        from: "{",
        to: "<",
    },
    SpecialCharsMapping {
        from: "}",
        to: ">",
    }
];
const TRANSLATION_URL_GOOGLE: &str = "https://translation.googleapis.com/language/translate/v2";
const TRANSLATION_URL_DEEPL: &str = "https://api-free.deepl.com/v2/translate";

#[derive(Debug)]
struct TranslationPart {
    orig_text: String,
    translated_text: String,
}

struct SpecialCharsMapping<'a> {
    from: &'a str,
    to: &'a str,
}

impl TranslationPart {
    fn to_orig_text_encoded(self: &Self) -> String {
        let mut sanatized = self.orig_text.clone();

        for map in SPECIAL_CHARS_MAPPING {
            sanatized = sanatized.replace(map.from, map.to);
        }

        log::debug!("sanatized text: {:?}", &sanatized);

        sanatized
    }

    fn to_translated_text_decoded(self: &Self) -> String {
        let mut decoded = self.translated_text.clone();

        for map in SPECIAL_CHARS_MAPPING {
            decoded = decoded.replace(map.to, map.from);
        }

        log::debug!("decoded text: {:?}", &decoded);

        decoded
    }
}

pub async fn translate() -> Result<()> {
    let translate_cli = cli::Cli::build_cli();
    let mut file_content = fs::read_to_string(translate_cli.file_to_translate)?;
    let texts = get_translation_strings(&file_content)?;

    if texts.is_empty() {
        println!("Keine Translation Strings In Datei gefunden, welche übersetzt werden können.");
        return Ok(());
    }
    
    let translation_parts = translate_texts_deepl(translate_cli.api_key.as_str(), texts).await?;
    for part in translation_parts {
        file_content = file_content.replace(&part.orig_text, &part.to_translated_text_decoded());
    }

    fs::write(
        "src/translation/translated_files/translated.txt",
        file_content,
    )?;
    Ok(())
}

async fn translate_texts(
    api_key: &str,
    texts: Vec<TranslationPart>,
) -> Result<Vec<TranslationPart>> {
    log::debug!("<<<Translating Found Text - Starting URL Requests GOOGLE>>>");
    let basic_url = format!(
        "{}?key={}&source=ru&target=de",
        TRANSLATION_URL_GOOGLE, api_key
    );
    let requests = texts
        .iter()
        .map(|t| {
            reqwest::Client::new()
                .get(
                    Url::parse(basic_url.as_str())
                        .unwrap()
                        .query_pairs_mut()
                        .append_pair("q", t.to_orig_text_encoded().as_str())
                        .append_pair("orig_text", t.orig_text.as_str())
                        .finish()
                        .as_str(),
                )
                .send()
        })
        .collect_vec();

    let responses = join_all(requests).await;

    let mut translation_parts: Vec<TranslationPart> = vec![];
    for response in responses {
        let res = response?;
        let url = res.url().clone();
        let orig_text = url
            .query_pairs()
            .find(|pair| pair.0 == "orig_text")
            .unwrap()
            .1;

        let body = res.text().await?;
        let parsed: json::JsonValue = json::parse(body.trim()).unwrap();
        let translated_text = &parsed["data"]["translations"];

        translation_parts.push(TranslationPart {
            orig_text: orig_text.clone().to_string(),
            translated_text: translated_text.members().last().unwrap()["translatedText"]
                .as_str()
                .unwrap()
                .into(),
        });
    }
    log::debug!("translated: {:?}", &translation_parts);
    log::info!("Anzahl Translated Strings: {:?}", &translation_parts.len());

    Ok(translation_parts)
}

async fn translate_texts_deepl(
    api_key: &str,
    texts: Vec<TranslationPart>,
) -> Result<Vec<TranslationPart>> {
    log::debug!("<<<Translating Found Text - Starting URL Requests DEEPL>>>");

    let requests = texts
        .iter()
        .map(|t| {
            let mut params = HashMap::new();
            let encoded_text = t.to_orig_text_encoded();
            log::debug!("<<<Translating {encoded_text} ...>>>");
            params.insert("text", encoded_text.as_str());
            params.insert("target_lang", "DE");
            params.insert("tag_handling", "html");

            let url = Url::parse(TRANSLATION_URL_DEEPL)
                .unwrap()
                .query_pairs_mut()
                .append_pair("orig_text", t.orig_text.as_str())
                .finish()
                .to_string();

            reqwest::Client::new()
                .post(url)
                .form(&params)
                .header("Authorization", String::from("DeepL-Auth-Key ") + api_key)
                .send()
        })
        .collect_vec();

    let responses = join_all(requests).await;

    let mut translation_parts: Vec<TranslationPart> = vec![];
    for response in responses {
        let res = response?;
        let url = res.url().clone();
        let orig_text = url
            .query_pairs()
            .find(|pair| pair.0 == "orig_text")
            .unwrap()
            .1;

        let body = res.text().await?;
        
        let parsed: json::JsonValue = json::parse(body.trim()).unwrap();
        let translated_text = &parsed["translations"];

        translation_parts.push(TranslationPart {
            orig_text: orig_text.clone().to_string(),
            translated_text: translated_text.members().last().unwrap()["text"]
                .as_str()
                .unwrap()
                .into(),
        });
    }
    log::debug!("translated: {:?}", &translation_parts);
    log::info!("Anzahl Translated Strings: {:?}", &translation_parts.len());

    Ok(translation_parts)
}

fn get_translation_strings(file_content: &str) -> Result<Vec<TranslationPart>> {
    log::debug!("<<<start get_translation_strings Function>>>");
    let texts = file_content
        .chars()
        .batching(|it| {
            let mut chars = String::new();
            let delim;
            let mut prev_inner_char = char::default();

            for v in it.by_ref() {
                log::debug!("outer ({v}) | ");
                match v {
                    '"' | '\'' => {
                        delim = v;

                        inner_while(&mut prev_inner_char, &mut chars, delim, it);
                        break;
                    }
                    _ => (),
                }
            }
            if !chars.is_empty() {
                Some(chars)
            } else {
                None
            }
        })
        .filter(|t| !t.is_ascii())
        .map(|text| TranslationPart {
            orig_text: text,
            translated_text: String::default(),
        })
        .collect_vec();
    log::debug!("Final Translation Strings: {:?}", &texts);
    log::info!("Anzahl Translation Strings: {:?}", &texts.len());
    Ok(texts)
}

fn inner_while(
    prev_inner_char: &mut char,
    chars: &mut String,
    delim: char,
    it: &mut std::str::Chars<'_>,
) {
    for v in it {
        log::debug!("inner prev:({prev_inner_char}:{v}) | ");
        match v {
            '"' | '\'' if delim == v => {
                if *prev_inner_char == '\\' {
                    log::debug!("<<found prev backslash skip>>");
                    chars.push(v);
                    *prev_inner_char = v;
                    continue;
                }

                break;
            }
            _ => chars.push(v),
        }
        *prev_inner_char = v;
    }
}
