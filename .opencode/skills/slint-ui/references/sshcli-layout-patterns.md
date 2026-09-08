# sshcli Slint layout patterns

These are structural patterns, not a required visual redesign.

## Main desktop shell

A typical sshcli shell should be reasoned about as:

```text
MainWindow
└── VerticalLayout
    ├── Toolbar
    ├── HorizontalLayout
    │   ├── Sidebar
    │   └── Workspace
    │       └── tab/split content
    └── StatusBar
```

The toolbar and status bar normally have bounded/preferred heights.
The sidebar normally has a bounded/preferred width.
The workspace consumes remaining space.

Do not hard-code the full application to one screenshot size.

## Sidebar

```text
Sidebar
└── VerticalLayout
    ├── Search/QuickConnect
    ├── Filter/toolbar
    └── ConnectionList [stretch + scroll]
```

The connection list, not the header controls, should normally absorb vertical expansion.

## Terminal workspace

```text
TerminalWorkspace
└── VerticalLayout
    ├── TabBar [bounded height]
    └── WorkspaceNode [stretch]
```

For nested splits, the view reflects a recursive workspace model rather than flattening every pane into one static layout.

## SFTP

```text
SftpView
└── VerticalLayout
    ├── Path/toolbar
    ├── FileList [stretch]
    └── Transfer/Status area [conditional or bounded]
```

Large file lists require efficient models and scrolling. Do not create one expensive Rust-to-UI transaction per tiny progress update.

## Settings

Prefer a stable desktop form hierarchy:

```text
Settings
└── HorizontalLayout or VerticalLayout
    ├── category navigation
    └── settings content [scroll]
```

Do not turn settings into oversized marketing-style cards unless the current application already uses that language.

## Dialogs

Dialogs should:
- preserve keyboard confirmation/cancel behavior
- have bounded preferred sizes
- scroll internally when content can exceed the window
- keep primary/destructive action semantics clear

## Size rules

Use fixed sizes for elements whose physical role is genuinely fixed/bounded:
- icon size
- toolbar/status height
- minimum splitter handle size
- compact control heights

Use stretch/constraints for:
- terminal surface
- SFTP lists
- connection list
- main workspace
- settings content
