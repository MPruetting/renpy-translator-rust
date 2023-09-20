use std::{collections::HashMap, fs, str::FromStr};

use crate::translation::cli;
use color_eyre::eyre::Result;
use futures::future::join_all;
use itertools::Itertools;
use reqwest::Url;

struct CharsMapping<'a> {
    from: &'a str,
    to: &'a str,
}

const SPECIAL_CHARS_MAPPING: [CharsMapping; 2] = [
    CharsMapping { from: "{", to: "<" },
    CharsMapping { from: "}", to: ">" },
];
const ESCAPE_CHARS_MAPPING: [CharsMapping; 2] = [
    CharsMapping {
        from: "\"",
        to: "\\\"",
    },
    CharsMapping {
        from: "'",
        to: "\\'",
    },
];
const TRANSLATING_CHUNK_SIZE: usize = 100;
const TRANSLATION_URL_DEEPL: &str = "https://api-free.deepl.com/v2/translate";

#[derive(Clone)]
struct TranslationPart {
    file_row_number: i32,
    orig_text: String,
    translated_text: String,
}
impl TranslationPart {
    fn encode_orig_text(self: &Self) -> String {
        let mut sanatized = self.orig_text.clone();

        for map in SPECIAL_CHARS_MAPPING {
            sanatized = sanatized.replace(map.from, map.to);
        }

        log::debug!("sanatized text: {:?}", &sanatized);

        sanatized
    }

    fn decode_translated_text(self: &Self) -> String {
        let mut decoded = self.translated_text.clone();

        for map in SPECIAL_CHARS_MAPPING {
            decoded = decoded.replace(map.to, map.from);
        }

        for map in ESCAPE_CHARS_MAPPING {
            decoded = decoded.replace(map.from, map.to);
        }

        log::debug!("decoded text: {:?}", &decoded);

        decoded
    }
}
#[derive(Debug)]
struct Translation {
    translation_parts: Vec<TranslationPart>,
}
impl FromStr for Translation {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut row_number = 1;
        let texts = s
            .chars()
            .batching(|it| {
                let mut chars = String::new();
                let delim;
                let mut prev_inner_char = char::default();

                for v in it.by_ref() {
                    match v {
                        '"' | '\'' => {
                            delim = v;

                            for v in it {
                                match v {
                                    '"' | '\'' if delim == v => {
                                        if prev_inner_char == '\\' {
                                            chars.push(v);
                                            prev_inner_char = v;
                                            continue;
                                        }

                                        break;
                                    }
                                    '\n' => row_number += 1,
                                    _ => chars.push(v),
                                }
                                prev_inner_char = v;
                            }
                            break;
                        }
                        '\n' => row_number += 1,
                        _ => (),
                    }
                }
                if !chars.is_empty() {
                    Some((row_number, chars))
                } else {
                    None
                }
            })
            .filter(|(_row_number, text)| text.chars().any(|c| matches!(c, 'а'..='я')))
            .map(|(row_number, text)| TranslationPart {
                file_row_number: row_number,
                orig_text: text,
                translated_text: String::default(),
            })
            .collect_vec();

        Ok(Self {
            translation_parts: texts,
        })
    }
}

impl std::fmt::Debug for TranslationPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\nRow: {}\nOrig Text: {}\nTranslated Text: {}\n",
            self.file_row_number, self.orig_text, self.translated_text
        )
    }
}

impl Translation {
    async fn translate(&mut self, api_key: &str) {
        let mut counter = 0;

        for chunk in &self
            .translation_parts
            .clone()
            .iter()
            .chunks(TRANSLATING_CHUNK_SIZE)
        {
            let counter_to = if self.translation_parts.len() - counter <= TRANSLATING_CHUNK_SIZE {
                self.translation_parts.len()
            } else {
                counter + TRANSLATING_CHUNK_SIZE
            };
            log::info!(
                "<<<Translating >>> {} to {} of {}",
                counter,
                counter_to,
                &self.translation_parts.len()
            );

            let requests = chunk
                .map(|t| {
                    let mut params = HashMap::new();
                    let encoded_text = t.encode_orig_text();
                    log::debug!("<<<Translating {encoded_text} ...>>>");
                    params.insert("text", encoded_text.as_str());
                    params.insert("target_lang", "DE");
                    params.insert("tag_handling", "html");

                    let url = Url::parse(TRANSLATION_URL_DEEPL)
                        .unwrap()
                        .query_pairs_mut()
                        .append_pair("row_number", t.file_row_number.to_string().as_str())
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

            for response in responses {
                let res = response.unwrap();
                if res.status().is_server_error() || res.status().is_client_error() {
                    match res.status().as_u16() {
                        456 => panic!("Translation Error: Quota Exceeded"),
                        _ => {
                            panic!("Translation Error: {}", res.status())
                        }
                    }
                }

                let url = res.url().clone();
                let row_number = url
                    .query_pairs()
                    .find(|pair| pair.0 == "row_number")
                    .unwrap()
                    .1;
                let row_number: i32 = row_number.parse().unwrap();
                let body = res.text().await.unwrap();

                let parsed: json::JsonValue = json::parse(body.trim()).unwrap();
                let translated_text =
                    &parsed["translations"].members().last().unwrap()["text"].to_string();
                let part = self
                    .translation_parts
                    .iter_mut()
                    .find(|part| part.file_row_number == row_number)
                    .unwrap();
                part.translated_text = translated_text.to_string();
            }

            counter += TRANSLATING_CHUNK_SIZE;
        }
    }
}

pub async fn main() -> Result<()> {
    init();

    let translate_cli = cli::Cli::build_cli();
    let mut file_content = fs::read_to_string(translate_cli.file_to_translate)?;
    let mut translation = Translation::from_str(file_content.as_str())?;
    log::info!(
        "Anzahl Translation Strings: {}",
        &translation.translation_parts.len()
    );
    log::debug!("{:?}", &translation);

    translation.translate(&translate_cli.api_key).await;
    log::debug!("{:?}", &translation);
    log::info!("Translated Strings. Override file...");

    for part in translation.translation_parts {
        file_content = file_content.replacen(&part.orig_text, &part.decode_translated_text(), 1);
    }
    fs::write(
        "src/translation/translated_files/translated.txt",
        file_content,
    )?;

    Ok(())
}

fn init() {
    color_eyre::install().unwrap();
    env_logger::init();
}
