import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import PromptDialog from './PromptDialog';

type Entry = { name: string; kind?: string; is_dir: boolean; size: number };
type Props = { profile: string; onClose: () => void };
type DialogState = { kind: 'mkdir' } | { kind: 'delete'; entry: Entry };

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

type RowProps = {
  entry: Entry;
  remote: boolean;
  busy: boolean;
  onOpen: (entry: Entry) => void;
  onTransfer: (entry: Entry) => void;
  onDelete?: (entry: Entry) => void;
};

function EntryRow({ entry, remote, busy, onOpen, onTransfer, onDelete }: RowProps) {
  const action = remote ? 'Descargar' : 'Subir';
  return (
    <li>
      <div className="pane-item">
        <button
          type="button"
          className="pane-entry"
          disabled={busy}
          aria-label={
            entry.is_dir ? `Abrir carpeta ${entry.name}` : `${action} ${entry.name}, ${fmtSize(entry.size)}`
          }
          onClick={() => (entry.is_dir ? onOpen(entry) : onTransfer(entry))}
        >
          <span className={`pane-icon ${entry.is_dir ? 'dir' : 'file'}`} aria-hidden="true">
            {entry.is_dir ? '▸' : ''}
          </span>
          <span className="pane-name">{entry.name}</span>
          <span className="pane-size">{entry.is_dir ? '' : fmtSize(entry.size)}</span>
        </button>
        {!entry.is_dir && (
          <button
            type="button"
            className="icon-btn small"
            disabled={busy}
            aria-label={`${action} ${entry.name}`}
            title={action}
            onClick={() => onTransfer(entry)}
          >
            {remote ? '↓' : '↑'}
          </button>
        )}
        {remote && onDelete && (
          <button
            type="button"
            className="icon-btn small danger"
            disabled={busy}
            aria-label={`Borrar ${entry.name}`}
            title="Borrar"
            onClick={() => onDelete(entry)}
          >
            ✕
          </button>
        )}
      </div>
    </li>
  );
}

export default function SftpPanel({ profile, onClose }: Props) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [remotePath, setRemotePath] = useState('.');
  const [remoteEntries, setRemoteEntries] = useState<Entry[]>([]);
  const [localPath, setLocalPath] = useState('');
  const [localEntries, setLocalEntries] = useState<Entry[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [dialog, setDialog] = useState<DialogState | null>(null);
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

  const base = (path: string) => path.replace(/\/$/, '');

  const openRemoteDir = async (name: string) => {
    if (!sessionId) return;
    setBusy(true);
    await refreshRemote(sessionId, `${base(remotePath)}/${name}`);
    setBusy(false);
  };

  const goRemoteUp = async () => {
    if (!sessionId) return;
    const parent = remotePath.split('/').filter(Boolean).slice(0, -1).join('/');
    await refreshRemote(sessionId, parent || '.');
  };

  const openLocalDir = async (name: string) => {
    setBusy(true);
    await refreshLocal(`${base(localPath)}/${name}`);
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
      () =>
        invoke('sftp_download', {
          id: sessionId,
          remote: `${base(remotePath)}/${entry.name}`,
          local: `${base(localPath)}/${entry.name}`,
        }),
      `Descargado ${entry.name}`,
    );

  const upload = (entry: Entry) =>
    sessionId &&
    run(
      () =>
        invoke('sftp_upload', {
          id: sessionId,
          local: `${base(localPath)}/${entry.name}`,
          remote: `${base(remotePath)}/${entry.name}`,
        }),
      `Subido ${entry.name}`,
    );

  const removeRemote = (entry: Entry) => {
    if (!sessionId) return;
    void run(
      () =>
        invoke(entry.is_dir ? 'sftp_rm_dir' : 'sftp_rm_file', {
          id: sessionId,
          path: `${base(remotePath)}/${entry.name}`,
        }),
      `Borrado ${entry.name}`,
    );
  };

  return (
    <div className="sftp-panel">
      <div className="sftp-header">
        <span className="terminal-dot" aria-hidden="true" />
        <span className="terminal-title">SFTP · {profile}</span>
        <button
          type="button"
          className="terminal-close"
          aria-label={`Cerrar SFTP de ${profile}`}
          onClick={close}
        >
          ✕
        </button>
      </div>
      {message && (
        <div className="pane-message" role="status">
          <span>{message}</span>
          <button type="button" className="btn ghost small" onClick={() => setMessage(null)}>
            cerrar
          </button>
        </div>
      )}
      <div className="pane-grid">
        <div className="pane">
          <div className="pane-header">
            <button type="button" className="btn ghost small" onClick={goLocalUp} disabled={busy} aria-label="Subir al directorio local anterior">
              ↑
            </button>
            <input value={localPath} readOnly className="pane-path" aria-label="Ruta local" />
          </div>
          <ul className="pane-list">
            {localEntries.map((entry) => (
              <EntryRow
                key={entry.name}
                entry={entry}
                remote={false}
                busy={busy}
                onOpen={(item) => void openLocalDir(item.name)}
                onTransfer={upload}
              />
            ))}
          </ul>
          <div className="pane-footer muted small">
            Clic en carpeta para abrir · ↑ sube el archivo
          </div>
        </div>

        <div className="pane">
          <div className="pane-header">
            <button type="button" className="btn ghost small" onClick={goRemoteUp} disabled={busy} aria-label="Subir al directorio remoto anterior">
              ↑
            </button>
            <input value={remotePath} readOnly className="pane-path" aria-label="Ruta remota" />
            <button
              type="button"
              className="btn ghost small"
              onClick={() => setDialog({ kind: 'mkdir' })}
              disabled={busy || !sessionId}
            >
              + carpeta
            </button>
          </div>
          <ul className="pane-list">
            {remoteEntries.map((entry) => (
              <EntryRow
                key={entry.name}
                entry={entry}
                remote
                busy={busy}
                onOpen={(item) => void openRemoteDir(item.name)}
                onTransfer={download}
                onDelete={(item) => setDialog({ kind: 'delete', entry: item })}
              />
            ))}
          </ul>
          <div className="pane-footer muted small">
            Clic en carpeta para abrir · clic en archivo para descargar
          </div>
        </div>
      </div>

      {dialog?.kind === 'mkdir' && (
        <PromptDialog
          title="Nueva carpeta remota"
          label="Nombre del directorio"
          confirmLabel="Crear"
          requireValue
          onCancel={() => setDialog(null)}
          onConfirm={(value) => {
            setDialog(null);
            if (!sessionId || !value) return;
            void run(
              () => invoke('sftp_mkdir', { id: sessionId, path: `${base(remotePath)}/${value}` }),
              `Creado ${value}`,
            );
          }}
        />
      )}
      {dialog?.kind === 'delete' && (
        <PromptDialog
          title={`¿Borrar ${dialog.entry.name}?`}
          description={
            dialog.entry.is_dir ? 'Se eliminará la carpeta y todo su contenido.' : undefined
          }
          confirmLabel="Borrar"
          danger
          onCancel={() => setDialog(null)}
          onConfirm={() => {
            const entry = dialog.entry;
            setDialog(null);
            removeRemote(entry);
          }}
        />
      )}
    </div>
  );
}
