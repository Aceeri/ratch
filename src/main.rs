extern crate clap;

mod error;

use std::io::{Write, Read, stdout};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use self::error::RatchError;
use clap::{App, AppSettings, Arg, SubCommand};
//use std::error::Error;

fn parse_interval(interval: Option<&str>) -> Result<f64, RatchError> {
    match interval {
        Some(value) => match value.trim().parse::<f64>() {
            Ok(parsed) => Ok(parsed),
            Err(_) => Err(RatchError::ParseError(format!(
                "Could not take `{}` as a float value",
                value
            ))),
        },
        None => Err(RatchError::ParseError(
            "Interval value not found".to_owned(),
        )),
    }
}

fn main() -> Result<(), RatchError> {
    let matches = App::with_defaults("My Program")
        .author("Aceeri <conmcclusk@gmail.com>")
        .about("A better `watch`")
        .setting(AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name("interval")
                .short("n")
                .long("interval")
                .takes_value(true)
                .default_value("2.0")
                .help("Interval to update the program"),
        ).arg(Arg::with_name("command").required(true).multiple(true))
        .get_matches();

    let interval = parse_interval(matches.value_of("interval"))?;
    println!("Interval: {:?}", interval);

    let command = match matches.values_of("command") {
        Some(command) => command.collect::<Vec<&str>>(),
        None => return Err(RatchError::ParseError("command not found".to_string())),
    };

    println!("Command: {:?}", command.clone());
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        buffer.clear();
        
        let spawn_result = Command::new(command[0])
            .args(&command[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = spawn_result?;
        let _done = child.wait()?;
        //match done.code() {
            //Some(code) => println!("Process exitted with status code: {}", code),
            //None => println!("Process terminated by signal"),
        //}

        if let Some(ref mut stdout) = child.stdout {
            stdout.read_to_end(&mut buffer);
        }

        if let Some(ref mut stderr) = child.stderr {
            stderr.read_to_end(&mut buffer);
        }

        stdout().write(&buffer);

        thread::sleep(Duration::from_nanos((interval * 1_000_000_000.0) as u64));
    }

    Ok(())
}
