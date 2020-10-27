mod job_runner;

use job_runner::JobRunnerErrorKind::{
    InitScriptExecutionError, NoInitScriptFound, TooManyInitScriptsFound,
};
use job_runner::CONFIG_INIT_PREFIX;
use std::process::exit;

#[macro_use]
extern crate log;

fn main() {
    env_logger::init();
    match job_runner::initialize() {
        Ok(_) => (),
        Err(init_error) => match init_error.kind {
            NoInitScriptFound => {
                eprintln!(
                    "No job initialization script (starting with '{}') was found!",
                    CONFIG_INIT_PREFIX
                );
                eprintln!("If this is your first time, I recommend you run one of the starter tools, e.g.: setup_git");
                eprintln!("This will setup a scaffold/skeleton jobs configuration ready to populate with new jobs!");
                exit(exitcode::DATAERR);
            }
            TooManyInitScriptsFound(duplicate_scripts) => {
                let duplicate_string_list: String = duplicate_scripts.join("\n *");
                eprintln!(
                    "More than one '{}' script was found in the current directory: \n * {}",
                    CONFIG_INIT_PREFIX, duplicate_string_list
                );
                exit(exitcode::DATAERR);
            }
            InitScriptExecutionError(bad_execution) => {
                println!(
                    "The execution terminated with status {}",
                    bad_execution.status
                );
                println!(
                    "The execution terminated with output:\n {}",
                    String::from_utf8(bad_execution.stdout).unwrap()
                );
                println!(
                    "The execution terminated with error output:\n {}",
                    String::from_utf8(bad_execution.stderr).unwrap()
                );
                exit(bad_execution.status.code().unwrap_or(exitcode::SOFTWARE));
            }
            NoUpdateScriptInsideConfig => {
                eprintln!(
                    "No job update script (starting with '{}') was found in the configuration directory!",
                    job_runner::UPDATE
                );
                eprintln!("If this is your first time, I recommend you run one of the starter tools, e.g.: setup_git");
                eprintln!("This will setup a scaffold/skeleton jobs configuration ready to populate with new jobs!");
                exit(exitcode::DATAERR);
            }
            TooManyInitScriptsFound(duplicate_scripts) => {
                let duplicate_string_list: String = duplicate_scripts.join("\n *");
                eprintln!(
                    "More than one '{}' script was found in the configuration directory: \n * {}",
                    CONFIG_INIT_PREFIX, duplicate_string_list
                );
                exit(exitcode::DATAERR);
            }
            InitScriptExecutionError(bad_execution) => {
                println!(
                    "The execution of the update script terminated with status {}",
                    bad_execution.status
                );
                println!(
                    "The execution terminated with output:\n {}",
                    String::from_utf8(bad_execution.stdout).unwrap()
                );
                println!(
                    "The execution terminated with error output:\n {}",
                    String::from_utf8(bad_execution.stderr).unwrap()
                );
                exit(bad_execution.status.code().unwrap_or(exitcode::SOFTWARE));
            }
        },
    }
    println!("The jobs folder has been initialized correctly!");
}
