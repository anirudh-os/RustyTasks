use std::{io};
use std::io::Read;

// Struct representing a task in the to-do list
struct Task {
    id: usize,         // Unique identifier for the task
    name: String,      // Name/description of the task
    status: bool,      // Status of the task (true for done, false for not done)
}

impl Task {
    // Adds a new task to the todo list
    // Takes a mutable reference to the vector of tasks and the task name
    fn add_task(todo: &mut Vec<Task>, name: String) {
        // Create a new Task with an id based on the last task in the vector or 1 if the vector is empty
        let task = Task {
            id: todo.last().map(|todo| todo.id + 1).unwrap_or(1),
            name,
            status: false, // New tasks are not done by default
        };
        // Push the new task onto the todo list
        todo.push(task);
    }

    // Removes a task from the todo list by its id
    // Takes a mutable reference to the vector of tasks and the task id
    fn remove_task(todo: &mut Vec<Task>, id: usize) {
        // Retain all tasks whose id is not the one passed in (effectively removes the task)
        todo.retain(|task| task.id != id);
    }

    // Marks a task as done by its id
    // Takes a mutable reference to the vector of tasks and the task id
    fn mark_done(todo: &mut Vec<Task>, id: usize) {
        // Iterate over the tasks and find the task with the matching id
        for task in todo.iter_mut() { // Use iter_mut() to get a mutable reference
            if task.id == id {
                task.status = true; // Mark the task as done
            }
        }
    }

    // Lists all tasks in the todo list
    // Takes a reference to the vector of tasks
    fn list_tasks(todo: &Vec<Task>) {
        // Print the header for the task list
        println!("ID\tName\tStatus");
        // Iterate over the tasks and print each task's details
        for task in todo {
            println!("{} {} {}", task.id, task.name, task.status);
        }
    }
}

fn main() {
    // Initialize an empty vector to store tasks
    let mut todo: Vec<Task> = Vec::new();

    // Start an infinite loop to interact with the user
    loop {
        // Display menu options to the user
        println!("1. Add a Task");
        println!("2. Remove a Task");
        println!("3. Mark a Task as done");
        println!("4. List all Tasks");
        println!("5. Quit");
        println!("Enter your choice:");

        // Read the user's choice
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line.");

        // Try to parse the choice as a number (usize)
        let choice: usize = match input.trim().parse() {
            Ok(num) => num, // If successful, store the number
            Err(_) => {
                // If input is invalid, print an error and continue the loop
                println!("Invalid input. Please enter a number.");
                continue;
            }
        };

        // Match on the user's choice to perform the corresponding action
        match choice {
            // Option 1: Add a task
            1 => {
                println!("Enter task name:");
                let mut task_name = String::new();
                io::stdin().read_line(&mut task_name).expect("Failed to read line.");
                let task_name = task_name.trim().to_string(); // Trim newline and convert to String
                Task::add_task(&mut todo, task_name); // Call add_task function
            }
            // Option 2: Remove a task by its ID
            2 => {
                println!("Enter task ID to remove:");
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("Failed to read line.");
                let id: usize = match input.trim().parse() {
                    Ok(num) => num, // Parse the task ID
                    Err(_) => {
                        println!("Invalid input. Please enter a valid ID.");
                        continue;
                    }
                };
                Task::remove_task(&mut todo, id); // Call remove_task function
            }
            // Option 3: Mark a task as done by its ID
            3 => {
                println!("Enter task ID to mark as done:");
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("Failed to read line.");
                let id: usize = match input.trim().parse() {
                    Ok(num) => num, // Parse the task ID
                    Err(_) => {
                        println!("Invalid input. Please enter a valid ID.");
                        continue;
                    }
                };
                Task::mark_done(&mut todo, id); // Call mark_done function
            }
            // Option 4: List all tasks
            4 => {
                Task::list_tasks(&todo); // Call list_tasks function to display all tasks
            }
            // Option 5: Quit the program
            5 => {
                println!("Thank you for using the todo list!"); // Thank the user and exit the loop
                break;
            }
            // Catch-all for invalid choices
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }
}