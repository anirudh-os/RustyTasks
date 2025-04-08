# RustyTasks

RustyTasks is a peer-to-peer, CRDT-backed command-line To-Do List application written in Rust. It builds on core Rust principles while experimenting with distributed state synchronization using [Automerge](https://github.com/automerge/automerge).

## Features

- Add new tasks
- Remove tasks by index
- Mark tasks as completed
- List all tasks with their status
- CRDT-backed storage using Automerge
- Persistence across sessions via save/load
- (WIP) Peer-to-peer synchronization

## How to Run

1. Ensure you have Rust installed: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2. Clone this repository:
   ```sh
   git clone https://github.com/anirudh-os/RustyTasks.git
   cd RustyTasks
   ```
3. Build and run the app:
   ```sh
   cargo run
   ```

## Directory Structure

- `main.rs`: CLI interface and interactive loop
- `tasks.rs`: Task definition and core logic
- `crdt.rs`: Automerge-based CRDT implementation
- `network.rs`: (WIP) Handles peer-to-peer communication

## Built With Rust Concepts

- Structs and enums to model task data
- Vectors and ownership-safe iteration
- Pattern matching for input handling
- Borrowing and lifetime safety
- Error handling with `Result` and `Option`
- External crate integration (`automerge`)

## Future Enhancements

- **Peer-to-peer sync:** Automatic CRDT state sharing over TCP
- **Conflict resolution:** CRDT-based merge guarantees
- **Task Prioritization:** Add priority levels and sorting
- **Due Dates and Reminders**
- **Filtering Options:** View only completed or pending tasks
- **Optional TUI:** Terminal interface using `tui-rs`

## Contributing

Contributions are welcome! Feel free to fork the repository, open issues, or submit pull requests for improvements or features.

## License

This project is open-source and available under the MIT License.
