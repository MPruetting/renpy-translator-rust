use std::fs::{File, OpenOptions};
use std::io::{Write, BufReader, BufRead, self};
use std::path::{Path, PathBuf};

pub fn build_src_path() -> PathBuf{
    Path::new(".").join("src")
}

pub fn string_to_path(string: &str) -> &Path{
    Path::new(string)
}

pub fn create_file(path: &Path){
    match File::create(path){
        Ok(_file) => println!("Erstellte Datei {:?}", path),
        Err(err) => panic!("Konnte Datei nicht erstellen | {err}")
    };
}

pub fn write_from_to_file_per_line(from_file: File, to_file: &mut File){
    let mut orig_file_buf = BufReader::new(from_file);
    let mut orig_buf_content = String::new();
    let mut read_bytes: usize;

    loop {
        read_bytes = orig_file_buf.read_line(&mut orig_buf_content).unwrap();

        if read_bytes == 0{ // end of file reached
            break;
        }

        write_file(to_file, orig_buf_content.as_str());
        orig_buf_content.clear();
    }
}

pub fn read_file(file: File){
    let mut file_buf = BufReader::new(file);
    let mut buf_content = String::new();
    let mut read_bytes: usize;

    loop {
        read_bytes = file_buf.read_line(&mut buf_content).unwrap();

        if read_bytes == 0{ // end of file reached
            break;
        }
        println!("{}",buf_content.trim());
        buf_content.clear();
    }
}

pub fn write_file(file: &mut File, text: &str){
    match file.write(text.as_bytes()){
        Ok(_) => (),
        Err(err) => panic!("Fehler beim Datei schreiben {err}")
    }
}

pub fn open_file(path: &Path, open_options: OpenOptions) -> File{
    match open_options.open(path){
        Ok(file) => return file,
        Err(err) => panic!("Pfad: {:?} - Konnte Datei nicht öffnen | {err}", path)
    };
}

pub fn write_options() -> OpenOptions{
    let mut options = OpenOptions::new();

    options.write(true);
    options
} 

pub fn append_options() -> OpenOptions{
    let mut options = OpenOptions::new();

    options.append(true);
    options
} 

pub fn read_options() -> OpenOptions{
    let mut options = OpenOptions::new();

    options.read(true);
    options
} 

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let mut results = Vec::new();

    for line in contents.lines() {
        if line.contains(query) {
            results.push(line);
        }
    }

    results
}

pub fn user_input_file() -> String{
    let mut translation_file = String::new();

    io::stdin().read_line(&mut translation_file).unwrap();
    let translation_file_content = translation_file.trim().to_string();
    translation_file_content
}
