mod crdt;
mod tasks;
mod cli;
mod network;
mod peer;
mod identity;

use clap::Parser;
use cli::{Cli, Commands};
use tasks::Task;
use crdt::CrdtToDoList;
use std::io::{stdin, stdout, Write};
use serde::de::Unexpected::Str;
use crate::network::connect_to_peer;

fn main() {
    let cli = Cli::parse();
    let mut crdt = match CrdtToDoList::new(Some("autocommit_doc.automerge")) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Failed to initialize CRDT document: {e}");
            std::process::exit(1);
        }
    };

    let mut todo: Vec<Task> = crdt.task_entries.iter().map(|e| e.task.clone()).collect();

    match &cli.command {
        Some(Commands::Interactive) | None => {
            run_interactive(&mut crdt, &mut todo);
        }

        Some(Commands::Add { name }) => {
            Task::add_task(&mut todo, name.trim().to_string());
            match crdt.add_task(todo.last().unwrap()) {
                Ok(()) => {},
                Err(e) => println!("An error \"{}\" has occurred!", e),
            };
        }

        Some(Commands::Remove { index }) => {
            Task::remove_task(&mut todo, *index);
            match crdt.remove_task(*index) {
                Ok(()) => {},
                Err(e) => println!("An error \"{}\" has occurred!", e),
            };
        }

        Some(Commands::Done { index }) => {
            Task::mark_done(&mut todo, *index);
            match crdt.mark_done(*index) {
                Ok(()) => {},
                Err(e) => println!("An error \"{}\" has occurred!", e),
            };
        }

        Some(Commands::List) => {
            Task::list_tasks(&todo);
        }
    }

    crdt.save_to_file("autocommit_doc.automerge").unwrap()
}

fn run_interactive(crdt: &mut CrdtToDoList, todo: &mut Vec<Task>) {
    loop {
        println!("\n1. Add a Task");
        println!("2. Remove a Task");
        println!("3. Mark a Task as done");
        println!("4. List all Tasks");
        println!("5. Connect to a Peer");
        println!("6. Quit");
        print!("Enter your choice: ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).expect("Failed to read line.");

        let choice: usize = match input.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Invalid input. Please enter a number.");
                continue;
            }
        };

        match choice {
            1 => {
                print!("Enter task name: ");
                stdout().flush().unwrap();
                let mut task_name = String::new();
                stdin().read_line(&mut task_name).expect("Failed to read line.");
                Task::add_task(todo, task_name.trim().to_string());
                if let Some(task) = todo.last() {
                    match crdt.add_task(task) {
                        Ok(()) => {},
                        Err(e) => println!("An error \"{}\" has occurred!", e),
                    };
                }

            },
            2 => {
                print!("Enter task ID to remove: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");
                if let Ok(index) = input.trim().parse::<usize>() {
                    Task::remove_task(todo, index);
                    match crdt.remove_task(index) {
                        Ok(()) => {},
                        Err(e) => println!("An error \"{}\" has occurred!", e),
                    };
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
            },
            3 => {
                print!("Enter task ID to mark as done: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");
                if let Ok(index) = input.trim().parse::<usize>() {
                    Task::mark_done(todo, index);
                    match crdt.mark_done(index) {
                        Ok(()) => {},
                        Err(e) => println!("An error \"{}\" has occurred!", e),
                    };
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
            },
            4 => {
                Task::list_tasks(todo);
            },
            5 => {
                println!("Enter the IP Address of the Peer: ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read the input!");
                let ip = input.trim().to_string();

                println!("Enter the port number: ");
                std::io::stdin().read_line(&mut input).expect("Failed to read the input!");
                let port = input.trim().parse::<u16>().expect("Failed to parse the port number!");

                connect_to_peer(ip, port, peer_id, public_key);
            },
            6 => {
                crdt.save_to_file("autocommit_doc.automerge").unwrap();
                println!("Thank you for using the to-do list!");
                break;
            },
            _ => {
                println!("Invalid choice. Please try again.");
            },
        }
    }
}
