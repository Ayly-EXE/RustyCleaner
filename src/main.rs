use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use clap::Parser;
use fs_extra::dir::get_size;

static TARGETS: [&str; 4] = ["target", ".embuild", "venv", ".env"];

#[derive(Parser)]
struct Cli {
    path: Option<PathBuf>,
}


fn detect_language(entry_path: &Path, file_name: &str) -> Option<&'static str> {
    let parent = entry_path.parent();
    let has = |f: &str| parent.map(|p| p.join(f).exists()).unwrap_or(false);

    match file_name {
        "node_modules"  => Some("JavaScript"),
        "__pycache__"   => Some("Python"),
        "_build" | "deps" => Some("Elixir"),
        ".stack-work" | "dist-newstyle" => Some("Haskell"),
        "Pods"          => Some("Swift"),

        "target" => {
            if      has("Cargo.toml")    { Some("Rust") }
            else if has("pom.xml")       { Some("Java (Maven)") }
            else if has("build.gradle")  { Some("Java (Gradle)") }
            else if has("build.sbt")     { Some("Scala") }
            else                         { None }
        }
        "build" => {
            if      has("build.gradle")  { Some("Java (Gradle)") }
            else if has("Package.swift") { Some("Swift") }
            else                         { None }
        }
        "venv" | "env" | ".venv" | ".env" => {
            if entry_path.join("pyvenv.cfg").exists() { Some("Python") } else { None }
        }
        "vendor" => {
            if      has("composer.json") { Some("PHP") }
            else if has("Gemfile")       { Some("Ruby") }
            else                         { None }
        }
        "dist" => {
            if      has("package.json")  { Some("JavaScript") }
            else if has("pyproject.toml") || has("setup.py") { Some("Python") }
            else                         { None }
        }

        _ => None,
    }
}

fn lookup(file_map: &mut HashMap<String, Vec<PathBuf>>, path: &Path) {
    if !path.is_dir() {
        return;
    }

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();

        let file_name = match entry_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let is_target = TARGETS.contains(&file_name);

        if is_target && entry_path.is_dir() {

            let key = detect_language(&entry_path, file_name).unwrap_or(file_name).to_string();

            file_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(entry_path.clone());

            continue;
        }

        if entry_path.is_dir() {
            lookup(file_map, &entry_path);
        }
    }
}

fn main() {
    let args = Cli::parse();

    let start_path = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let handle = thread::spawn(move || {
        let mut file_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        lookup(&mut file_map, &start_path);
        file_map
    });

    let spinner = ["|", "/", "-", "\\"];
    let mut i = 0;

    while !handle.is_finished() {
        print!("\rScanning {} ", spinner[i % spinner.len()]);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        i += 1;
        thread::sleep(Duration::from_millis(100));
    }
    println!("\rScan completed!   ");

    let file_map = handle.join().unwrap();

    let all_paths: Vec<PathBuf> = file_map.values().flatten().cloned().collect();

    let size_handle = thread::spawn(move || {
        let mut file_size: HashMap<PathBuf, u64> = HashMap::new();
        for path in all_paths {
            let dir_size = get_size(&path).unwrap_or(0);
            file_size.insert(path, dir_size);
        }
        file_size
    });

    i = 0;

    while !size_handle.is_finished() {
        print!("\rCalculating size {} ", spinner[i % spinner.len()]);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        i += 1;
        thread::sleep(Duration::from_millis(100));
    }
    println!("\rCalculating size completed!   ");

    let file_size = size_handle.join().unwrap();

    println!("Results:");
    for (key, paths) in &file_map {
        println!("\n[{}]:", key);
        for path in paths {
            let size = *file_size.get(path).unwrap_or(&0) as f32;

            let size_print = if size > 1_073_741_824.0 {
                format!("[{:>10.2} GB]", size / 1_073_741_824.0)
            } else if size > 1_048_576.0 {
                format!("[{:>10.2} MB]", size / 1_048_576.0)
            } else if size > 1_024.0 {
                format!("[{:>10.2} KB]", size / 1_024.0)
            } else {
                format!("[{:>10.2} B ]", size)
            };

            println!("  {:<10} {}", size_print , path.display());
        }
    }
}