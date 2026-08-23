import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
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
    if (containerRef.current) {
      term.open(containerRef.current);
    }
    terminalRef.current = term;
    fitRef.current = fit;

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

  return (
    <div className="terminal-tab">
      <div className="terminal-body" ref={containerRef} />
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
