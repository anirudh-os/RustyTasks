mod crdt;
mod tasks;
mod cli;
mod network;
mod peer;
mod identity;
mod sync;

use std::collections::HashMap;
use clap::Parser;
use cli::{Cli, Commands};
use tasks::Task;
use crdt::CrdtToDoList;
use sync::SyncState;
use std::io::{stdin, stdout, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use identity::Identity;
use network::{connect_to_peer, connections};
use peer::SharedPeers;
use crate::tasks::update_local_list_from_crdt;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Wrap crdt in Arc<Mutex<>> immediately
    let crdt_arc = Arc::new(Mutex::new(
        CrdtToDoList::new(Some("autocommit_doc.automerge")).unwrap_or_else(|e| {
            eprintln!("Failed to initialize CRDT document: {e}");
            std::process::exit(1);
        }),
    ));

    let mut todo: Vec<Task> = crdt_arc.lock().await.task_entries.iter().map(|e| e.task.clone()).collect();

    match &cli.command {
        Some(Commands::Interactive) | None => {
            run_interactive(&mut todo, crdt_arc.clone()).await;
        }

        Some(Commands::Add { name }) => {
            Task::add_task(&mut todo, name.trim().to_string());
            if let Some(task) = todo.last() {
                crdt_arc.lock().await.add_task_offline(task).unwrap_or_else(|e| {
                    println!("An error \"{}\" has occurred!", e);
                });
            }
        }

        Some(Commands::Remove { index }) => {
            Task::remove_task(&mut todo, *index);
            crdt_arc.lock().await.remove_task_offline(*index).unwrap_or_else(|e| {
                println!("An error \"{}\" has occurred!", e);
            });
        }

        Some(Commands::Done { index }) => {
            Task::mark_done(&mut todo, *index);
            crdt_arc.lock().await.mark_done_offline(*index).unwrap_or_else(|e| {
                println!("An error \"{}\" has occurred!", e);
            });
        }

        Some(Commands::List) => {
            Task::list_tasks(&todo);
        }
    }

    crdt_arc.lock().await.save_to_file("autocommit_doc.automerge").unwrap();
}

async fn run_interactive(todo: &mut Vec<Task>, crdt: Arc<Mutex<CrdtToDoList>>) {
    let crdt_for_network = crdt.clone();

    let shared_peers: SharedPeers = Arc::new(Mutex::new(HashMap::new()));
    let peers_for_network = shared_peers.clone();

    let sync_state = Arc::new(Mutex::new(SyncState::new()));
    let sync_state_peers = sync_state.clone();
    tokio::spawn(async move {
        if let Err(e) = connections(peers_for_network, crdt_for_network, sync_state_peers).await {
            println!("No peers are available: {}!", e);
        }
    });
    let identity = Identity::generate();
    let peer_id = identity.derive_peer_id();
    let public_key = identity.public_key;
    // let private_key = identity.private_key;

    loop {
        let crdt_guard = crdt.lock().await;
        update_local_list_from_crdt(&crdt_guard, todo);
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

                Task::add_task(todo, task_name.clone());
                if let Some(task) = todo.last() {
                    let mut crdt_guard = crdt.lock().await;
                    let mut sync = sync_state.lock().await;
                    let peers = &shared_peers;

                    match crdt_guard.add_task(task, &mut sync, &peers).await {
                        Ok(()) => {},
                        Err(e) => println!("An error \"{}\" has occurred!", e),
                    }
                    update_local_list_from_crdt(&crdt_guard, todo);
                }
                crdt.lock().await.save_to_file("autocommit_doc.automerge").unwrap()
            },
            2 => {
                print!("Enter task ID to remove: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");

                if let Ok(index) = input.trim().parse::<usize>() {
                    let mut crdt_guard = crdt.lock().await;
                    let mut sync = sync_state.lock().await;
                    let peers = &shared_peers;

                    match crdt_guard.remove_task(index, &mut sync, &peers).await {
                        Ok(()) => {},
                        Err(e) => println!("An error \"{}\" has occurred!", e),
                    }
                    update_local_list_from_crdt(&crdt_guard, todo);
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
                crdt.lock().await.save_to_file("autocommit_doc.automerge").unwrap()
            },
            3 => {
                print!("Enter task ID to mark as done: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");

                if let Ok(index) = input.trim().parse::<usize>() {
                    let mut crdt_guard = crdt.lock().await;
                    let mut sync = sync_state.lock().await;
                    let peers = &shared_peers;

                    match crdt_guard.mark_done(index, &mut sync, &peers).await {
                        Ok(()) => {},
                        Err(e) => { println!("An error \"{}\" has occurred!", e) },
                    }
                    update_local_list_from_crdt(&crdt_guard, todo);
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
                crdt.lock().await.save_to_file("autocommit_doc.automerge").unwrap()
            },
            4 => {
                Task::list_tasks(todo);
            },
            5 => {
                println!("Enter the IP Address of the Peer: ");
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read the input!");
                let ip = input.trim().to_string();

                let peer_id_clone = peer_id.clone();
                let shared_peers_clone = shared_peers.clone();
                let crdt_clone = crdt.clone();
                let sync_state_clone = sync_state.clone();

                tokio::spawn(async move {
                    if let Err(e) = connect_to_peer(
                        ip.clone(),
                        peer_id_clone,
                        public_key,
                        shared_peers_clone,
                        crdt_clone,
                        sync_state_clone,
                    ).await {
                        println!("Failed to connect to peer {}: {}", ip, e);
                    }
                });
            },
            6 => {
                crdt.lock().await.save_to_file("autocommit_doc.automerge").unwrap();
                println!("Thank you for using the to-do list!");
                break;
            },
            _ => {
                println!("Invalid choice. Please try again.");
            },
        }
    }
}