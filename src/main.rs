extern crate clap;
extern crate duct;
extern crate os_pipe;
extern crate pancurses;

mod error;

use std::io::Read;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use self::error::RatchError;
use clap::{App, AppSettings, Arg};
use duct::Expression;


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

fn run_command(command: &Expression) -> Result<Vec<String>, RatchError> {
    let (mut read, write) = os_pipe::pipe()?;

    let child = command.stdout_handle(write).start()?;

    let mut buffer = String::new();
    read.read_to_string(&mut buffer)?;
    child.wait()?;

    Ok(buffer
        .lines()
        .map(|line| line.to_owned() + "\n")
        .collect::<Vec<String>>())
}

fn run() -> Result<(), RatchError> {
    let matches = App::with_defaults("Ratch")
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
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Prints debug information outside of the window"),
        )
        .arg(
            Arg::with_name("unconstrain")
                .short("u")
                .long("unconstrain")
                .help("Doesn't prevent the cursor from going outside of the output of the program"),
        )
        .arg(Arg::with_name("command").required(true).multiple(true))
        .get_matches();

    let debug = matches.is_present("debug");
    let unconstrained = matches.is_present("unconstrain");

    let interval = parse_interval(matches.value_of("interval"))?;
    if debug {
        println!("Interval: {:?}", interval);
        println!("Unconstrained: {:?}", unconstrained);
    }

    let interval_duration = Duration::from_nanos((interval * 1_000_000_000.0) as u64);
    let interval_secs = interval_duration.as_secs();
    let interval_nanos = interval_duration.subsec_nanos();

    let command = match matches.values_of("command") {
        Some(command) => command.collect::<Vec<&str>>(),
        None => return Err(RatchError::ParseError("command not found".to_string())),
    };

    if debug {
        println!("Command: {:?}", command.clone());
    }

    let command = duct::cmd(command[0], &command[1..]).stderr_to_stdout();
    let window = initscr();
    window.keypad(true);
    window.nodelay(true);
    noecho();

    let mut vertical_cursor: isize = 0;
    let constrain = |cursor: isize, length: usize| -> isize {
        let end = length as isize - window.get_max_y() as isize;
        match cursor {
            x if x < 0 => 0,
            x if x > end => end,
            x => x,
        }
    };

    let (sender, receiver) = mpsc::channel();

    let mut last_instant = Instant::now() - interval_duration;

    let mut current_msg = 0;
    let mut lines = Vec::new();
    'top: loop {
        let mut redraw = false;
        loop {
            match window.getch() {
                Some(Input::KeyDC) | Some(Input::Character('q')) | Some(Input::Character('Q')) => {
                    break 'top
                }
                Some(Input::Character('j')) | Some(Input::KeyDown) => {
                    vertical_cursor = vertical_cursor.saturating_add(1);
                    redraw = true;
                }
                Some(Input::Character('k')) | Some(Input::KeyUp) => {
                    vertical_cursor = vertical_cursor.saturating_sub(1);
                    redraw = true;
                }
                Some(Input::Character('G')) => {
                    let end = (lines.len() - window.get_max_y() as usize) as isize;
                    if end > vertical_cursor {
                        vertical_cursor = end;
                        redraw = true;
                    }
                }
                Some(_) => (),
                None => break,
            }
        }

        if !unconstrained {
            vertical_cursor = constrain(vertical_cursor, lines.len());
        }

        match receiver.try_recv() {
            Ok((msg, split)) => {
                if msg >= current_msg {
                    lines = split;
                    redraw = true;
                }
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => (),
        }

        let elapsed = last_instant.elapsed();
        let secs = elapsed.as_secs();
        let nanos = elapsed.subsec_nanos();

        if interval_secs <= secs && interval_nanos <= nanos {
            last_instant = Instant::now();

            let command = command.clone();
            let command_sender = sender.clone();

            current_msg += 1;
            let counter = current_msg.clone();
            thread::spawn(move || {
                let message = match run_command(&command) {
                    Ok(result) => result,
                    Err(err) => vec![err.to_string()],
                };

                match command_sender.send((counter, message)) {
                    Ok(_sent) => (),
                    Err(_err) => (),
                }
            });
        }

        if redraw {
            window.erase();
            for index in vertical_cursor..(vertical_cursor + window.get_max_y() as isize) {
                if index < 0 || index >= lines.len() as isize {
                    window.printw("\n");
                } else {
                    window.printw(&lines[index as usize]);
                }
            }
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
