import { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Prefs } from './prefs';
import '@xterm/xterm/css/xterm.css';

const OSC7_PREFIX = [0x1b, 0x5d, 0x37, 0x3b];

function createOsc7Parser(onCwd: (path: string) => void) {
  let prefixMatch = 0;
  let scanning = false;
  let stNext = false;
  let payload: number[] = [];
  let pending: number[] = [];
  const out: number[] = [];

  const finish = () => {
    const url = String.fromCharCode(...payload);
    payload = [];
    scanning = false;
    stNext = false;
    try {
      const parsed = new URL(url);
      if (parsed.protocol === 'file:' && parsed.pathname.startsWith('/')) {
        onCwd(decodeURIComponent(parsed.pathname));
      }
    } catch {
      /* secuencia malformada: ignorar */
    }
  };

  return (chunk: Uint8Array): Uint8Array => {
    out.length = 0;
    for (let i = 0; i < chunk.length; i++) {
      const byte = chunk[i];
      if (scanning) {
        if (stNext) {
          stNext = false;
          if (byte === 0x5c) {
            finish();
          } else {
            payload.push(0x1b, byte);
          }
          continue;
        }
        if (byte === 0x07) {
          finish();
          continue;
        }
        if (byte === 0x1b) {
          stNext = true;
          continue;
        }
        payload.push(byte);
        if (payload.length > 4096) {
          payload = [];
          scanning = false;
        }
        continue;
      }
      if (byte === OSC7_PREFIX[prefixMatch]) {
        pending.push(byte);
        prefixMatch += 1;
        if (prefixMatch === OSC7_PREFIX.length) {
          prefixMatch = 0;
          pending = [];
          scanning = true;
          payload = [];
        }
        continue;
      }
      out.push(...pending);
      pending = [];
      if (byte === 0x1b) {
        pending.push(byte);
        prefixMatch = 1;
      } else {
        prefixMatch = 0;
        out.push(byte);
      }
    }
    return new Uint8Array(out);
  };
}

type Props = {
  sessionId: string;
  connected: boolean;
  visible: boolean;
  prefs: Prefs;
  transport?: 'ssh' | 'local';
  onClose: (sessionId: string) => void;
  onReconnect: (sessionId: string) => void;
  onCwd?: (sessionId: string, path: string) => void;
  onError: (message: string) => void;
};

const focusRegistry = new Map<string, () => void>();

export function focusTerminal(sessionId: string) {
  const focus = focusRegistry.get(sessionId);
  if (!focus) return;
  focus();
  requestAnimationFrame(() => {
    focusRegistry.get(sessionId)?.();
  });
}

