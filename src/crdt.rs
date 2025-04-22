use std::borrow::Cow;
use std::fs::File;
use std::io::{stdin, stdout, Read, Write};
use automerge::{AutoCommit, AutomergeError, Change, ObjId, ObjType, ReadDoc, ScalarValue, Value, ROOT};
use automerge::transaction::Transactable;
use crate::display::show_welcome_screen;
use crate::network::Message;
use crate::peer::SharedPeers;
use crate::sync::SyncState;
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

    pub fn add_task_offline(&mut self, task: &Task) -> Result<(), AutomergeError>{
        println!("Adding task to CRDT: {}", task.name);
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

    pub async fn add_task(&mut self, task: &Task, sync_state: &mut SyncState, shared_peers: &SharedPeers) -> Result<(), AutomergeError>{
        print!("Adding task to CRDT: {}", task.name);
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
        self.send_changes(sync_state, shared_peers).await;
        println!("{}", "Press Enter to continue...");
        let mut input = String::new();
        let _ = stdout().flush();
        stdin().read_line(&mut input).expect("Failed to read line");
        show_welcome_screen();
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

    pub fn remove_task_offline(&mut self, index:usize) -> Result<(), AutomergeError>{
        println!("Removing the task from CRDT");
        self.doc.delete(&self.list_id, index)?;
        self.load_tasks()?;
        Ok(())
    }

    pub async fn remove_task(&mut self, index:usize, sync_state: &mut SyncState, shared_peers: &SharedPeers) -> Result<(), AutomergeError>{
        println!("Removing the task from CRDT");
        self.doc.delete(&self.list_id, index)?;
        self.load_tasks()?;
        self.send_changes(sync_state, shared_peers).await;
        println!("{}", "Press Enter to continue...");
        let mut input = String::new();
        let _ = stdout().flush();
        stdin().read_line(&mut input).expect("Failed to read line");
        show_welcome_screen();
        Ok(())
    }

    pub fn mark_done_offline(&mut self, index: usize) -> Result<(), AutomergeError> {
        println!("Marking the task done");
        if index >= self.task_entries.len() {
            println!("Invalid index: {}", index);
            return Ok(());
        }
        let task_id = &self.task_entries[index].obj_id;
        self.doc.put(task_id, "status", true)?;
        self.load_tasks()?;
        println!("{}", "Press Enter to continue...");
        let mut input = String::new();
        let _ = stdout().flush();
        stdin().read_line(&mut input).expect("Failed to read line");
        show_welcome_screen();
        Ok(())
    }

    pub async fn mark_done(&mut self, index: usize, sync_state: &mut SyncState, shared_peers: &SharedPeers) -> Result<(), AutomergeError> {
        println!("Marking the task done");
        if index >= self.task_entries.len() {
            println!("Invalid index: {}", index);
            return Ok(());
        }
        let task_id = &self.task_entries[index].obj_id;
        self.doc.put(task_id, "status", true)?;
        self.send_changes(sync_state, shared_peers).await;
        self.load_tasks()?;
        Ok(())
    }

    pub fn save_to_file(&mut self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let bytes = self.doc.save();
        file.write_all(&bytes)?;
        Ok(())
    }

    pub async fn send_changes(
        &mut self,
        sync_state: &mut SyncState,
        shared_peers: &SharedPeers,
    ) {
        let have_deps = sync_state.get_have_deps();
    
        let changes = self.doc.get_changes(&have_deps);
        let owned_changes: Vec<Change> = changes.iter().map(|c| (*c).to_owned()).collect();
        let raw_changes: Vec<Vec<u8>> = owned_changes.iter().map(|c| c.raw_bytes().to_vec()).collect();
    
        if raw_changes.is_empty() {
            return;
        }
    
        let message = Message::Changes(raw_changes);
    
        let peers = shared_peers.lock().await;
        for (peer_id, peer) in peers.iter() {
            if let Some(sender) = &peer.sender {
                if let Err(e) = sender.send(message.clone()).await {
                    eprintln!("Failed to send changes to {}: {}", peer_id.id, e);
                }
            } else {
                eprintln!("No sender channel found for peer: {}", peer_id.id);
            }
        }
    
        for change in owned_changes {
            sync_state.add_received_change(change.hash());
        }
    }    

    pub async fn apply_changes_from_bytes(
        &mut self,
        raw_changes: Vec<Vec<u8>>,
        sync_state: &mut SyncState
    ) {
        for bytes in raw_changes {
            match Change::from_bytes(bytes) {
                Ok(change) => {
                    if let Err(e) = self.doc.apply_changes(vec![change.clone()]) {
                        eprintln!("Failed to apply change: {}", e);
                        continue;
                    }
                    sync_state.add_received_change(change.hash());
                }
                Err(e) => {
                    eprintln!("Failed to decode change bytes: {}", e);
                }
            }
        }

        if let Err(e) = self.load_tasks() {
            eprintln!("Failed to reload task entries: {}", e);
        }
    }

}