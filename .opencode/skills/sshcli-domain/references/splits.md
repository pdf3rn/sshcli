# Tabs and nested split panes

Represent workspace topology as data, not as accidental widget nesting only.

Conceptual model:

```text
WorkspaceNode =
  Pane(session_id)
  | Split {
      orientation,
      ratio,
      first: WorkspaceNode,
      second: WorkspaceNode
    }
```

Preserve:
- orientation
- nested topology
- split ratio
- min pane dimensions
- focused pane
- active tab
- close semantics
- drag/move semantics if present
- persistence/restoration if present

The Slint view should render the model; Rust can own durable workspace/session state when appropriate.
