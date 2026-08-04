# Regex Tester TUI

An interactive terminal UI for building and testing regular expressions with live match highlighting.

---

### ⚙️ How It Works

- **Split-pane layout**: A pattern pane on top, a test-text pane in the middle, and a live results pane at the bottom.
- **Live matching**: Every keystroke re-runs the regex against the test text and updates the highlights and match list in real time.
- **Capture group breakdown**: Each match shows its byte range and the value of every named or numbered capture group.
- **Invalid-pattern feedback**: Bad regexes are flagged in the pattern title, and the error message appears in the results pane so you can see exactly what went wrong.

---

## 📁 Setup

### 1. Requirements

- Rust 1.75 or higher
- A terminal that supports ANSI colors and the alternate screen

### 2. Installation

```bash
git clone https://github.com/fantasywastaken/Regex-Tester-TUI.git
cd Regex-Tester-TUI
cargo build --release
```

Binary will be at `target/release/regex-tester`.

---

### 🚀 Usage

```bash
regex-tester
```

Keys:

- `Tab` switches between the pattern and test-text panes
- `Ctrl+U` clears the active pane
- `Enter` inserts a newline in the test-text pane
- `Esc` or `Ctrl+C` exits

Example screen:

```
+-- Pattern ------------------------------+
| (\w+)@(\w+\.\w+)                        |
+-----------------------------------------+
+-- Test Text ----------------------------+
| Contact us at alice@example.com ...     |
+-----------------------------------------+
+-- Matches & Capture Groups -------------+
| Matches: 3                              |
|   #1  'alice@example.com'  [14..31]     |
|       group 1: 'alice'                  |
|       group 2: 'example.com'            |
+-----------------------------------------+
```

---

### ✨ Features

- ✅ Live, keystroke-by-keystroke regex evaluation
- ✅ Highlighted matches inside the test text
- ✅ Full capture-group inspection with byte offsets
- ✅ Clear invalid-regex reporting
- ✅ Multi-line test input
- ✅ Zero configuration, single binary
