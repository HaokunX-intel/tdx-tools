use std::{process::{Command, Output}, io};

pub fn run_command(cmd: String, args: Option<Vec<String>>) -> io::Result<Output> {
    let mut c :Command= Command::new(cmd);
    if let Some(a) = args {
        c.args(a);
    } 
    c.output()
}

