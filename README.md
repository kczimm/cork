# Cork: A Build System for C Projects

Cork is a Rust-based build tool inspired by [Cargo](https://doc.rust-lang.org/cargo/), designed to streamline the process of managing, building, and running C projects, inspired by Rust's Cargo. It aims to provide a simple, consistent, and efficient way to handle project initialization, building, running, and cleaning for C developers.

## Features

- **Project Initialization**: Create a new C project with a predefined directory structure.
- **Build System**: Compile your C code into executables with ease.
- **Run**: Build and run your project with one command.
- **Clean**: Remove build artifacts to keep your project directory clean.
- **Git Integration**: Automatically initializes a Git repository for version control.

## Getting Started

### Prerequisites

- Rust and Cargo installed on your system ([installation guide](https://www.rust-lang.org/tools/install))
- GCC (or any C compiler that responds to the `gcc` command)

### Installation

To install `cork`, you'll need to build it from source:

```sh
git clone <repository-url>
cd cork
cargo build --release
````

The binary will be located in the target/release directory. You can then move the binary to a directory in your $PATH or use it directly from there.

### Usage
Here's how to use cork for different operations:
- Create a new project:
```sh
cork new <project-name>
````

-Build the project:
```sh
cork build
# or for a release build
cork build --release
```

- Run the project:
```sh
cork run
```
  
- Clean build artifacts:
```sh
cork clean
```

- Short aliases for commands:
- - `cork b` for `build`
- - `cork r` for `run`
- - `cork c` for `clean`

### Project Structure
Cork expects and creates the following project structure:

myproject/
├── src/
│   ├── main.c
│   └── ... other source files
├── include/
│   ├── headers.h
│   └── ... other header files
├── tests/
│   └── ... test files
├── Cork.toml
├── .gitignore
└── .git/

### Commands Overview
`cork new <project-name>`: Generates a new project directory with the basic structure.
`cork build [--release]`: Compiles the project. Use `--release` for optimized builds.
`cork run [--release]`: Builds (if necessary) and runs the project.
`cork clean`: Removes the `build` directory, cleaning up all build artifacts.

### Contributing
Contributions are welcome! Here's how you can contribute:
Fork the repository
Create your feature branch (`git checkout -b feature/AmazingFeature`)
Commit your changes (`git commit -m 'Add some AmazingFeature'`)
Push to the branch (`git push origin feature/AmazingFeature`)
Open a pull request
