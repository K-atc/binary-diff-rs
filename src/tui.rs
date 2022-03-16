#![feature(stdin_forwarders)]
extern crate binary_diff;
extern crate clap;
extern crate simplelog;
extern crate termion;
extern crate tui_rs as tui;

mod util;

use binary_diff::{BinaryDiff, BinaryDiffChunk};
use clap::{App, Arg};
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, SharedLogger, TermLogger, TerminalMode,
    WriteLogger,
};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use util::event::{Event, Events};

fn main() {
    let mut logger_options: Vec<Box<dyn SharedLogger>> = vec![TermLogger::new(
        LevelFilter::Warn,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )];
    if let Ok(_) = env::var("RUST_LOG") {
        logger_options.push(WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            File::create("binary-diff-tui.log").unwrap(),
        ))
    }
    CombinedLogger::init(logger_options).unwrap();

    let matches = App::new("TUI version of binary diff tool")
        .version("1.0")
        .author("Nao Tomori (@K_atc)")
        .about("Show changes between two binaries in xdd format in TUI")
        .arg(
            Arg::with_name("FILE")
                .help("Files to compare")
                .required(false)
                .multiple(true)
                .index(1),
        )
        .arg(
            Arg::with_name("stdin")
                .help("Takes list of FILE from stdin")
                .long("stdin")
                .takes_value(false),
        )
        .get_matches();

    let files: Vec<PathBuf> = if matches.is_present("stdin") {
        let mut files = Vec::new();
        for line in io::stdin().lines() {
            match line {
                Ok(ref line) => {
                    if line.len() > 0 {
                        files.push(Path::new(line).to_path_buf())
                    } else {
                        log::warn!("Stdin contains *null* line. Skipped this line")
                    }
                }
                Err(why) => panic!("Failed to read line: {:?}", why),
            }
        }
        files
    } else {
        match matches.values_of("FILE") {
            Some(files) => files.map(|file| Path::new(file).to_path_buf()).collect(),
            None => {
                panic!("FILE is not specified")
            }
        }
    };
    log::info!("files = {:?}", files);
    if files.len() < 2 {
        panic!("Specify more than 2 FILEs")
    }

    app(files).unwrap();
}

enum ComparedFile<'a> {
    Before(&'a Path),
    After(&'a Path),
}

fn render_xxd<'a>(
    compared_file: ComparedFile,
    diff: &'a BinaryDiff,
) -> Result<Vec<Spans<'a>>, io::Error> {
    let mut file = match compared_file {
        ComparedFile::Before(path) => File::open(path)?,
        ComparedFile::After(path) => File::open(path)?,
    };

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let highlight_color = match compared_file {
        ComparedFile::Before(_) => Color::Red,
        ComparedFile::After(_) => Color::Blue,
    };
    let highlight_chunks = {
        let mut highlight_chunks: HashSet<usize> = HashSet::new();
        match compared_file {
            ComparedFile::Before(_) => {
                for chunk in diff.chunks() {
                    if let BinaryDiffChunk::Delete(offset, length) = chunk {
                        for i in usize::from(offset.clone())..usize::from(offset + length) {
                            highlight_chunks.insert(i.clone());
                        }
                    }
                }
            }
            ComparedFile::After(_) => {
                let mut offset = 0;
                for chunk in diff.chunks() {
                    match chunk {
                        BinaryDiffChunk::Insert(_, bytes) => {
                            for i in offset..offset + bytes.len() {
                                highlight_chunks.insert(i);
                            }
                            offset += bytes.len()
                        }
                        BinaryDiffChunk::Replace(_, _, bytes) => {
                            for i in offset..offset + bytes.len() {
                                highlight_chunks.insert(i);
                            }
                            offset += bytes.len()
                        }
                        BinaryDiffChunk::Same(_, length) => offset += length,
                        BinaryDiffChunk::Delete(..) => (), // NOTE: This chunk does not affect after file
                    }
                }
            }
        }
        highlight_chunks
    };

    let mut text: Vec<Spans> = Vec::new();
    // text.push(Spans::from(format!("Load {} bytes", bytes.len()))); // DEBUG:
    for line_offset in 0..(1 + bytes.len() / 16) {
        let offset = line_offset * 16;
        let mut line: Vec<Span> = Vec::new();
        line.push(Span::from(format!("{:08x}:", offset)));
        for i in offset + 0..offset + 16 {
            if i % 2 == 0 {
                let color = if (i % 16) > 0
                    && highlight_chunks.contains(&(i - 1))
                    && highlight_chunks.contains(&i)
                {
                    highlight_color
                } else {
                    Color::Reset
                };
                line.push(Span::styled(" ", Style::default().bg(color)))
            }
            if i < bytes.len() {
                let span_text = format!("{:02x}", bytes[i]);
                if highlight_chunks.contains(&i) {
                    line.push(Span::styled(
                        span_text,
                        Style::default().fg(Color::White).bg(highlight_color),
                    ))
                } else {
                    line.push(Span::from(span_text));
                }
            } else {
                line.push(Span::from(format!("  ")));
            }
        }
        line.push(Span::from(" "));
        for i in offset + 0..offset + 16 {
            if i < bytes.len() {
                let byte_char = char::from(bytes[i]);
                let (fg, bg) = if highlight_chunks.contains(&i) {
                    (Color::White, highlight_color)
                } else {
                    (Color::Reset, Color::Reset)
                };
                if byte_char.is_ascii_graphic() {
                    line.push(Span::styled(
                        format!("{}", byte_char),
                        Style::default().fg(fg).bg(bg),
                    ));
                } else {
                    line.push(Span::styled(
                        ".",
                        Style::default().fg(Color::DarkGray).bg(bg),
                    ));
                }
            } else {
                break;
            }
        }
        text.push(Spans::from(line));
    }

    Ok(text)
}

