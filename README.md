# sshcli

Cliente SSH de escritorio multiplataforma con interfaz gráfica tipo Termius.

## Estructura

```
crates/core/   sshcli-core (lib): config, perfiles, credenciales (keyring), SSH, SFTP, forwarding
crates/gui/    sshcli-gui  (bin):  app de escritorio Tauri 2 + frontend web (React + xterm.js)
```

- La lógica de red y credenciales vive en `sshcli-core` y no depende de ninguna UI.
- `sshcli-gui` consume ese núcleo mediante comandos y eventos de Tauri.

## Requisitos

- Rust (toolchain estable).
- Node.js 20+ y npm (para el frontend).
- Linux: dependencias de Tauri v2
  (`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev`).

## Desarrollo

```bash
npm install          # frontend + tauri-cli
npm run dev          # arranca la GUI en modo desarrollo (ventana + hot reload)
npm run build:web    # solo compila el frontend
cargo test --workspace
```

## Estado actual

- Fase 0: esqueleto Tauri + React + xterm.js; la ventana abre y muestra perfiles vía IPC.
- Fase 1: refactor a workspace; `sshcli-core` reutiliza toda la lógica (SSH/SFTP/forwarding/keyring).

## Próximas fases

2. Gestión de perfiles en la GUI (crear/editar/borrar, selector de claves `~/.ssh`).
3. Sesión SSH en `xterm.js` (streaming bidireccional, resize, pestañas, reconexión).
4. SFTP dual-pane.
5. Port forwarding con lista de túneles.
6. Pulido y empaquetado (deb/rpm/AppImage, dmg, msi) + CI.
