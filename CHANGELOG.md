# Changelog


## [1.5.0] - 2026-09-03


### Added

- Add persistent light and dark themes


## [1.4.0] - 2026-09-03


### Added

- Add sftp rename and multi-select transfers


### Fixed

- Sync lockfile version after v1.3.0


## [1.3.0] - 2026-09-03


### Added

- Sort and duplicate connections

- Validate profile fields before save

- Show connection progress per profile

- Test connection before saving profile


## [1.2.0] - 2026-09-03


### Added

- Custom context menu in connections and terminal copy shortcut

- Add keyboard shortcut reference

- Confirm closing active sessions

- Add terminal clear shortcut


### Fixed

- Sync lockfile release version


## [1.1.1] - 2026-09-02


### Fixed

- Stabilize local terminal startup on Windows


## [1.1.0] - 2026-09-02


### Added

- Interactive host key confirmation on unknown or changed key


### Fixed

- Use plain keyring entry instead of custom target


## [1.0.1] - 2026-09-02


### Fixed

- Normalize release versions

- Configure native credential stores

- Support tab splits in webviews


## [1.0.0] - 2026-09-02


### Added

- Add phase one tui foundation

- Add interactive ssh sessions

- Add secure connection profiles

- Add sftp file operations

- Add sftp browser and transfers

- Add local port forwarding

- Manage forwarding from tui

- Create profiles from tui

- Select existing ssh keys

- Show profile form as modal

- Render ssh sessions in workspace

- Polish tui visual design

- Usable ssh terminal and delete shortcut

- Redesign profile creation modal

- Live ssh sessions and terminal fidelity

- Migrate to tauri gui with core workspace

- Manage profiles from gui (crud, keyring, key selector)

- Live ssh sessions with xterm.js tabs

- Dual-pane sftp browser in gui

- Manage local port forwarding in gui

- Mobaxterm-style tabs with persistent terminals, split view and reconnect

- Accessible tabs, sidebar, dialogs and sftp rows with custom prompt dialog (ui)

- Loading skeletons and empty states for sftp panes and sidebar (ui)

- Active tab indicator, shortcut tooltip and refined details view (ui)

- Multi-view shell with global topbar, statusbar and graphite tokens from stitch design (ui)

- Extend profile schema with group, tags and last_used

- Connections view with table, groups, search and quick connect (ui)

- Home view with recent connections and quick actions including toml import (ui)

- Settings view with persisted terminal prefs applied to xterm sessions (ui)

- Opt-in SSH host telemetry panel with per-profile exec session cache

- Icon-based session toolbar and persisted telemetry panel toggle (ui)

- Favorites, terminal search, profile export, instant telemetry and pane height fix

- Restore sftp and tunnels entry points in connections table

- Stitch-style hero with quick ad-hoc connect (user@host)

- Reveal quick-connect password field only when server requires it

- Tab bar plus button with new-connection modal and drag-to-reorder tabs

- Confirm overwrite before download or upload when target file exists

- Optional remote explorer panel with OSC 7 cwd tracking

- Local terminal with native pty and shell autodetection

- Drag tabs into directional drop zones to create or reshape splits

- Focus the terminal when its tab is selected

- Tree-aware split layout with resize dividers

- Migrate session layout to Dockview

- Migrate app icons to Lucide

- Add connection action menu with more options for profiles

- Improve SFTP navigation and connection actions


### Changed

- Design tokens, focus-visible, scrollbars and button states (ui)

- Remove sidebar from session view in favor of connections view (ui)

- Drop terminal header, overlay reconnect state and split pane labels (ui)

- Remove session toolbar, move split and telemetry toggles to statusbar (ui)

- PaneNode tree model + recursive render for splits


### Documentation

- Update README with enhanced features and installation instructions for sshcli


### Fixed

- Keep session view mounted when switching views to preserve terminal buffer

- Hide quick-connect password field when target is cleared

- Prevent duplicate connections from rapid double clicks

- Navigable sftp with editable paths, file icons, progress bar and no header

- Transfer files only via explicit buttons, not by clicking them

- Remove duplicated transfer button in sftp entry rows

- Initiate tab drag on webkitgtk by setting dataTransfer data

- Spawn local shell by absolute path with explicit -l login flag

- Local tab label and non-login interactive shell for colors

- Keep drop zones visible during tab drag by removing dragleave cleanup

- Apply vertical flex direction for column splits

- Resolve drop zones by coordinates and skip split drag for active tab

- Constrain pane height so column splits stay in view

- Persistent tab group + flat render + last-focused-member tracking

- Preserve split direction when moving member within group

- Reduce SSH input latency

- Improve profile credential handling

- Harden SSH operations
