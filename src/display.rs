use figlet_rs::FIGfont;
use colored::*;
use std::io::{stdin, stdout, Write};

pub fn show_welcome_screen_start() {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("RustyTasks").unwrap();

    print!("\x1B[2J\x1B[1;1H");

    println!("{}", figure.to_string().bright_cyan().bold());

    println!("{}", "A CRDT-powered, P2P terminal task manager".italic().dimmed());
    println!("{}", "\n\nPress Enter to start...".yellow().bold());

    let mut input = String::new();
    let _ = stdout().flush();
    stdin().read_line(&mut input).expect("Failed to read line");
}

pub fn show_welcome_screen() {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("RustyTasks").unwrap();

    print!("\x1B[2J\x1B[1;1H");

    println!("{}", figure.to_string().bright_cyan().bold());

    println!("{}\n\n", "A CRDT-powered terminal task manager".italic().dimmed());
}

pub fn show_welcome_screen_exit() {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("RustyTasks").unwrap();

    print!("\x1B[2J\x1B[1;1H");

    println!("{}", figure.to_string().bright_cyan().bold());

    println!("{}\n\n", "A CRDT-powered terminal task manager".italic().dimmed());
    println!("\n{}", "Thank you for using the to-do list!".blue().bold());
}