fn app(files: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    // Check if files exists
    for file in files.iter() {
        if let Err(why) = File::open(file) {
            log::error!("Failed to open file {:?}", file);
            return Err(Box::new(why));
        }
    }

    // Calculate diff
    let mut diff_map: HashMap<(&Path, &Path), BinaryDiff> = HashMap::new();
    for (before, after) in files[0..files.len() - 1]
        .iter()
        .zip(files[1..files.len()].iter())
    {
        diff_map.insert(
            (before, after),
            BinaryDiff::new(
                &mut BufReader::new(File::open(before)?),
                &mut BufReader::new(File::open(after)?),
            )
            .unwrap(),
        );
    }
    log::info!("Calculating diff done");

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let mut scroll: u16 = 0;
    let mut page: usize = 0;
    loop {
        terminal.draw(|f| {
            let size = f.size();

            let files_to_be_compared = (files[page / 2].as_path(), files[page / 2 + 1].as_path());
            let diff = diff_map.get(&files_to_be_compared).unwrap();

            let block = Block::default().style(Style::default().bg(Color::Reset).fg(Color::Reset));
            f.render_widget(block, size);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Max(1), Constraint::Percentage(50)].as_ref())
                .split(size);

            let (color_before, color_after) = if page % 2 == 0 {
                (Color::Yellow, Color::Reset)
            } else {
                (Color::Reset, Color::Yellow)
            };
            let title = Spans::from(vec![
                Span::styled(
                    files_to_be_compared
                        .0
                        .file_name()
                        .unwrap()
                        .to_string_lossy(),
                    Style::default().fg(color_before),
                ),
                Span::from(" -> "),
                Span::styled(
                    files_to_be_compared
                        .1
                        .file_name()
                        .unwrap()
                        .to_string_lossy(),
                    Style::default().fg(color_after),
                ),
            ]);
            let paragraph = Paragraph::new(title)
                .style(Style::default())
                .block(Block::default().borders(Borders::NONE))
                .alignment(Alignment::Left);
            f.render_widget(paragraph, chunks[0]);

            let create_block = |title| {
                Block::default()
                    .borders(Borders::TOP)
                    .style(Style::default())
                    .title(Span::styled(
                        title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
            };
            let current_file = files[page / 2 + page % 2].as_path();
            let frame_title = format!("[{}]", current_file.display()).to_string();
            let file = if page % 2 == 0 {
                ComparedFile::Before(current_file)
            } else {
                ComparedFile::After(current_file)
            };
            let paragraph = Paragraph::new(render_xxd(file, &diff).unwrap())
                .style(Style::default())
                .block(create_block(frame_title.as_str()))
                .scroll((scroll, 0))
                .alignment(Alignment::Left);
            f.render_widget(paragraph, chunks[1]);
        })?;

        match events.next()? {
            Event::Input(key) => match key {
                Key::Char('q') => break,
                Key::Right => page = min(page + 1, (files.len() - 1) * 2 - 1),
                Key::Left => page = page.saturating_sub(1),
                Key::Down => scroll = scroll.saturating_add(1),
                Key::Up => scroll = scroll.saturating_sub(1),
                _ => (),
            },
            _ => (),
        };
    }

    Ok(())
}
