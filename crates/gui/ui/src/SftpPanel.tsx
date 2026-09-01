import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import PromptDialog from './PromptDialog';
import { PASSWORD_REQUIRED } from './adhoc';
import { FileIcon, FolderIcon } from './icons';

const SKELETON_WIDTHS = ['85%', '70%', '92%', '60%', '78%', '88%'];

function PaneSkeletons() {
  return (
    <>
      {SKELETON_WIDTHS.map((width, index) => (
        <li key={index} className="pane-skeleton shimmer" style={{ width }} aria-hidden="true" />
      ))}
    </>
  );
}

type Entry = { name: string; kind?: string; is_dir: boolean; size: number };
type Props = { profile: string };
type DialogState =
  | { kind: 'mkdir' }
  | { kind: 'delete'; entry: Entry }
  | { kind: 'overwrite'; direction: 'down' | 'up'; entry: Entry };
type TransferProgress = {
  name: string;
  direction: 'upload' | 'download';
  transferred: number;
  total: number;
};

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

function parentLocalPath(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '');
  if (!trimmed) return path || '.';

  const separatorIndex = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  if (separatorIndex < 0) return '.';
  if (separatorIndex === 0) return trimmed[0];

  const drive = trimmed.slice(0, separatorIndex);
  if (separatorIndex === 2 && /^[A-Za-z]:$/.test(drive)) {
    return `${drive}${trimmed[separatorIndex]}`;
  }
  return trimmed.slice(0, separatorIndex);
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
        {entry.is_dir ? (
          <button
            type="button"
            className="pane-entry"
            disabled={busy}
            aria-label={`Abrir carpeta ${entry.name}`}
            onClick={() => onOpen(entry)}
          >
            <span className="pane-icon dir" aria-hidden="true">
              <FolderIcon />
            </span>
            <span className="pane-name">{entry.name}</span>
            <span className="pane-size" />
          </button>
        ) : (
          <div className="pane-entry pane-entry-static">
            <span className="pane-icon" aria-hidden="true">
              <FileIcon />
            </span>
            <span className="pane-name">{entry.name}</span>
            <span className="pane-size">{fmtSize(entry.size)}</span>
          </div>
        )}
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

