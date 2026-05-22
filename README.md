# ![Application Icon for Edit](./assets/edit.svg) Edit

A modern, premium graphical text editor built with Rust, `egui`, and `eframe`.

This editor is a graphical redesign paying homage to the classic [MS-DOS Editor](https://en.wikipedia.org/wiki/MS-DOS_Editor), but re-imagined with a state-of-the-art GUI layout, custom themes, and standard input controls. The goal is to provide a clean, lightweight, visual text editor supporting multiple tabs, native file saving/loading, and interactive search/replace highlighting.

## Features

- **Multi-Tab Document Management**: Open, switch between, and close multiple documents concurrently. Indicates unsaved changes with a dirty `*` status.
- **Interactive Find & Replace**: A clean overlay panel showing real-time search match counts, highlighting results, and enabling individual or global replacements.
- **Premium Aesthetics**: Slate-grey visual aesthetics, a curated dark mode (with options to toggle light mode), smooth rounded button frames, and configurable font sizing.
- **Familiar Keyboard Shortcuts**:
  - `Ctrl + N`: New File
  - `Ctrl + O`: Open File...
  - `Ctrl + S`: Save
  - `Ctrl + Shift + S`: Save As...
  - `Ctrl + W`: Close Tab
  - `Ctrl + F`: Search / Find
  - `Ctrl + H` / `Ctrl + R`: Replace Mode
  - `Ctrl + Q`: Quit Application
- **Native File Dialogs**: Integrated native system file picker and save prompts powered by `rfd`.

---

## Build & Run Instructions

### Prerequisites

To compile and run the graphical interface on Linux, make sure you have the standard windowing and input development packages installed (e.g. `libx11-dev` and `libasound2-dev` on Debian/Ubuntu systems).

### Build

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone the repository.
3. Build the application:
   ```sh
   cargo build --release
   ```
4. Run the editor:
   ```sh
   cargo run --release -- [FILE]
   ```

---

## License

Licensed under the MIT License.
