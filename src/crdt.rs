use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use automerge::{AutoCommit, AutomergeError, ObjId, ObjType, ReadDoc, ScalarValue, Value, ROOT};
use automerge::transaction::Transactable;
use crate::tasks::Task;

pub struct CrdtToDoList {
    doc: AutoCommit,
    list_id: ObjId,
    pub task_entries: Vec<TaskEntry>,
}

pub struct TaskEntry {
    pub obj_id: ObjId,
    pub task: Task,
}

impl CrdtToDoList {
    pub fn new(path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut doc = if let Some(path) = path {
            match File::open(path) {
                Ok(mut file) => {
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)?;
                    AutoCommit::load(&bytes)?
                },
                Err(_) => {
                    AutoCommit::new()
                }
            }
        } else {
            AutoCommit::new()
        };

        let list_id = doc
            .get(ROOT, "tasks")?
            .and_then(|(val, obj_id)| {
                if matches!(val, Value::Object(ObjType::List)) {
                    Some(obj_id)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // If "tasks" list doesn't exist, create it
                doc.put_object(ROOT, "tasks", ObjType::List)
                    .expect("Failed to create task list")
            });

        let mut todo_list = CrdtToDoList {
            doc,
            list_id,
            task_entries: Vec::new(),
        };

        todo_list.load_tasks()?;

        Ok(todo_list)
    }

    pub fn add_task(&mut self, task: &Task) -> Result<(), AutomergeError>{
        let index = self.doc.length(&self.list_id);
        let task_obj = self.doc.insert_object(&self.list_id, index, ObjType::Map)?;
        self.doc.put(&task_obj, "name", task.name.clone())?;
        self.doc.put(&task_obj, "status", task.status)?;
        self.task_entries.push(TaskEntry {
            obj_id: task_obj,
            task: Task {
                name: task.name.clone(),
                status: task.status,
            },
        });
        Ok(())
    }

    fn load_tasks(&mut self) -> Result<(), AutomergeError> {
        self.task_entries.clear();

        let len = self.doc.length(&self.list_id);

        for i in 0..len {
            let (_, obj_id) = self.doc.get(&self.list_id, i)?.ok_or(AutomergeError::InvalidIndex(i))?;

            let name_val = self.doc.get(&obj_id, "name")?.ok_or(AutomergeError::InvalidIndex(i))?;

            let status_val = self.doc.get(&obj_id, "status")?.ok_or(AutomergeError::InvalidIndex(i))?;

            let name_str = match name_val.0 {
                Value::Scalar(Cow::Borrowed(ScalarValue::Str(s))) => s.to_string(),
                Value::Scalar(Cow::Owned(ScalarValue::Str(s))) => s.to_string(),
                _ => {
                    eprintln!("Unexpected format for task name at index {}", i);
                    continue;
                }
            };

            let status_bool = match status_val.0 {
                Value::Scalar(Cow::Borrowed(ScalarValue::Boolean(b))) => *b,
                Value::Scalar(Cow::Owned(ScalarValue::Boolean(b))) => b,
                _ => {
                    eprintln!("Unexpected format for task status at index {}", i);
                    continue;
                }
            };

            let task = Task {
                name: name_str,
                status: status_bool,
            };

            self.task_entries.push(TaskEntry { obj_id, task });
        }

        Ok(())
    }

    pub fn remove_task(&mut self, index:usize) -> Result<(), AutomergeError>{
        self.doc.delete(&self.list_id, index)
    }

    pub fn mark_done(&mut self, index: usize) -> Result<(), AutomergeError> {
        if index >= self.task_entries.len() {
            println!("Invalid index: {}", index);
            return Ok(());
        }
        let task_id = &self.task_entries[index].obj_id;
        self.doc.put(task_id, "status", true)
    }

    pub fn save_to_file(&mut self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let bytes = self.doc.save();
        file.write_all(&bytes)?;
        Ok(())
    }
}