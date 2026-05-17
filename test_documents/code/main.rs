use std::collections::HashMap;

/// A simple key-value store.
struct Store {
    data: HashMap<String, String>,
}

impl Store {
    fn new() -> Self {
        Store {
            data: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }
}

fn main() {
    let mut store = Store::new();
    store.set("hello".to_string(), "world".to_string());
    println!("{:?}", store.get("hello"));
}
