use std::io::{Write, stdout, stdin, BufWriter};
use std::fmt;

struct Task {
    id: usize,
    name: String,
    status: bool,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status_symbol = if self.status { "✔ Done" } else { "✘ Not Done" };
        write!(f, "{:<5} {:<30} {}", self.id, self.name, status_symbol)
    }
}

impl Task {
    fn add_task(todo: &mut Vec<Task>, name: String) {
        let task = Task {
            id: todo.last().map(|todo| todo.id + 1).unwrap_or(1),
            name,
            status: false,
        };
        todo.push(task);
    }

    fn remove_task(todo: &mut Vec<Task>, id: usize) {
        todo.retain(|task| task.id != id);
    }

    fn mark_done(todo: &mut Vec<Task>, id: usize) {
        for task in todo.iter_mut() {
            if task.id == id {
                task.status = true;
            }
        }
    }

    fn list_tasks(todo: &Vec<Task>) {
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());

        writeln!(writer, "\n\n{:<5} {:<30} {}", "ID", "Name", "Status").unwrap();
        writeln!(writer, "{}", "-".repeat(50)).unwrap();

        for task in todo {
            writeln!(writer, "{}", task).unwrap();
        }

        writer.flush().unwrap();
    }
}

fn main() {
    let mut todo: Vec<Task> = Vec::new();

    loop {
        println!("\n1. Add a Task");
        println!("2. Remove a Task");
        println!("3. Mark a Task as done");
        println!("4. List all Tasks");
        println!("5. Quit");
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
                Task::add_task(&mut todo, task_name.trim().to_string());
            }
            2 => {
                print!("Enter task ID to remove: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");
                if let Ok(id) = input.trim().parse() {
                    Task::remove_task(&mut todo, id);
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
            }
            3 => {
                print!("Enter task ID to mark as done: ");
                stdout().flush().unwrap();
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line.");
                if let Ok(id) = input.trim().parse() {
                    Task::mark_done(&mut todo, id);
                } else {
                    println!("Invalid input. Please enter a valid ID.");
                }
            }
            4 => {
                Task::list_tasks(&todo);
            }
            5 => {
                println!("Thank you for using the to-do list!");
                break;
            }
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }
}