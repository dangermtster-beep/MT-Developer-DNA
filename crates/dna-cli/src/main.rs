use clap::Parser;

fn main() {
    match dna_cli::run(dna_cli::Cli::parse()) {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("Hata: {error}");
            std::process::exit(2);
        }
    }
}
