use std::io::{stdout, BufWriter, Write};
use crate::crdt::CrdtToDoList;

#[derive(Clone, Debug)]
pub struct Task {
    pub name: String,
    pub status: bool,
}

impl Task {
    pub fn add_task(todo: &mut Vec<Task>, name: String) {
        let task = Task {
            name,
            status: false,
        };
        todo.push(task);
    }

    pub fn remove_task(todo: &mut Vec<Task>, index: usize) {
        if index < todo.len() {
            todo.remove(index);
        } else {
            println!("Invalid index: {}", index);
        }
    }

    pub fn mark_done(todo: &mut Vec<Task>, index: usize) {
        if let Some(task) = todo.get_mut(index) {
            task.status = true;
        } else {
            println!("Invalid index: {}", index);
        }
    }

    pub fn list_tasks(todo: &Vec<Task>) {
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());

        writeln!(writer, "\n\n{:<5} {:<30} {}", "ID", "Name", "Status").unwrap();
        writeln!(writer, "{}", "-".repeat(50)).unwrap();

        for (index, task) in todo.iter().enumerate() {
            writeln!(writer, "{:<5} {:<30} {}", index, task.name, task.status_string()).unwrap();
        }

        writer.flush().unwrap();
    }

    fn status_string(&self) -> &'static str {
        if self.status {
            "✔ Done"
        } else {
            "✘ Not Done"
        }
    }
}

pub fn update_local_list_from_crdt(crdt: &CrdtToDoList, todo: &mut Vec<Task>) {
    todo.clear();
    for entry in &crdt.task_entries {
        todo.push(entry.task.clone());
    }
}
