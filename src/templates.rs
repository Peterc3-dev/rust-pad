pub const TEMPLATES: &[(&str, &str)] = &[
    (
        "Hello World",
        r#"fn main() {
    println!("Hello, world!");
}
"#,
    ),
    (
        "File I/O",
        r#"use std::fs;
use std::io::Write;

fn main() {
    // Write
    let mut file = fs::File::create("/tmp/rust_pad_test.txt").unwrap();
    writeln!(file, "Hello from rust-pad!").unwrap();

    // Read
    let content = fs::read_to_string("/tmp/rust_pad_test.txt").unwrap();
    println!("Read: {}", content);
}
"#,
    ),
    (
        "HTTP Request (reqwest)",
        r#"//! dep: reqwest
//! dep: tokio

// NOTE: This requires cargo project mode (future feature).
// For now, this is a template reference.

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let body = reqwest::get("https://httpbin.org/ip")
//         .await?
//         .text()
//         .await?;
//     println!("{}", body);
//     Ok(())
// }

fn main() {
    println!("HTTP template — needs cargo mode for external deps");
    println!("Use: cargo new myproject && cd myproject");
    println!("Add reqwest + tokio to Cargo.toml");
}
"#,
    ),
    (
        "JSON Parsing",
        r##"//! dep: serde_json

// Simple JSON parsing with just std (no serde needed for basic use)
fn main() {
    let data = r#"{"name": "rust-pad", "version": 1, "features": ["editor", "runner"]}"#;

    // Manual parsing approach without external deps:
    println!("Raw JSON string: {}", data);

    // For real JSON work, use serde_json (requires cargo project):
    // let v: serde_json::Value = serde_json::from_str(data).unwrap();
    // println!("name = {}", v["name"]);
}
"##,
    ),
    (
        "Thread Spawning",
        r#"use std::thread;
use std::sync::{Arc, Mutex};

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for i in 0..5 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
            println!("Thread {} incremented counter to {}", i, *num);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final count: {}", *counter.lock().unwrap());
}
"#,
    ),
];
