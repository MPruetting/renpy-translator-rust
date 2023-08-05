use clap::{arg, Command};
use std::env;

const API_KEY_NAME: &str = "GOOGLE_TRANSLATE_API_KEY";

pub struct Cli {
    pub file_to_translate: String,
    pub api_key: String,
}

impl Cli {
    /// Build the Command line interface
    pub fn build_cli() -> Cli {
        let matches = Command::new("TextTranslator")
            .version("1.0")
            .author("Marcus Prütting")
            .about("Translates Renpy Text")
            .arg(arg!(--f <translation_file>).required(true))
            .arg(arg!(--k <api_key>).required(false))
            .get_matches();

        let file_to_translate: String = matches
            .get_one::<String>("f")
            .expect("required")
            .to_string();
        let api_key: String = match matches.get_one::<String>("k") {
            Some(key) => key.to_string(),
            None => Cli::get_api_key(),
        };

        Cli {
            file_to_translate,
            api_key,
        }
    }

    fn get_api_key() -> String {
        let key = match env::var(API_KEY_NAME){
            Ok(k) => k,
            Err(_) => panic!("Api Key is not provided set '{API_KEY_NAME}' as evironment variable or use '--k' in cli interface")
        };

        key
    }
}
