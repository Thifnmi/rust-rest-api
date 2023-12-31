use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "persist")]
use tokio::fs;

/// Represents a single todo item
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TodoItem {
    pub title: String,
    pub notes: String,
    pub assigned_to: String,
    pub completed: bool,
}

/// DTO for patching a todo item
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateTodoItem {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub assigned_to: Option<String>,
    pub completed: Option<bool>,
}

/// Represents a todo item with an id
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentifyableTodoItem {
    pub id: usize,

    #[serde(flatten)]
    pub item: TodoItem,
}

impl IdentifyableTodoItem {
    pub fn new(id: usize, item: TodoItem) -> IdentifyableTodoItem {
        IdentifyableTodoItem { id, item }
    }
}

/// Parameters for pagination
///
/// Used to demonstrate handling of query parameters.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}
impl Pagination {
    pub fn new(offset: Option<usize>, limit: Option<usize>) -> Pagination {
        Pagination { offset, limit }
    }
}

/// Error type for the todo items store
#[derive(thiserror::Error, Debug)]
pub enum TodoStoreError {
    #[error("persistent data store error")]
    FileAccessError(#[from] std::io::Error),
    #[error("serialization error")]
    SerializationError(#[from] serde_json::error::Error),
}

/// Todo items store
#[derive(Default)]
pub struct TodoStore {
    store: HashMap<usize, IdentifyableTodoItem>,
    id_generator: AtomicUsize,
}
impl TodoStore {
    pub fn from_hashmap(store: HashMap<usize, IdentifyableTodoItem>) -> Self {
        let id_generator = AtomicUsize::new(
            store
                .keys()
                .max()
                .map(|v| v + 1)
                .unwrap_or(0),
        );
        TodoStore {
            store,
            id_generator,
        }
    }

    /// Get list of todo items
    ///
    /// Supports pagination.
    pub fn get_todos(&self, pagination: Pagination) -> Vec<IdentifyableTodoItem> {
        self.store
            .values()
            .skip(pagination.offset.unwrap_or(0))
            .take(pagination.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect::<Vec<_>>()
    }

    /// Get a single todo item by id
    pub fn get_todo(&self, id: usize) -> Option<&IdentifyableTodoItem> {
        self.store.get(&id)
    }

    /// Create a new todo item
    pub fn add_todo(&mut self, todo: TodoItem) -> IdentifyableTodoItem {
        let id = self.id_generator.fetch_add(1, Ordering::Relaxed);
        let new_item = IdentifyableTodoItem::new(id, todo);
        self.store.insert(id, new_item.clone());
        new_item
    }

    /// Remove a todo item by id
    pub fn remove_todo(&mut self, id: usize) -> Option<IdentifyableTodoItem> {
        self.store.remove(&id)
    }

    /// Patch a todo item by id
    pub fn update_todo(&mut self, id: &usize, todo: UpdateTodoItem) -> Option<&IdentifyableTodoItem> {
        if let Some(item) = self.store.get_mut(id) {
            if let Some(title) = todo.title {
                item.item.title = title;
            }
            if let Some(notes) = todo.notes {
                item.item.notes = notes;
            }
            if let Some(assigned_to) = todo.assigned_to {
                item.item.assigned_to = assigned_to;
            }
            if let Some(completed) = todo.completed {
                item.item.completed = completed;
            }

            Some(item)
        } else {
            None
        }
    }

    /// Store todo items to disk
    ///
    /// Used to demonstrate error handling.
    #[cfg(feature = "persist")]
    pub async fn persist(&self) -> Result<(), TodoStoreError> {
        const FILENAME: &str = "data.json";

        let json = serde_json::to_string_pretty(&self.store.values().collect::<Vec<&IdentifyableTodoItem>>())
            .map_err(TodoStoreError::SerializationError)?;
        fs::write(FILENAME, json.as_bytes())
            .await
            .map_err(TodoStoreError::FileAccessError)?;
        Ok(())
    }
}

impl From<TodoStore> for HashMap<usize, IdentifyableTodoItem> {
    fn from(value: TodoStore) -> Self {
        value.store
    }
}