export default function TerminalTab({
  sessionId,
  connected,
  visible,
  prefs,
  transport = 'ssh',
  onClose,
  onReconnect,
  onCwd,
  onError,
}: Props) {
  const writeCommand = transport === 'local' ? 'local_write' : 'ssh_write';
  const resizeCommand = transport === 'local' ? 'local_resize' : 'ssh_resize';
  const containerRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const searchRef = useRef<SearchAddon | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  const onCwdRef = useRef(onCwd);
  onCwdRef.current = onCwd;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;
  const prefsRef = useRef(prefs);
  prefsRef.current = prefs;

  useEffect(() => {
    const term = new Terminal({
      cursorBlink: prefs.cursorBlink,
      cursorStyle: prefs.cursorStyle,
      fontFamily: prefs.fontFamily,
      fontSize: prefs.fontSize,
      lineHeight: prefs.lineHeight,
      scrollback: prefs.scrollback,
      theme: {
        background: '#090f15',
        foreground: '#dee2ec',
        cursor: '#56fd93',
        cursorAccent: '#00210c',
        selectionBackground: '#2e5d43',
        black: '#0f141a',
        red: '#ffb4ab',
        green: '#56fd93',
        yellow: '#ffd166',
        blue: '#a2c9ff',
        magenta: '#c678dd',
        cyan: '#56d6dd',
        white: '#dee2ec',
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    const search = new SearchAddon();
    term.loadAddon(search);
    if (containerRef.current) {
      term.open(containerRef.current);
    }
    terminalRef.current = term;
    fitRef.current = fit;
    searchRef.current = search;
    focusRegistry.set(sessionId, () => term.focus());

    let disposed = false;

    let lastCwd = '';
    const stripOsc7 = createOsc7Parser((path) => {
      if (path !== lastCwd) {
        lastCwd = path;
        onCwdRef.current?.(sessionId, path);
      }
    });

    const resize = () => {
      if (disposed) return;
      if (!containerRef.current || containerRef.current.offsetParent === null) return;
      fit.fit();
      const { cols, rows } = term;
      if (cols > 0 && rows > 0) {
        invoke(resizeCommand, { id: sessionId, columns: cols, rows }).catch(() => undefined);
      }
    };

    const observer = new ResizeObserver(resize);
    if (containerRef.current) observer.observe(containerRef.current);
    window.addEventListener('resize', resize);

    let writeFailed = false;
    const dataListener = term.onData((data) => {
      const bytes = new TextEncoder().encode(data);
      let binary = '';
      for (const byte of bytes) binary += String.fromCharCode(byte);
      invoke(writeCommand, { id: sessionId, data: btoa(binary) }).catch((reason) => {
        if (writeFailed) return;
        writeFailed = true;
        onErrorRef.current(`No se pudo escribir en la terminal: ${String(reason)}`);
      });
    });

    const selectionListener = term.onSelectionChange(() => {
      if (!prefsRef.current.copyOnSelect) return;
      const selection = term.getSelection();
      if (selection) {
        navigator.clipboard.writeText(selection).catch(() => undefined);
      }
    });

    const onContextMenu = (event: MouseEvent) => {
      if (!prefsRef.current.rightClickPaste) return;
      event.preventDefault();
      navigator.clipboard
        .readText()
        .then((text) => {
          if (text) term.paste(text);
        })
        .catch(() => undefined);
    };
    containerRef.current?.addEventListener('contextmenu', onContextMenu);

    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === 'f') {
        event.preventDefault();
        setSearchOpen(true);
        requestAnimationFrame(() => searchInputRef.current?.focus());
      }
    };
    containerRef.current?.addEventListener('keydown', onKeyDown);

    let unlistenData: (() => void) | undefined;
    let unlistenStatus: (() => void) | undefined;

    (async () => {
      unlistenData = await listen<{ id: string; data: string }>('ssh-data', (event) => {
        if (event.payload.id !== sessionId || disposed) return;
        const binary = atob(event.payload.data);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
        term.write(stripOsc7(bytes));
      });
      unlistenStatus = await listen<{ id: string; status: string }>('ssh-status', (event) => {
        if (event.payload.id !== sessionId || disposed) return;
        if (event.payload.status === 'closed') {
          term.write('\r\n\x1b[2m[sshcli] conexión cerrada\x1b[0m\r\n');
          term.blur();
        }
      });
      if (transport === 'local') {
        await invoke('local_shell_ready', { id: sessionId });
      }
      requestAnimationFrame(() => term.focus());
    })().catch((reason) => onErrorRef.current(`No se pudo preparar la terminal: ${String(reason)}`));

    return () => {
      disposed = true;
      focusRegistry.delete(sessionId);
      dataListener.dispose();
      selectionListener.dispose();
      containerRef.current?.removeEventListener('contextmenu', onContextMenu);
      containerRef.current?.removeEventListener('keydown', onKeyDown);
      unlistenData?.();
      unlistenStatus?.();
      observer.disconnect();
      window.removeEventListener('resize', resize);
      term.dispose();
    };
  }, [sessionId]);

  useEffect(() => {
    const term = terminalRef.current;
    if (!term) return;
    term.options.fontFamily = prefs.fontFamily;
    term.options.fontSize = prefs.fontSize;
    term.options.lineHeight = prefs.lineHeight;
    term.options.scrollback = prefs.scrollback;
    term.options.cursorStyle = prefs.cursorStyle;
    term.options.cursorBlink = prefs.cursorBlink;
  }, [prefs]);

  useEffect(() => {
    if (!visible) return;
    const frame = requestAnimationFrame(() => {
      const fit = fitRef.current;
      if (!fit || !containerRef.current || containerRef.current.offsetParent === null) return;
      fit.fit();
      terminalRef.current?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [visible]);

  const runSearch = (backwards: boolean) => {
    const search = searchRef.current;
    if (!search || !searchQuery) return;
    if (backwards) search.findPrevious(searchQuery);
    else search.findNext(searchQuery);
  };

  const closeSearch = () => {
    searchRef.current?.clearDecorations();
    setSearchOpen(false);
    terminalRef.current?.focus();
  };

  return (
    <div className="terminal-tab">
      <div className="terminal-body" ref={containerRef} />
      {searchOpen && (
        <div className="term-search" role="search" aria-label="Buscar en terminal">
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            placeholder="Buscar…"
            aria-label="Texto a buscar"
            onChange={(event) => setSearchQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                runSearch(event.shiftKey);
              } else if (event.key === 'Escape') {
                event.preventDefault();
                closeSearch();
              }
            }}
          />
          <button
            type="button"
            className="term-search-btn"
            aria-label="Resultado anterior"
            title="Anterior (Shift+Enter)"
            onClick={() => runSearch(true)}
          >
            ‹
          </button>
          <button
            type="button"
            className="term-search-btn"
            aria-label="Resultado siguiente"
            title="Siguiente (Enter)"
            onClick={() => runSearch(false)}
          >
            ›
          </button>
          <button
            type="button"
            className="term-search-btn"
            aria-label="Cerrar búsqueda"
            title="Cerrar (Esc)"
            onClick={closeSearch}
          >
            ✕
          </button>
        </div>
      )}
      {!connected && (
        <div className="terminal-dead" role="status">
          <span className="terminal-dead-text">Sesión cerrada</span>
          <button
            type="button"
            className="btn small primary"
            onClick={() => onReconnect(sessionId)}
          >
            Reconectar
          </button>
        </div>
      )}
    </div>
  );
}
