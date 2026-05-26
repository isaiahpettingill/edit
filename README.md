# ![Application Icon for Chedit](./crates/edit/assets/edit.svg) Chedit

A bare-bones retro GUI-based text editor with a tiny binary size and no syntax or coding features.

Chedit is a graphical visual text editor paying homage to the classic [MS-DOS Editor](https://en.wikipedia.org/wiki/MS-DOS_Editor), but re-imagined as a minimal, lightweight, visual text editor with a state-of-the-art GUI layout, custom themes, and standard input controls. The goal is to provide a clean, lightweight, visual text editor supporting single-document editing, native file saving/loading, and interactive search/replace highlighting, without any bloated coding or IDE features.

## Features

- **Single-Document Editor**: Focuses on editing one document at a time, preserving the simple workspace model of the original MS-DOS Editor.
- **Custom Title Bar & Controls**: Undecorated viewport with integrated menus, window controls (Minimize `.`, Maximize `+`/`-`, Close `X`), and a middle draggable area that displays the active filename.
- **Interactive Find & Replace**: A clean overlay panel showing real-time search match counts, highlighting results, and enabling individual or global replacements.
- **Flat Retro Aesthetics**: Soft, astigmatism-friendly dark mode (and clean light mode) with flat corners, minimal non-zero margins/paddings, steel-slate blue highlighting, and no purple selection colors.
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
