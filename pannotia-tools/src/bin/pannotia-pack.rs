use std::process::ExitCode;

use clap::Parser;

use pannotia::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    family: String,
    textfile: clio::Input,
    output: clio::Output,
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    let family = if let Ok(family) = Family::try_from(cli.family.as_str()) {
        family
    } else {
        eprintln!("invalid family");
        return ExitCode::FAILURE;
    };

    let mut b = Bitstream::new(family);

    let e = b.save(cli.output);
    match e {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
