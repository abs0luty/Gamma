use std::process::exit;

use gamma_parser::parser::Parser;

fn main() {
    let rl = rustyline::Editor::<()>::new();
    match rl {
        Ok(mut e) => loop {
            match e.readline(">>") {
                Ok(line) => {
                    let mut parser = Parser::new(line.as_str(), "<stdin>");
                    let ast = parser.parse();
                    println!("{:?}", ast);
                }
                Err(_) => {
                    println!("bye");
                    exit(0);
                }
            }
        },
        Err(err) => {
            panic!("{}", err)
        }
    }
}
