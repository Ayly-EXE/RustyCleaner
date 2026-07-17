# Rusty Cleaner

A cli/tui tool to find and remove targets, builds and packages from forgotten projects.

![Video](https://github.com/Ayly-EXE/Assets/blob/main/rusty_cleaner.gif)

## How it works ? 

The tool scans for known footprints, listed below, and save its path and size. You can then select and delete the desired directories. 

The tool do not delete source files, only builds and packages.


## Targets 

### Matched :

- Rust (target)
- JavaScript (node_modules, dist)
- Python (__pycache__, venv, .venv, env, .env, dist)
- Java (Maven) (target)
- Java (Gradle) (target, build)
- Elixir (_build, deps)
- Haskell (.stack-work, dist-newstyle)
- Swift (Pods, build)
- Ruby (vendor)

### Taget not in the list ?
> **Add it in the utils.rs**

### Unmatched :
- .embuild
- dist

## Usage 
/!\ It is highly recommended to use your dev folder as a starting point for the scan. 
```shell
rusty_clean <path>
```
> _Note_: If no argument is given, start the scan at the current DIR
