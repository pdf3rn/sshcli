import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Entry = { name: string; kind?: string; is_dir: boolean; size: number };
type Props = { profile: string; onClose: () => void };

function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (const next of units.slice(1)) {
    if (value < 1024) break;
    value /= 1024;
    unit = next;
  }
  return `${value.toFixed(1)} ${unit}`;
}

export default function SftpPanel({ profile, onClose }: Props) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [remotePath, setRemotePath] = useState('.');
  const [remoteEntries, setRemoteEntries] = useState<Entry[]>([]);
  const [localPath, setLocalPath] = useState('');
  const [localEntries, setLocalEntries] = useState<Entry[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const mounted = useRef(true);

  const refreshRemote = useCallback(
    async (id: string, path: string) => {
      try {
        const entries = await invoke<Entry[]>('sftp_list_dir', { id, path });
        if (mounted.current) {
          setRemoteEntries(entries);
          setRemotePath(path);
        }
      } catch (reason) {
        if (mounted.current) setMessage(String(reason));
      }
    },
    [],
  );

  const refreshLocal = useCallback(async (path: string) => {
    try {
      const entries = await invoke<Entry[]>('list_local_dir', { path });
      if (mounted.current) {
        setLocalEntries(entries);
        setLocalPath(path);
      }
    } catch (reason) {
      if (mounted.current) setMessage(String(reason));
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    (async () => {
      try {
        const id = await invoke<string>('sftp_connect', { profileName: profile });
        if (!mounted.current) {
          invoke('sftp_close', { id }).catch(() => undefined);
          return;
        }
        setSessionId(id);
        const home = await invoke<string>('local_home');
        await Promise.all([
          refreshRemote(id, '.'),
          refreshLocal(home).then(() => setLocalPath(home)),
        ]);
      } catch (reason) {
        if (mounted.current) setMessage(String(reason));
      }
    })();
    return () => {
      mounted.current = false;
    };
  }, [profile, refreshRemote, refreshLocal]);

  const close = () => {
    if (sessionId) invoke('sftp_close', { id: sessionId }).catch(() => undefined);
    onClose();
  };

  const openRemoteDir = async (name: string) => {
    if (!sessionId) return;
    setBusy(true);
    await refreshRemote(sessionId, `${remotePath.replace(/\/$/, '')}/${name}`);
    setBusy(false);
  };

  const goRemoteUp = async () => {
    if (!sessionId) return;
    const parent = remotePath.split('/').filter(Boolean).slice(0, -1).join('/');
    await refreshRemote(sessionId, parent || '.');
  };

  const openLocalDir = async (name: string) => {
    const next = `${localPath.replace(/\/$/, '')}/${name}`;
    setBusy(true);
    await refreshLocal(next);
    setBusy(false);
  };

  const goLocalUp = async () => {
    const parent = localPath.split('/').filter(Boolean).slice(0, -1).join('/');
    await refreshLocal(parent || '/');
  };

  const run = async (fn: () => Promise<unknown>, okMessage: string) => {
    setBusy(true);
    try {
      await fn();
      if (sessionId) await refreshRemote(sessionId, remotePath);
      setMessage(okMessage);
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const download = (entry: Entry) =>
    sessionId &&
    run(
      () => invoke('sftp_download', { id: sessionId, remote: `${remotePath.replace(/\/$/, '')}/${entry.name}`, local: `${localPath.replace(/\/$/, '')}/${entry.name}` }),
      `Descargado ${entry.name}`,
    );

  const upload = (entry: Entry) =>
    sessionId &&
    run(
      () => invoke('sftp_upload', { id: sessionId, local: `${localPath.replace(/\/$/, '')}/${entry.name}`, remote: `${remotePath.replace(/\/$/, '')}/${entry.name}` }),
      `Subido ${entry.name}`,
    );

  const makeRemoteDir = () => {
    const name = prompt('Nombre del directorio remoto:');
    if (name && sessionId) {
      run(
        () => invoke('sftp_mkdir', { id: sessionId, path: `${remotePath.replace(/\/$/, '')}/${name}` }),
        `Creado ${name}`,
      );
    }
  };

  const removeRemote = (entry: Entry) => {
    if (!sessionId || !confirm(`¿Borrar ${entry.name}?`)) return;
    const path = `${remotePath.replace(/\/$/, '')}/${entry.name}`;
    run(
      () => invoke(entry.is_dir ? 'sftp_rm_dir' : 'sftp_rm_file', { id: sessionId, path }),
      `Borrado ${entry.name}`,
    );
  };

  const pane = (
    entries: Entry[],
    path: string,
    goUp: () => void,
    onDir: (name: string) => void,
    onFile: (entry: Entry) => void,
    remote: boolean,
  ) => (
    <div className="pane">
      <div className="pane-header">
        <button className="btn ghost small" onClick={goUp} disabled={busy}>
          ↑
        </button>
        <input value={path} readOnly className="pane-path" />
        {remote && (
          <button className="btn ghost small" onClick={makeRemoteDir} disabled={busy}>
            + carpeta
          </button>
        )}
      </div>
      <ul className="pane-list">
        {entries.map((entry) => (
          <li
            key={entry.name}
            className="pane-item"
            onClick={() => (entry.is_dir ? onDir(entry.name) : onFile(entry))}
          >
            <span className={`pane-icon ${entry.is_dir ? 'dir' : 'file'}`}>
              {entry.is_dir ? '▸' : ' '}
            </span>
            <span className="pane-name">{entry.name}</span>
            <span className="pane-size">{entry.is_dir ? '' : fmtSize(entry.size)}</span>
          </li>
        ))}
      </ul>
      <div className="pane-footer muted small">
        {remote ? 'doble clic: abrir · clic: descargar' : 'clic en archivo: subir'}
      </div>
    </div>
  );

  return (
    <div className="sftp-panel">
      <div className="sftp-header">
        <span className="terminal-dot" />
        <span className="terminal-title">SFTP · {profile}</span>
        <button className="terminal-close" onClick={close}>
          ✕
        </button>
      </div>
      {message && (
        <div className="pane-message">
          <span>{message}</span>
          <button className="btn ghost small" onClick={() => setMessage(null)}>
            cerrar
          </button>
        </div>
      )}
      <div className="pane-grid">
        {pane(localEntries, localPath, goLocalUp, openLocalDir, upload, false)}
        {pane(remoteEntries, remotePath, goRemoteUp, openRemoteDir, download, true)}
      </div>
    </div>
  );
}
