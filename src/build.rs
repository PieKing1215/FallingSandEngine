
use std::process::Command;

use chrono::Utc;

fn main() {
    if let Ok(output) = Command::new("git").args(&["describe", "--always"]).output() {
        let git_hash = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    }

    let now = Utc::now();
    println!("cargo:rustc-env=BUILD_DATETIME={}", now.format("%D %R"));
    
}