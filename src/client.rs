use std::collections::HashMap;

use color_eyre::eyre::Result;
use itertools::Itertools;
use reqwest::Url;

const SPECIAL_CHARS_MAPPING: [SpecialCharsMapping; 2] = [
    SpecialCharsMapping { from: "{", to: "<" },
    SpecialCharsMapping { from: "}", to: ">" },
];
const ESCAPE_CHARS_MAPPING: [EscapeCharsMapping; 2] = [
    EscapeCharsMapping {
        from: "\"",
        to: "\\\"",
    },
    EscapeCharsMapping {
        from: "'",
        to: "\\'",
    },
];
const TRANSLATING_CHUNK_SIZE: usize = 100;

const TRANSLATION_URL_GOOGLE: &str = "https://translation.googleapis.com/language/translate/v2";
const TRANSLATION_URL_DEEPL: &str = "https://api-free.deepl.com/v2/translate";

struct SpecialCharsMapping<'a> {
    from: &'a str,
    to: &'a str,
}

struct EscapeCharsMapping<'a> {
    from: &'a str,
    to: &'a str,
}

#[derive(Debug)]
pub struct TranslationPart {
    file_row_number: String,
    orig_text: String,
    translated_text: String,
}

pub trait ClientTranslator {
    fn make_request(&self) -> Result<Vec<TranslationPart>>;
}

pub struct ClientTranslatorConfig{
    api_key: String
}

pub struct DeeplClient {
    config: ClientTranslatorConfig,
    text: Vec<TranslationPart>
}

impl DeeplClient{
    fn encode_text(self: &Self) -> String {
        let mut sanatized = self.orig_text.clone();

        for map in SPECIAL_CHARS_MAPPING {
            sanatized = sanatized.replace(map.from, map.to);
        }

        log::debug!("sanatized text: {:?}", &sanatized);

        sanatized
    }

    fn decode_text(self: &Self) -> String {
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

impl ClientTranslator for DeeplClient {
    fn make_request(&self) -> Result<Vec<TranslationPart>> {
        log::debug!("<<<Translating Found Text - Starting URL Requests DEEPL>>>");

        log::debug!("<<<---Encoding Text--->>>");
        let encoded_text = &self.encode_text();
        log::debug!("<<<---Encoding Finished | Start Translating--->>>");

        let mut translation_parts: Vec<TranslationPart> = vec![];
        let mut counter = 0;
        for chunk in &encoded_text.iter().chunks(TRANSLATING_CHUNK_SIZE) {
            let counter_to = if &self.text.len() - counter <= TRANSLATING_CHUNK_SIZE {
                self.text.len()
            } else {
                counter + TRANSLATING_CHUNK_SIZE
            };
            log::info!(
                "<<<Translating >>> {} to {} of {}",
                counter,
                counter_to,
                &self.text.len()
            );

            let requests = chunk
                .map(|t| {
                    let mut params = HashMap::new();
                    let encoded_text = t;
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
                        .header("Authorization", String::from("DeepL-Auth-Key ") + self.config.api_key.as_str())
                        .send()
                })
                .collect_vec();

            let responses = join_all(requests).await;

            for response in responses {
                let res = response?;
                let url = res.url().clone();
                let orig_text = url
                    .query_pairs()
                    .find(|pair| pair.0 == "orig_text")
                    .unwrap()
                    .1;

                let body = res.text().await?;

                log::debug!("<<<Response Body {body}>>>");

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
            counter += TRANSLATING_CHUNK_SIZE;
        }

        log::debug!("translated: {:?}", &translation_parts);
        log::info!("Anzahl Translated Strings: {:?}", &translation_parts.len());

        Ok(translation_parts)
    }
}
