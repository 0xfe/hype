use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Headers {
    pub fields: HashMap<String, Vec<String>>,
}

impl Headers {
    pub fn new() -> Self {
        Headers {
            fields: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into().to_lowercase();
        let values = self.fields.entry(key).or_insert(Vec::new());
        values.push(value.into().trim().into());
    }

    pub fn add_from(&mut self, str: &str) {
        let header: Vec<String> = str.split(':').map(|l| l.trim().to_string()).collect();
        self.add(header[0].clone(), header[1].clone());
    }

    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.fields.get(&key.to_lowercase())
    }

    pub fn get_first_or_set(&mut self, key: &str, default: impl Into<String>) -> &String {
        let values = self.fields.entry(key.to_lowercase()).or_insert(Vec::new());
        values.push(default.into().trim().into());
        values.first().unwrap()
    }

    pub fn get_first(&self, key: &str) -> Option<&String> {
        self.fields.get(&key.to_lowercase()).and_then(|v| v.first())
    }

    pub fn get_last(&self, key: &str) -> Option<&String> {
        self.fields.get(&key.to_lowercase()).and_then(|v| v.last())
    }

    pub fn remove(&mut self, key: &str) {
        self.fields.remove(&key.to_lowercase());
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into().to_lowercase();
        let values = self.fields.entry(key).or_insert(Vec::new());
        values.clear();
        values.push(value.into().trim().into());
    }

    pub fn set_multiple(&mut self, key: impl Into<String>, new_values: Vec<String>) {
        let key = key.into().to_lowercase();
        let values = self.fields.entry(key).or_insert(Vec::new());
        values.clear();
        values.extend(new_values);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.fields.iter()
    }

    pub fn serialize(&self) -> String {
        let mut serialized = String::new();
        for (key, values) in self.fields.iter() {
            for value in values {
                serialized.push_str(&format!("{}: {}\r\n", key, value));
            }
        }

        if serialized.len() > 2 {
            // Remove the last \r\n
            serialized.truncate(serialized.len() - 2);
        }
        serialized
    }
}
