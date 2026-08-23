import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import '@xterm/xterm/css/xterm.css';

type Props = {
  sessionId: string;
  profile: string;
  connected: boolean;
  visible: boolean;
  onClose: (sessionId: string) => void;
  onReconnect: (sessionId: string) => void;
};

export default function TerminalTab({
  sessionId,
  profile,
  connected,
  visible,
  onClose,
  onReconnect,
}: Props) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    const term = new Terminal({
      cursorBlink: true,
      fontFamily: 'Menlo, Monaco, "Cascadia Mono", "Fira Code", monospace',
      fontSize: 13,
      theme: {
        background: '#0c0f16',
        foreground: '#e8ecf4',
        cursor: '#5dd3ff',
        selectionBackground: '#24507a',
        black: '#0c0f16',
        red: '#ff6b6b',
        green: '#7dff9e',
        yellow: '#ffd166',
        blue: '#5dd3ff',
        magenta: '#c678dd',
        cyan: '#56d6dd',
        white: '#e8ecf4',
      },
      scrollback: 5000,
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
      unlistenData?.();
      unlistenStatus?.();
      observer.disconnect();
      window.removeEventListener('resize', resize);
      term.dispose();
    };
  }, [sessionId]);

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
      <div className="terminal-header">
        <span className={`terminal-dot ${connected ? '' : 'dead'}`} />
        <span className="terminal-title">
          {profile}
          {!connected && ' (desconectado)'}
        </span>
        {!connected && (
          <button className="btn small primary" onClick={() => onReconnect(sessionId)}>
            Reconectar
          </button>
        )}
        <button
          className="terminal-close"
          title="Cerrar pestaña"
          onClick={() => {
            invoke('ssh_close', { id: sessionId }).catch(() => undefined);
            onClose(sessionId);
          }}
        >
          ✕
        </button>
      </div>
      <div className="terminal-body" ref={containerRef} />
    </div>
  );
}
