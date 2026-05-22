# ![Application Icon for Edit](./assets/edit.svg) Edit

A modern, premium graphical text editor built with Rust, `egui`, and `eframe`.

This editor is a graphical redesign paying homage to the classic [MS-DOS Editor](https://en.wikipedia.org/wiki/MS-DOS_Editor), but re-imagined with a state-of-the-art GUI layout, custom themes, and standard input controls. The goal is to provide a clean, lightweight, visual text editor supporting single-document editing, native file saving/loading, and interactive search/replace highlighting.

## Features

- **Single-Document Editor**: Focuses on editing one document at a time, preserving the simple workspace model of the original MS-DOS Editor.
- **Custom Title Bar & Controls**: Undecorated viewport with integrated menus, window controls (Minimize `-`, Maximize `+`, Close `X`), and a middle draggable area that displays the active filename.
- **Interactive Find & Replace**: A clean overlay panel showing real-time search match counts, highlighting results, and enabling individual or global replacements.
- **Flat Premium Aesthetics**: Curated slate-grey dark mode (and clean light mode) with flat corners, minimal non-zero margins/paddings, steel-slate blue highlighting, and no purple selection colors.
- **Familiar Keyboard Shortcuts**:
  - `Ctrl + N`: New File
  - `Ctrl + O`: Open File...
  - `Ctrl + S`: Save
  - `Ctrl + Shift + S`: Save As...
  - `Ctrl + W`: Close File
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
