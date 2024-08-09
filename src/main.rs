use std::env;
use std::ffi::OsString;
use std::fs::remove_dir;
use std::path::{Path, PathBuf};

use chrono::{Datelike, DateTime, Utc};
use exif::{Error, In, Tag};
use glob::glob;

fn main() {
    let args: Vec<String> = env::args().collect();
    const DEFAULT_DIRECTORY: &str = ".";
    let source_path = args.get(1).map(|string| string.as_str()).unwrap_or(DEFAULT_DIRECTORY);

    let mut glob_pattern = String::new();
    glob_pattern.push_str(source_path);
    glob_pattern.push_str("\\**\\*");

    for path_buf in glob(&glob_pattern).expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .filter(|path_buf| path_buf.is_file())
        .filter(|path_buf| path_buf.file_name().map(|filename| !filename.eq_ignore_ascii_case("pixort.exe")).unwrap_or(false)) {
        let extension = path_buf.extension().and_then(|f| f.to_str()).unwrap_or("");
        let extension = match extension {
            "" => String::from(""),
            "jpeg" => String::from(".jpg"),
            x => format!(".{x}"),
        };

        let file = std::fs::File::open(path_buf.as_path()).unwrap();
        let mut buf_reader = std::io::BufReader::new(&file);
        let exif_reader = exif::Reader::new();

        let new_file_path = {
            match exif_reader.read_from_container(&mut buf_reader).as_ref().and_then(|exif| exif.get_field(Tag::DateTimeOriginal, In::PRIMARY).ok_or(&Error::UnexpectedValue(""))) {
                Ok(date_time_original) => {
                    let date_time_original_value = format!("{}", date_time_original.display_value());

                    let year = date_time_original_value[0..4].parse::<i32>().unwrap_or(0);
                    let month = date_time_original_value[5..7].parse::<u32>().unwrap_or(0);

                    Ok((year, month, date_time_original_value.replace(":", "-")))
                }
                Err(error) => {
                    match std::fs::metadata(path_buf.as_path()).and_then(|metadata| metadata.modified()) {
                        Ok(modified_time) => {
                            let modified_time: DateTime<Utc> = modified_time.into();
                            let modified_time_value = format!("{}", modified_time.format("%Y-%m-%d %H-%M-%S"));

                            let year = modified_time.year(); //String::from(&modified_time_value[0..4]);
                            let month = modified_time.month(); //String::from(&modified_time_value[5..7]);

                            Ok((year, month, modified_time_value))
                        }
                        Err(error2) => {
                            eprintln!("unable to sort \"{}\" error={} error2={}!", path_buf.as_path().display(), error, error2);
                            Err(error2)
                        }
                    }
                }
            }
        };

        if let Ok((year, month, date_time)) = new_file_path {
            if let Some(new_file_path) = find_unused_target_file_path(&path_buf.as_path(), source_path, year, month, date_time.as_str(), extension.as_str()) {
                let new_file_path = Path::new(&new_file_path);

                sort_pic(path_buf.as_path(), new_file_path, false);
            }
        }
    }

    // clear all empty folders after
    for path_buf in glob(&glob_pattern).expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .filter(|path_buf| path_buf.is_dir()) {
        if path_buf.read_dir().map(|entries| entries.count() == 0).unwrap_or(false) {
            match remove_dir(&path_buf) {
                Ok(_) => println!("removed empty dir {}", path_buf.display()),
                Err(_) => eprintln!("could not remove empty dir {}", path_buf.display())
            }
        }
    }
}

fn month_to_string(month: u32) -> String {
    match month {
        1 => String::from("Jänner"),
        2 => String::from("Februar"),
        3 => String::from("März"),
        4 => String::from("April"),
        5 => String::from("Mai"),
        6 => String::from("Juni"),
        7 => String::from("Juli"),
        8 => String::from("August"),
        9 => String::from("September"),
        10 => String::from("Oktober"),
        11 => String::from("November"),
        12 => String::from("Dezember"),
        _ => format!("{month}"),
    }
}

fn find_unused_target_file_path<'a>(source_path: &Path, target_path: &'a str, year: i32, month: u32, date_time: &'a str, extension: &'a str) -> Option<OsString> {
    let mut number = 0;
    let index = (year - 2016) as u32 * 12 + month - 2;
    let month_string = month_to_string(month);
    let new_file_path_base = PathBuf::from(format!("{target_path}\\Jannik {year}\\Jannik {index:03} {month_string} {year}\\"));
    let new_file_path_base =     new_file_path_base.strip_prefix(".").map(|path| path.to_path_buf()).unwrap_or(new_file_path_base);

    let mut result: OsString = OsString::new();

    loop {
        let mut new_file_path = new_file_path_base.clone();
        let file = format!("{date_time}-{number:03}{extension}");
        new_file_path.push(&file[..]);
        let _new_file_path = Path::new(&new_file_path);
        if source_path.eq(_new_file_path) {
            return None;
        } else if !_new_file_path.exists() {
            _new_file_path.as_os_str().clone_into(&mut result);
            return Some(result);
        }
        number += 1;
    }
}

fn sort_pic(original_path: &Path, sorted_path: &Path, keep_original_files: bool) {
    match sorted_path.parent() {
        Some(parent) => {
            if let Err(error) = std::fs::create_dir_all(parent) {
                eprintln!("couldn't create parent directory for  \"{}\" due to {error}", sorted_path.display());
            }
        }
        None => {
            eprintln!("couldn't get parent directory for  \"{}\" as there seems to be none", sorted_path.display());
        }
    }

    let result = if keep_original_files {
        std::fs::copy(original_path, sorted_path).map(|_| ())
    } else {
        std::fs::rename(original_path, sorted_path)
    };

    match result {
        Ok(_) =>
            println!("\"{}\" to \"{}\"", original_path.display(), sorted_path.display()),
        Err(error) =>
            eprintln!("couldn't sort \"{}\" due to {error}", original_path.display()),
    }
}
