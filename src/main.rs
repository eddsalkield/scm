extern crate clap;
extern crate sys_info;
extern crate toml;
extern crate regex;
extern crate serde;

use args::Command;
use runner::Runner;

mod app;
mod args;
mod runner;
mod hooks;
mod file_ops;
mod settings;

// exit code structure idea from https://stackoverflow.com/a/30285110
fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let app = app::new();
    let args = match args::get_args(app.get_matches()) {
        Ok(args) => args,
        Err(msg) => {
            println!("Argument error: {}", msg);
            return 1;
        }
    };

    if args.test {
        println!(":: Test mode active. Hooks will not execute and files will not be modified.");
    }

    if args.force {
        println!(":: Force mode active. Files will be overwritten/removed without question.");
    }


    let runner = Runner::new(&args);

    let success = match args.command {
        Command::Install => runner.install(),
        Command::Uninstall => runner.uninstall(),
        Command::Add => runner.add(),
        Command::Empty => {
            println!("ERR: No subcommand given!");
            false
        }
    };

    if success {
        println!(":: Complete with success!");
        return 0;
    } else {
        println!(":: Exited on error.");
        return 1;
    }

}
