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
crates/gui/    sshcli-gui: binario Tauri, comandos IPC y PTY local
crates/gui/ui/ React, xterm.js, Dockview y componentes de la interfaz
```

La lógica de red, perfiles y credenciales vive en `sshcli-core` y no depende de
la interfaz gráfica. `sshcli-gui` expone esa funcionalidad al frontend mediante
comandos y eventos IPC de Tauri.

## Requisitos

Todos los sistemas requieren:

- Rust stable.
- Node.js 20 o superior.
- npm.

### Linux

En Debian/Ubuntu:

```bash
sudo apt update
sudo apt install \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libxdo-dev \
  libssl-dev
```

### Windows

- Visual Studio Build Tools.
- Workload `Desktop development with C++`.
- Windows SDK.
- Rust target `stable-x86_64-pc-windows-msvc`.
- WebView2 Runtime.

### macOS

- Xcode Command Line Tools.
- Rust stable.
- Node.js 20 o superior.

## Instalación Y Desarrollo

Instalar las dependencias del frontend:

```bash
npm install
```

Ejecutar la aplicación en modo desarrollo:

```bash
npm run dev
```

Ejecutar solo el servidor web del frontend:

```bash
npm run web:dev
```

Compilar solo el frontend:

```bash
npm run build:web
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

El build completo compila primero el frontend y después genera los artefactos
de Tauri:

```bash
npm run build
```

### Linux

La configuración actual genera paquetes `deb` y `AppImage` en:

```text
target/release/bundle/
```

### Windows

Ejecutar desde un entorno Windows con las herramientas indicadas arriba:

```powershell
rustup default stable-x86_64-pc-windows-msvc
npm install
npm run build
```

El instalador NSIS se genera en:

```text
target\release\bundle\nsis\
```

El ejecutable sin instalador se encuentra en:

```text
target\release\sshcli-gui.exe
```

### macOS

```bash
npm install
npm run build
```

El paquete `.dmg` se genera en `target/release/bundle/dmg/`.

## Configuración Y Datos

- Los perfiles se gestionan desde la aplicación y se pueden importar/exportar
  como TOML.
- Las contraseñas y secretos se almacenan en el keyring del sistema.
- Las preferencias de interfaz se almacenan localmente en el perfil de usuario.
- La configuración de empaquetado está en `crates/gui/tauri.conf.json`.

## Licencia

MIT
