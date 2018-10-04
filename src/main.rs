extern crate clap;
extern crate duct;
extern crate os_pipe;
extern crate pancurses;

mod error;

use std::io::{stdout, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Instant, Duration};

use self::error::RatchError;
use clap::{App, AppSettings, Arg, SubCommand};

use pancurses::{endwin, initscr, noecho, Input};

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

fn run() -> Result<(), RatchError> {
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
    let interval_duration = Duration::from_nanos((interval * 1_000_000_000.0) as u64);
    let interval_secs = interval_duration.as_secs();
    let interval_nanos = interval_duration.subsec_nanos();

    let command = match matches.values_of("command") {
        Some(command) => command.collect::<Vec<&str>>(),
        None => return Err(RatchError::ParseError("command not found".to_string())),
    };
    let window = initscr();

    window.nodelay(true);
    noecho();

    let mut vertical_cursor: usize = 0;
    println!("Command: {:?}", command.clone());

    let mut last_instant = Instant::now();
    let mut buffer = String::new();
    'top: loop {
        let mut redraw = false;
        loop {
            match window.getch() {
                Some(Input::KeyDC) => break 'top,
                Some(Input::Character('j')) => { vertical_cursor = vertical_cursor.saturating_add(1); redraw = true; },
                Some(Input::Character('k')) => { vertical_cursor = vertical_cursor.saturating_sub(1); redraw = true; },
                Some(_) => (),
                None => break,
            }
        }

        let elapsed = last_instant.elapsed();
        let secs = elapsed.as_secs();
        let nanos = elapsed.subsec_nanos();

        if interval_secs <= secs && interval_nanos <= nanos {
            last_instant = Instant::now();

            buffer.clear();
            let (mut read, write) = os_pipe::pipe()?;
            let child = duct::cmd(command[0], &command[1..])
                .stderr_to_stdout()
                .stdout_handle(write)
                .start()?;

            read.read_to_string(&mut buffer)?;
            child.wait()?;

            redraw = true;
        }

        if redraw {
            let skipped = buffer.lines().skip(vertical_cursor).map(|line| line.to_owned() + "\n").collect::<String>();
            window.erase();
            window.printw(format!("cursor: {}\n", vertical_cursor));
            window.printw(&skipped);
            window.refresh();
        }

        thread::sleep(Duration::from_millis(8));
    }

    Ok(())
}

fn main() -> Result<(), RatchError> {
    let result = run();
    endwin();
    result
}
