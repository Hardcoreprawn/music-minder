# Windows Setup Guide

To run Music Minder natively on Windows (outside of WSL/Dev Container), follow these steps:

## Prerequisites

1.  **Install Rust**:
    - Download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs/).
    - Follow the installation prompts (default installation is usually fine).

2.  **Install Build Tools**:
    - You likely need the **C++ build tools** for Visual Studio.
    - If you don't have Visual Studio installed, you can install the "Build Tools for Visual Studio" and select the "Desktop development with C++" workload.
    - This is required for linking against system libraries.

## Running the App

1.  Open a PowerShell or Command Prompt in the project directory.
2.  Run the application:
    ```powershell
    cargo run
    ```

## Troubleshooting

-   **SQLite**: The `sqlx` crate compiles SQLite bundled by default, so you shouldn't need to install SQLite separately.
-   **File Dialogs**: The `rfd` crate uses native Windows APIs, so no extra setup is needed.
-   **Audio/Fingerprinting**:
    -   Currently, the app only scans metadata using `lofty` (pure Rust).
    -   In the future, if we enable `chromaprint` or `ffmpeg` features, you will need to install those libraries on Windows (e.g., via `vcpkg` or by placing DLLs in the path).
