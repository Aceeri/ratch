extern crate clap;
extern crate duct;
extern crate os_pipe;
//extern crate pancurses;
extern crate regex;
extern crate termion;

mod error;

use std::io::{stdin, stdout, Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use clap::{App, AppSettings, Arg};
use duct::Expression;
use regex::Regex;
use termion::event::{Event, Key};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use termion::terminal_size;

use self::error::RatchError;

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

    //let mut debug_output = Vec::new();
    //let mut print_debug = |s: String| {
    //if debug {
    //debug_output.push(s);
    //}
    //};
    let command = duct::cmd(command[0], &command[1..]).stderr_to_stdout();
    let mut stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());
    //let window = initscr();
    //window.keypad(true);
    //window.nodelay(true);

    //let mut attributes = Attributes::new();
    //attributes.set_italic(true);
    //window.attron(attributes);
    //noecho();

    let mut vertical_cursor: isize = 0;
    let constrain = |cursor: isize, length: usize| -> isize {
        let y = match terminal_size() {
            Ok((_, y)) => y as isize,
            Err(_) => cursor,
        };
        let end = length as isize - y as isize - 1;
        match cursor {
            x if x < 0 => 0,
            x if x > end => end,
            x => x,
        }
    };

    let (sender, receiver) = mpsc::channel();

    let mut last_instant = Instant::now() - interval_duration;

    let mut searching = false;
    let mut last_search = "".to_owned();
    let mut search = "".to_owned();
    let mut search_regex = None;
    let mut current_msg = 0;
    let mut lines = Vec::new();
    'top: loop {
        let mut redraw = false;
        let stdin = stdin();
        for event in stdin.events() {
            if let Ok(event) = event {
                match event {
                    //Event::Key(Key::Enter)
                    Event::Key(Key::Char('\n'))
                    | Event::Key(Key::Char('\r'))
                        if searching =>
                    {
                        searching = false;
                        search = "".to_owned();
                        redraw = true;
                    }
                    Event::Key(Key::Backspace) | Event::Key(Key::Char('\u{7f}')) if searching => {
                        search.pop();
                        redraw = true;
                    }
                    Event::Key(Key::Char(character)) if searching => {
                        search += &character.to_string();
                        redraw = true;
                    }
                    Event::Key(Key::Char('q')) | Event::Key(Key::Char('Q')) => break 'top,
                    Event::Key(Key::Char('j')) | Event::Key(Key::Down) => {
                        vertical_cursor = vertical_cursor.saturating_add(1);
                        redraw = true;
                    }
                    Event::Key(Key::Char('k')) | Event::Key(Key::Up) => {
                        vertical_cursor = vertical_cursor.saturating_sub(1);
                        redraw = true;
                    }
                    Event::Key(Key::Char('G')) => {
                        if let Ok((_, y)) = terminal_size() {
                            let end = (lines.len() - y as usize) as isize;
                            if end > vertical_cursor {
                                vertical_cursor = end;
                                redraw = true;
                            }
                        }
                    }
                    Event::Key(Key::Char('g')) => {
                        vertical_cursor = 0;
                        redraw = true;
                    }
                    Event::Key(Key::Char('/')) => {
                        searching = true;
                        search = "".to_owned();
                        redraw = true;
                    }
                    Event::Key(Key::PageUp) => {
                        if let Ok((x, y)) = terminal_size() {
                            vertical_cursor += (y - 1) as isize;
                            redraw = true;
                        }
                    }
                    Event::Key(Key::PageDown) => {
                        if let Ok((x, y)) = terminal_size() {
                            vertical_cursor -= (y - 1) as isize;
                            redraw = true;
                        }
                    }
                    _ => {}
                }
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
            if last_search != search {
                if search == "" {
                    search_regex = None;
                } else {
                    search_regex = Some(Regex::new(&search)?);
                }

                last_search = search.clone();
            }

            write!(stdout, "{}", termion::clear::All).unwrap();

            if let Ok((_, y)) = terminal_size() {
                for index in vertical_cursor..((vertical_cursor + y as isize) - 1) {
                    if index < 0 || index >= lines.len() as isize {
                        write!(stdout, "\n").unwrap();
                    } else {
                        let line = &lines[index as usize];
                        if let Some(ref search_regex) = search_regex {
                            if let Some(mat) = search_regex.find(line) {
                                let (start, end) = (mat.start(), mat.end());
                                //window.addstr("\x1B[48m");
                                //window.addch('1');
                                //window.addch('B');
                                //window.addch('[');
                                //window.addch('4');
                                //window.addch('8');
                                //window.addch('m');
                                //window.printw(format!("{}, {};", start, end));
                                //window.addstr("test");
                                //window.attroff(ColorPair(1));
                                //window.attroff(attributes);
                            }
                        }
                        write!(stdout, "{}{}", termion::cursor::Goto(0, index as u16), line);
                    }
                }

                if searching {
                    write!(stdout, "/{}", search).unwrap();
                } else {
                    write!(stdout, ":").unwrap();
                }

                stdout.flush().unwrap();
            }
        }

        thread::sleep(Duration::from_millis(8));
    }

    //for line in debug_output {
    //println!("{}", line);
    //}
    Ok(())
}

fn main() -> Result<(), RatchError> {
    let result = run();
    //endwin();
    result
}
