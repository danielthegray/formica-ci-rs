mod job_runner;

use job_runner::InitErrorKind::{
    InitScriptExecutionError, NoInitScriptFound, NoUpdateScriptInsideConfig,
    TooManyInitScriptsFound, TooManyUpdateScriptsFound, UpdateScriptExecutionError,
};
use job_runner::{ShutdownNotifiers, CONFIG_INIT_PREFIX};

use crossbeam_channel::{select, unbounded, Receiver};
use std::process::exit;

use env_logger::Env;
#[macro_use]
extern crate log;

fn initialize_jobrunner() -> ShutdownNotifiers {
    match job_runner::initialize() {
        Ok(shutdown_notifiers) => shutdown_notifiers,
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
            TooManyUpdateScriptsFound(duplicate_scripts) => {
                let duplicate_string_list: String = duplicate_scripts.join("\n *");
                eprintln!(
                    "More than one '{}' script was found in the configuration directory: \n * {}",
                    CONFIG_INIT_PREFIX, duplicate_string_list
                );
                exit(exitcode::DATAERR);
            }
            UpdateScriptExecutionError(bad_execution) => {
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
}

fn explain_exit_logic() {
    info!("Successive Ctrl + C presses will exit, in the following way:");
    info!("Press Ctrl + C again to start a slow shutdown: no new jobs will be accepted, but the existing ones will run their course and only then will Formica shutdown.");
    info!("Press Ctrl + C again to start an immediate shutdown: all jobs will be terminated and the agent machines cleaned up.");
    info!("Press Ctrl + C again to force termination of all agent tracker processes: all the trackers will be terminated without cleaning up the agent machines (at your own risk!)");
    info!("Press Ctrl + C one more time to exit immediately (very much at your own risk, zombie processes may be left running on the machine).");
}

fn build_ctrl_c_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = unbounded();
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;
    Ok(receiver)
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let ctrl_c_receiver = match build_ctrl_c_channel() {
        Ok(ctrl_c_channel) => ctrl_c_channel,
        Err(ctrl_c_setup_err) => panic!(
            "There was an error when setting up the Ctrl+C listener! {:?}",
            ctrl_c_setup_err
        ),
    };
    println!("Formica CI is now running");
    explain_exit_logic();

    let shutdown_notifiers = initialize_jobrunner();

    let mut number_of_control_c_presses = 0;
    loop {
        select! {
            recv(ctrl_c_receiver) -> _ => {
                number_of_control_c_presses+=1;
                if number_of_control_c_presses == 1 {
                    info!("Starting slow shutdown: No more jobs will be accepted...");
                    shutdown_notifiers.slow_shutdown.send(());
                } else if number_of_control_c_presses == 2 {
                    info!("Triggering immediate shutdown: Cleaning up agents...");
                    shutdown_notifiers.immediate_shutdown.send(());
                } else if number_of_control_c_presses == 3 {
                    info!("Forcing termination of all worker tracker processes. Pressing Ctrl+C again may leave zombie processes!");
                    shutdown_notifiers.force_termination.send(());
                } else {
                    warn!("Terminating immediately! (zombie processes may be left, please restart this machine to clear them up)");
                    exit(exitcode::TEMPFAIL);
                }
            }
        }
    }
}
