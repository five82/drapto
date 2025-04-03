# Drapto

HandbrakeCLI video encoding wrapper.

Drapto provides a convenient command-line interface to automate video encoding tasks using HandBrakeCLI. It simplifies the process by allowing you to define encoding presets and apply them easily.

## Features

*   Wraps HandBrakeCLI for powerful and flexible video encoding.
*   Uses TOML configuration files for defining encoding presets (e.g., `handbrake_defaults.toml`).

## Installation

1.  **Install HandBrakeCLI:** Ensure you have `HandBrakeCLI` installed and available in your system's PATH. You can download it from the [official HandBrake website](https://handbrake.fr/downloads2.php).
2.  **Install Rust:** If you don't have Rust installed, follow the instructions at [rustup.rs](https://rustup.rs/).
3.  **Install Drapto:** Install directly from the Git repository using `cargo install`.
    ```bash
    cargo install --git https://github.com/five82/drapto
    ```
    This command clones the repository, builds the `drapto` binary, and installs it to `~/.cargo/bin/`.

    **Important:** Ensure `~/.cargo/bin` is included in your system's PATH environment variable so you can run `drapto` from anywhere.

## Usage

Basic usage involves specifying an input file/directory and an output directory. Drapto will use the default settings defined in `handbrake_defaults.toml` or a specified configuration file.

```bash
# Encode a single file using default settings
drapto encode -i /path/to/input/video.mkv -o /path/to/output/

# Encode all videos in a directory
drapto encode -i /path/to/input/directory/ -o /path/to/output/

# Specify a custom configuration file
# Note: Custom config file loading is not implemented in the provided main.rs yet.
# This example assumes future implementation.
# drapto encode -c /path/to/custom_config.toml -i /path/to/input/video.mkv -o /path/to/output/

# The current implementation uses handbrake_defaults.toml automatically.
# Example using default settings:
drapto encode -i /path/to/input/video.mkv -o /path/to/output/
```