# sshcli

Cliente SSH de escritorio multiplataforma para administrar conexiones, terminales,
transferencias SFTP y túneles desde una única aplicación.

## Funcionalidades

- Perfiles SSH con host, puerto, usuario, grupo, etiquetas y favoritos.
- Conexión rápida mediante `usuario@host[:puerto]`.
- Terminales SSH con streaming bidireccional, reconexión y ajuste de tamaño.
- Terminal local mediante PTY nativo.
- Pestañas y paneles redimensionables con splits anidados, grupos y drag-and-drop.
- Transferencia y navegación de archivos remotos mediante SFTP.
- Explorador remoto opcional integrado en las sesiones SSH.
- Gestión de túneles y port forwarding.
- Importación y exportación de perfiles en formato TOML.
- Almacenamiento de credenciales mediante el keyring del sistema.
- Telemetría opcional para sesiones SSH.

## Arquitectura

```
crates/core/   sshcli-core: perfiles, credenciales, SSH, SFTP y forwarding
crates/app/    sshcli-app: servicios de aplicación (managers) sin UI
crates/native/ sshcli-native: interfaz nativa (eframe/egui + egui_dock)
```

La lógica de red, perfiles y credenciales vive en `sshcli-core` y no depende de
la interfaz gráfica. `sshcli-app` expone los managers (sesiones, SFTP, túneles,
telemetría, perfiles) sin ningún acoplamiento a un framework de UI. `sshcli-native`
es el binario de escritorio: una interfaz 100 % nativa basada en `eframe`/`egui`, un
emulador de terminal `alacritty_terminal` sobre `portable-pty`, y `egui_dock` para
pestañas y splits. No usa Tauri, WebView, Node ni React.

## Requisitos

Todos los sistemas requieren:

- Rust stable.

### Linux

En Debian/Ubuntu, para el stack de ventanas nativo (winit/glutin):

```bash
sudo apt update
sudo apt install \
  libxcb1-dev \
  libxkbcommon-dev \
  libwayland-dev \
  libx11-dev \
  libgl1-mesa-dev \
  libxrandr-dev \
  libxi-dev \
  libxcursor-dev \
  libssl-dev
```

### Windows

- Visual Studio Build Tools.
- Workload `Desktop development with C++`.
- Windows SDK.
- Rust target `stable-x86_64-pc-windows-msvc`.

### macOS

- Xcode Command Line Tools.
- Rust stable.

## Instalación Y Desarrollo

Compilar y ejecutar la aplicación nativa:

```bash
cargo run -p sshcli-native
```

## Tests

```bash
cargo test --workspace
```

## Releases

Los releases usan Conventional Commits y `git-cliff`. Inspeccionar la siguiente
versión y sus notas sin modificar el repositorio:

```bash
bin/release --dry
```

Crear el commit de versión, regenerar `CHANGELOG.md` y crear el tag local:

```bash
bin/release
```

Publicar el release después de revisarlo:

```bash
git push origin HEAD --follow-tags
```

## Compilación De Escritorio

El build nativo genera un único binario autónomo (`sshcli-native`) sin
dependencia de Node ni WebView:

```bash
cargo build -p sshcli-native --release
```

### Linux

El binario se genera en `target/release/sshcli-native`.

### Windows

Ejecutar desde un entorno Windows con las herramientas indicadas arriba:

```powershell
rustup default stable-x86_64-pc-windows-msvc
cargo build -p sshcli-native --release
```

El ejecutable se genera en `target\release\sshcli-native.exe`.

### macOS

```bash
cargo build -p sshcli-native --release
```

El binario se genera en `target/release/sshcli-native`.

## Configuración Y Datos

- Los perfiles se gestionan desde la aplicación y se pueden importar/exportar
  como TOML.
- Las contraseñas y secretos se almacenan en el keyring del sistema.
- En Linux, la sesión de escritorio debe tener disponible un servicio Secret
  Service, como `gnome-keyring` o KDE Wallet.
- Las preferencias de interfaz se almacenan en un `prefs.json` dentro del
  directorio de configuración de la plataforma.

## Licencia

MIT
