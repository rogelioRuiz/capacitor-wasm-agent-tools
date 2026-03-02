use std::io::Read;

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "echo".to_string());

    match mode.as_str() {
        "loop" => loop {
            std::hint::spin_loop();
        },
        "exit7" => {
            eprintln!("exit=7");
            std::process::exit(7);
        }
        _ => {
            let mut stdin = String::new();
            std::io::stdin()
                .read_to_string(&mut stdin)
                .expect("stdin should be readable");
            println!("mode={mode}");
            println!("stdin={stdin}");
            eprintln!("stderr={mode}");
        }
    }
}
