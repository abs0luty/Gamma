use std::{env, fs, process::exit};

mod eval;

use codemap::CodeMap;
use eval::Evaluator;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: gamma <filename>");
        exit(1);
    }

    let filename = args[1].as_str();
    match fs::read_to_string(filename) {
        Ok(content) => {
            let mut codemap = CodeMap::new();
            let mut exec = Evaluator::new(content.as_str(), filename, &mut codemap);
            exec.eval();
            //
            // }
        }
        Err(_) => {
            eprintln!("unable to read file");
            exit(1);
        }
    }
}
