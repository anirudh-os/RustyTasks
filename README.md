# RustyTasks

A peer-to-peer, CRDT‑backed command‑line To‑Do list application written in Rust. RustyTasks leverages [Automerge](https://github.com/automerge/automerge) for conflict‑free data replication and experiments with secure, decentralized synchronization over TCP using [tokio](https://tokio.rs/).

---

## Features

- **CRUD Tasks**: Create, read, update (mark done), and delete tasks.
- **CRDT Sync**: Underlying Automerge document for conflict‑free merges.
- **Persistence**: Automatic save/load of task history to disk.
- **P2P Networking**: Secure synchronization between peers via TCP.
- **Asynchronous**: Uses tokio for efficient and non-blocking communication
- **Extensible**: Modular `tasks`, `crdt`, and `network` components.
- **Offline support**: This can be used offline as a standalone application.

---

## Usage

```text
USAGE:
    rustytasks [OPTIONS]

COMMANDS:
    --interactive            Start the application in the interactive mode
    --list                   List all tasks
    --add <TASK>             Add a task to the task-list
    --remove <TASKID>        Remove a task from the task-list
    --done <TASKID>          Mark a task as done
OPTIONS:
    -h, --help               Print help information
```

### Local/Offline Mode

Run any command except `--interactive` to use the application offline.

### P2P/Interactive Mode

- User can connect to peer(s) using the IP address(es) of the peer(s).
- Peers will perform a handshake, exchange Automerge state, and merge tasks.
- The changes will be sent everytime a command is run while in the interactive mode.

---

## Configuration

- Default data file: `autocommit_doc.automerge` in current directory.

---

## Directory Structure

```
├── Cargo.toml      # Project metadata & dependencies
├── src
│   ├── main.rs     # CLI & entry point
│   ├── tasks.rs    # Task struct & operations
│   ├── crdt.rs     # Automerge integration
│   └── network.rs  # P2P networking (WIP)
|   └── identity.rs # Creates an identity for the peer
|   └── cli.rs      # clap config for cli
|   └── sync.rs     # Synchronization related functionality
|   └── tasks.rs    # Manages the local Task vector
```

---

## Roadmap

- [ ] Automatic peer discovery
- [ ] Enhanced conflict resolution
- [ ] Task priorities & due dates
- [ ] Reminder/notification support
- [ ] Optional TUI (via `tui-rs`)

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/YourFeature`).
3. Commit your changes (`git commit -m 'Add YourFeature'`).
4. Push to the branch (`git push origin feature/YourFeature`).
5. Open a Pull Request.

---

*Happy tasking with RustyTasks!*
