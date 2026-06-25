use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::utils;

pub static TARGETS: [&str; 11] = [
    "target",
    ".embuild",
    "venv", ".venv", "env", ".env",
    "node_modules",
    "_build", "deps",
    "vendor",
    "dist",
];


pub fn detect_language(entry_path: &Path, file_name: &str) -> Option<&'static str> {
    let parent = entry_path.parent();
    let has = |f: &str| parent.map(|p| p.join(f).exists()).unwrap_or(false);

    match file_name {
        "node_modules"  => Some("JavaScript"),
        "__pycache__"   => Some("Python"),
        "_build" | "deps" => Some("Elixir"),
        ".stack-work" | "dist-newstyle" => Some("Haskell"),
        "Pods"          => Some("Swift"),
        "target" => {
            if      has("Cargo.toml")   { Some("Rust") }
            else if has("pom.xml")      { Some("Java (Maven)") }
            else if has("build.gradle") { Some("Java (Gradle)") }
            else if has("build.sbt")    { Some("Scala") }
            else                        { None }
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

pub fn lookup(file_map: &mut HashMap<String, Vec<PathBuf>>, path: &Path) {
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
            let key = utils::detect_language(&entry_path, file_name)
                .unwrap_or(file_name)
                .to_string();
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

pub fn format_size(size: f32) -> String {
    let size = size.max(0.0);

    if size > 1_073_741_824.0 {
        format!("[{:>10.2} GB]", size / 1_073_741_824.0)
    } else if size > 1_048_576.0 {
        format!("[{:>10.2} MB]", size / 1_048_576.0)
    } else if size > 1_024.0 {
        format!("[{:>10.2} KB]", size / 1_024.0)
    } else {
        format!("[{:>10.2} B ]", size)
    }
}

