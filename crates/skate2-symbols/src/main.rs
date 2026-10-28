use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    process::ExitCode,
};

use skate2_symbols::parse_public_symbol;

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
    let path = env::args_os()
        .nth(1)
        .or_else(|| env::var_os("SKATE2_MAP_FILE"))
        .ok_or("pass a map-file path or set SKATE2_MAP_FILE")?;
    let reader = BufReader::new(File::open(&path)?);
    let mut symbols = 0_u64;
    let mut functions = 0_u64;
    let mut parse_errors = 0_u64;

    for line in reader.lines() {
        match parse_public_symbol(&line?) {
            Ok(Some(symbol)) => {
                symbols += 1;
                if symbol
                    .attribution
                    .split_whitespace()
                    .any(|flag| flag == "f")
                {
                    functions += 1;
                }
            }
            Ok(None) => {}
            Err(_) => parse_errors += 1,
        }
    }

    let display_path = path.to_string_lossy();
    println!("map: {display_path}");
    println!("public symbols: {symbols}");
    println!("function records: {functions}");
    println!("rejected candidate lines: {parse_errors}");
    if parse_errors != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "map parse errors").into());
    }
    Ok(())
}