export default function SftpPanel({ profile }: Props) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [remotePath, setRemotePath] = useState('');
  const [remoteDraft, setRemoteDraft] = useState('');
  const [remoteEntries, setRemoteEntries] = useState<Entry[]>([]);
  const [localPath, setLocalPath] = useState('');
  const [localDraft, setLocalDraft] = useState('');
  const [localEntries, setLocalEntries] = useState<Entry[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loadingRemote, setLoadingRemote] = useState(true);
  const [loadingLocal, setLoadingLocal] = useState(true);
  const [progress, setProgress] = useState<TransferProgress | null>(null);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [password, setPassword] = useState<string | null>(null);
  const [passwordPromptOpen, setPasswordPromptOpen] = useState(false);
  const mounted = useRef(true);
  const sessionIdRef = useRef<string | null>(null);

  useEffect(() => {
    setLocalDraft(localPath);
  }, [localPath]);

  useEffect(() => {
    setRemoteDraft(remotePath);
  }, [remotePath]);

  useEffect(() => {
    if (!sessionId) return;
    let disposed = false;
    const promise = listen<TransferProgress & { id: string }>('sftp-progress', (event) => {
      if (disposed || event.payload.id !== sessionId) return;
      const { id: _ignored, ...rest } = event.payload;
      setProgress(rest);
    });
    return () => {
      disposed = true;
      void promise.then((unlisten) => unlisten());
    };
  }, [sessionId]);

  const refreshRemote = useCallback(
    async (id: string, path: string) => {
      setLoadingRemote(true);
      try {
        const entries = await invoke<Entry[]>('sftp_list_dir', { id, path });
        if (mounted.current) {
          setRemoteEntries(entries);
          setRemotePath(path);
        }
      } catch (reason) {
        if (mounted.current) setMessage(String(reason));
      } finally {
        if (mounted.current) setLoadingRemote(false);
      }
    },
    [],
  );

  const refreshLocal = useCallback(async (path: string) => {
    setLoadingLocal(true);
    try {
      const entries = await invoke<Entry[]>('list_local_dir', { path });
      if (mounted.current) {
        setLocalEntries(entries);
        setLocalPath(path);
      }
    } catch (reason) {
      if (mounted.current) setMessage(String(reason));
    } finally {
      if (mounted.current) setLoadingLocal(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    (async () => {
      try {
        const id = await invoke<string>('sftp_connect', { profileName: profile, password });
        if (password) {
          await invoke('save_profile_secret', { name: profile, secret: password }).catch((reason) => {
            if (mounted.current) setMessage(`Conectado, pero no se pudo guardar la contraseña: ${String(reason)}`);
          });
        }
        if (!mounted.current) {
          invoke('sftp_close', { id }).catch(() => undefined);
          return;
        }
        sessionIdRef.current = id;
        setSessionId(id);
        const [home, localHome] = await Promise.all([
          invoke<string>('sftp_pwd', { id }),
          invoke<string>('local_home'),
        ]);
        await Promise.all([refreshRemote(id, home), refreshLocal(localHome)]);
      } catch (reason) {
        if (mounted.current && String(reason).includes(PASSWORD_REQUIRED)) {
          setPasswordPromptOpen(true);
        } else if (mounted.current) {
          setMessage(String(reason));
        }
      }
    })();
    return () => {
      mounted.current = false;
      if (sessionIdRef.current) {
        invoke('sftp_close', { id: sessionIdRef.current }).catch(() => undefined);
        sessionIdRef.current = null;
      }
    };
  }, [profile, password, refreshRemote, refreshLocal]);

  const base = (path: string) => path.replace(/\/$/, '');

  const openRemoteDir = async (name: string) => {
    if (!sessionId || !remotePath) return;
    setBusy(true);
    await refreshRemote(sessionId, `${base(remotePath)}/${name}`);
    setBusy(false);
  };

  const goRemoteUp = async () => {
    if (!sessionId || !remotePath) return;
    const parent = remotePath.split('/').filter(Boolean).slice(0, -1).join('/');
    await refreshRemote(sessionId, parent ? `/${parent}` : '/');
  };

  const submitRemotePath = async () => {
    const target = remoteDraft.trim();
    if (!sessionId || !target) {
      setRemoteDraft(remotePath);
      return;
    }
    setLoadingRemote(true);
    try {
      const entries = await invoke<Entry[]>('sftp_list_dir', { id: sessionId, path: target });
      if (!mounted.current) return;
      setRemoteEntries(entries);
      setRemotePath(target);
    } catch (reason) {
      if (mounted.current) setMessage(String(reason));
      setRemoteDraft(remotePath);
    } finally {
      if (mounted.current) setLoadingRemote(false);
    }
  };

  const submitLocalPath = async () => {
    const target = localDraft.trim();
    if (!target) {
      setLocalDraft(localPath);
      return;
    }
    setLoadingLocal(true);
    try {
      const entries = await invoke<Entry[]>('list_local_dir', { path: target });
      if (!mounted.current) return;
      setLocalEntries(entries);
      setLocalPath(target);
    } catch (reason) {
      if (mounted.current) setMessage(String(reason));
      setLocalDraft(localPath);
    } finally {
      if (mounted.current) setLoadingLocal(false);
    }
  };

  const openLocalDir = async (name: string) => {
    setBusy(true);
    await refreshLocal(`${base(localPath)}/${name}`);
    setBusy(false);
  };

  const goLocalUp = async () => {
    await refreshLocal(parentLocalPath(localPath));
  };

  const run = async (fn: () => Promise<unknown>, okMessage: string) => {
    setBusy(true);
    try {
      await fn();
      if (sessionId) await refreshRemote(sessionId, remotePath);
      setMessage(okMessage);
      setProgress(null);
    } catch (reason) {
      setMessage(String(reason));
      setProgress(null);
    } finally {
      setBusy(false);
    }
  };

  const performDownload = (entry: Entry) =>
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

  const performUpload = (entry: Entry) =>
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

  const requestTransfer = (direction: 'down' | 'up', entry: Entry) => {
    if (!sessionId || entry.is_dir) return;
    void (async () => {
      try {
        const exists =
          direction === 'down'
            ? await invoke<boolean>('local_file_exists', {
                path: `${base(localPath)}/${entry.name}`,
              })
            : await invoke<boolean>('sftp_file_exists', {
                id: sessionId,
                path: `${base(remotePath)}/${entry.name}`,
              });
        if (!mounted.current) return;
        if (exists) {
          setDialog({ kind: 'overwrite', direction, entry });
          return;
        }
      } catch (reason) {
        if (mounted.current) setMessage(String(reason));
        return;
      }
      if (direction === 'down') void performDownload(entry);
      else void performUpload(entry);
    })();
  };

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
            <input
              value={localDraft}
              className="pane-path"
              aria-label="Ruta local"
              spellCheck={false}
              onChange={(event) => setLocalDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void submitLocalPath();
                } else if (event.key === 'Escape') {
                  event.preventDefault();
                  setLocalDraft(localPath);
                }
              }}
            />
          </div>
          <ul className="pane-list" aria-busy={loadingLocal || undefined}>
            {loadingLocal ? (
              <PaneSkeletons />
            ) : localEntries.length === 0 ? (
              <li className="pane-empty muted small" role="status">
                <p>Carpeta vacía</p>
              </li>
            ) : (
              localEntries.map((entry) => (
                <EntryRow
                  key={entry.name}
                  entry={entry}
                  remote={false}
                  busy={busy}
                  onOpen={(item) => void openLocalDir(item.name)}
                  onTransfer={(item) => requestTransfer('up', item)}
                />
              ))
            )}
          </ul>
          <div className="pane-footer muted small">
            Clic en carpeta para abrir · ↑ sube el archivo seleccionado
          </div>
        </div>

        <div className="pane">
          <div className="pane-header">
            <button type="button" className="btn ghost small" onClick={goRemoteUp} disabled={busy} aria-label="Subir al directorio remoto anterior">
              ↑
            </button>
            <input
              value={remoteDraft}
              className="pane-path"
              aria-label="Ruta remota"
              spellCheck={false}
              onChange={(event) => setRemoteDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void submitRemotePath();
                } else if (event.key === 'Escape') {
                  event.preventDefault();
                  setRemoteDraft(remotePath);
                }
              }}
            />
            <button
              type="button"
              className="btn ghost small"
              onClick={() => setDialog({ kind: 'mkdir' })}
              disabled={busy || !sessionId}
            >
              + carpeta
            </button>
          </div>
          <ul className="pane-list" aria-busy={loadingRemote || undefined}>
            {loadingRemote ? (
              <PaneSkeletons />
            ) : remoteEntries.length === 0 ? (
              <li className="pane-empty muted small" role="status">
                <p>Carpeta vacía</p>
              </li>
            ) : (
              remoteEntries.map((entry) => (
                <EntryRow
                  key={entry.name}
                  entry={entry}
                  remote
                  busy={busy}
                  onOpen={(item) => void openRemoteDir(item.name)}
                  onTransfer={(item) => requestTransfer('down', item)}
                  onDelete={(item) => setDialog({ kind: 'delete', entry: item })}
                />
              ))
            )}
          </ul>
          <div className="pane-footer muted small">
            Clic en carpeta para abrir · ↓ descarga el archivo seleccionado
          </div>
        </div>
      </div>

      {progress && (
        <div className="pane-progress" role="status">
          <span className="pane-progress-label">
            {progress.direction === 'download' ? '↓' : '↑'} {progress.name} ·{' '}
            {fmtSize(progress.transferred)}
            {progress.total > 0 && ` / ${fmtSize(progress.total)}`}
          </span>
          <div
            className="pane-progress-track"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={
              progress.total > 0
                ? Math.min(100, Math.round((progress.transferred / progress.total) * 100))
                : undefined
            }
          >
            <div
              className="pane-progress-fill"
              style={{
                width:
                  progress.total > 0
                    ? `${Math.min(100, (progress.transferred / progress.total) * 100)}%`
                    : '100%',
              }}
            />
          </div>
        </div>
      )}

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
      {dialog?.kind === 'overwrite' && (
        <PromptDialog
          title={`¿Sobrescribir ${dialog.entry.name}?`}
          description={
            dialog.direction === 'down'
              ? 'Ya existe un archivo local con ese nombre y se reemplazará.'
              : 'Ya existe un archivo remoto con ese nombre y se reemplazará.'
          }
          confirmLabel="Sobrescribir"
          danger
          onCancel={() => setDialog(null)}
          onConfirm={() => {
            const { direction, entry } = dialog;
            setDialog(null);
            if (direction === 'down') void performDownload(entry);
            else void performUpload(entry);
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
      {passwordPromptOpen && (
        <PromptDialog
          title={`Contraseña para ${profile}`}
          description="No hay una contraseña guardada para este perfil. Se guardará si la conexión funciona."
          label="Contraseña"
          inputType="password"
          trimValue={false}
          confirmLabel="Conectar"
          requireValue
          onConfirm={(value) => {
            setPasswordPromptOpen(false);
            setPassword(value);
          }}
          onCancel={() => setPasswordPromptOpen(false)}
        />
      )}
    </div>
  );
}
