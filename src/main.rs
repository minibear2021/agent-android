mod flags;
mod adb;
mod commands;
mod output;
mod snapshot;
mod state;
mod query;

use std::env;
use std::process::exit;
use flags::{parse_flags, clean_args};
use commands::execute_command;
use output::{print_response, print_help, Response};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let flags = parse_flags(&args);
    let clean = clean_args(&args);

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }
    
    if clean.is_empty() {
        print_help();
        return;
    }

    match execute_command(&clean, &flags) {
        Ok(data) => {
            print_response(&Response::ok(data), flags.json);
        }
        Err(e) => {
            print_response(&Response::err(e), flags.json);
            exit(1);
        }
    }
}
