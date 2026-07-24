#[derive(Debug)]
pub struct Exception {
    pub message: String,
    pub type_name: String,
}

impl Exception {
    pub fn new(message: &str, type_name: &str) -> Self {
        Self {
            message: message.to_string(),
            type_name: type_name.to_string(),
        }
    }

    pub fn throw(&self) {
        eprintln!("{}: {}", self.type_name, self.message);
        std::process::abort();
    }
}

pub fn throw(message: &str, type_name: &str) {
    Exception::new(message, type_name).throw();
}
