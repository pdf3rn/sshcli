# Terminal migration checklist

Inventory the source terminal implementation.

If using xterm.js or similar, separate:
- parser/emulator
- screen buffer
- rendering
- keyboard input encoding
- selection
- clipboard
- scrollback
- cursor
- resize
- terminal modes
- mouse support
- colors/attributes
- links/search
- addons

Target design should explicitly choose:
1. Rust terminal emulation/parsing approach.
2. Terminal screen/state representation.
3. Efficient Slint/native rendering surface.
4. Input/resize channel to PTY/SSH.
5. batching/coalescing strategy.
6. test strategy.

Do not select a Rust terminal-emulator crate without checking its current maintenance, license, API compatibility, and feature coverage.

Behavior tests should include:
- resize
- ANSI colors/styles
- cursor movement
- alternate screen
- large output
- scrollback
- Unicode/wide characters if supported
- Ctrl/Alt/function-key sequences
- reconnect/session close behavior
