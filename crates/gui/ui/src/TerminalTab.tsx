import { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Prefs } from './prefs';
import '@xterm/xterm/css/xterm.css';

type Props = {
  sessionId: string;
  connected: boolean;
  visible: boolean;
  prefs: Prefs;
  onClose: (sessionId: string) => void;
  onReconnect: (sessionId: string) => void;
};

export default function TerminalTab({
  sessionId,
  connected,
  visible,
  prefs,
  onClose,
  onReconnect,
}: Props) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const searchRef = useRef<SearchAddon | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
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

    let disposed = false;

    const resize = () => {
      if (disposed) return;
      if (!containerRef.current || containerRef.current.offsetParent === null) return;
      fit.fit();
      const { cols, rows } = term;
      if (cols > 0 && rows > 0) {
        invoke('ssh_resize', { id: sessionId, columns: cols, rows }).catch(() => undefined);
      }
    };

    const observer = new ResizeObserver(resize);
    if (containerRef.current) observer.observe(containerRef.current);
    window.addEventListener('resize', resize);

    const dataListener = term.onData((data) => {
      const bytes = new TextEncoder().encode(data);
      let binary = '';
      for (const byte of bytes) binary += String.fromCharCode(byte);
      invoke('ssh_write', { id: sessionId, data: btoa(binary) }).catch((reason) =>
        console.error('ssh_write', reason),
      );
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
        term.write(bytes);
      });
      unlistenStatus = await listen<{ id: string; status: string }>('ssh-status', (event) => {
        if (event.payload.id !== sessionId || disposed) return;
        if (event.payload.status === 'closed') {
          term.write('\r\n\x1b[2m[sshcli] conexión cerrada\x1b[0m\r\n');
          term.blur();
        }
      });
    })();

    return () => {
      disposed = true;
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
