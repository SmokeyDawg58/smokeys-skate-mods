use std::{env, path::PathBuf, process::ExitCode};

use skate2_formats::{big::BigEntryKey, mount::MountedBig};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let supplied = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("pass a .big or .bh path")?;
    let use_external_header = supplied
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bh"));
    let mut data_path = supplied.clone();
    if use_external_header {
        data_path.set_extension("big");
    }
    let mounted = MountedBig::open(&data_path, use_external_header)?;
    let archive = mounted.archive();

    println!("data file: {}", mounted.data_path().display());
    println!("table file: {}", mounted.header_path().display());
    println!("format: {:?}", archive.format);
    println!("header bytes: {}", archive.header_size);
    println!("entries: {}", archive.entries.len());
    for entry in archive.entries.iter().take(10) {
        let key = match &entry.key {
            BigEntryKey::Name(name) => name.clone(),
            BigEntryKey::Hash(hash) => format!("hash:{hash:08x}"),
        };
        println!(
            "{key}: offset=0x{:x} stored={} unpacked={}",
            entry.offset, entry.stored_size, entry.unpacked_size
        );
    }

    if let Some(name) = env::args().nth(2) {
        let extracted = mounted.read_path(&name)?;
        let range_offset = env::args()
            .nth(3)
            .map(|value| parse_offset(&value))
            .transpose()?
            .unwrap_or(0);
        let range_length = env::args()
            .nth(4)
            .map(|value| parse_offset(&value))
            .transpose()?
            .unwrap_or(64);
        let range_end = range_offset
            .checked_add(range_length)
            .ok_or("selected range overflow")?
            .min(extracted.len());
        let selected = extracted
            .get(range_offset..range_end)
            .ok_or("selected range starts outside the entry")?;
        println!("selected: {name}");
        println!("extracted bytes: {}", extracted.len());
        println!("selected range: {range_offset}..{range_end}");
        println!("range bytes: {selected:02x?}");
        println!("range text: {:?}", String::from_utf8_lossy(selected));
    }
    Ok(())
}

fn parse_offset(value: &str) -> Result<usize, Box<dyn std::error::Error>> {
    if let Some(hex) = value.strip_prefix("0x") {
        Ok(usize::from_str_radix(hex, 16)?)
    } else {
        Ok(value.parse()?)
    }
}
