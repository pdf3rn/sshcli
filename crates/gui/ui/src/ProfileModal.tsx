import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useDialog } from './use-dialog';

type Profile = {
  name: string;
  host: string;
  port: number;
  username: string;
  identity_file: string | null;
  authentication: 'None' | 'Password' | 'PrivateKey';
  accept_unknown_host_key: boolean;
};

type Props = {
  editing: Profile | null;
  onClose: () => void;
  onSaved: () => void;
};

const AUTH_METHODS = ['password', 'private-key', 'none'] as const;

export default function ProfileModal({ editing, onClose, onSaved }: Props) {
  const [name, setName] = useState(editing?.name ?? '');
  const [host, setHost] = useState(editing?.host ?? '');
  const [port, setPort] = useState(String(editing?.port ?? 22));
  const [username, setUsername] = useState(editing?.username ?? '');
  const [identityFile, setIdentityFile] = useState(editing?.identity_file ?? '');
  const [auth, setAuth] = useState<string>(
    editing ? editing.authentication.toLowerCase() : 'password',
  );
  const [acceptHostKey, setAcceptHostKey] = useState(editing?.accept_unknown_host_key ?? false);
  const [secret, setSecret] = useState('');
  const [keys, setKeys] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useDialog<HTMLDivElement>(onClose);

  useEffect(() => {
    invoke<string[]>('list_identity_keys').then(setKeys).catch(() => undefined);
  }, []);

  const needsSecret = auth === 'password' || auth === 'private-key';

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    const payload = {
      name: name.trim(),
      host: host.trim(),
      port: Number(port),
      username: username.trim(),
      identity_file: auth === 'private-key' ? identityFile.trim() || null : null,
      authentication: auth,
      accept_unknown_host_key: acceptHostKey,
      secret: needsSecret ? secret : null,
    };
    try {
      if (editing) {
        await invoke('update_profile', { input: payload });
      } else {
        await invoke('create_profile', { input: payload });
      }
      onSaved();
    } catch (reason) {
      setError(String(reason));
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="profile-dialog-title"
        className="modal"
        onClick={(event) => event.stopPropagation()}
      >
        <form onSubmit={submit}>
          <h2 id="profile-dialog-title">{editing ? `Editar ${editing.name}` : 'Nueva conexión'}</h2>

          <label className="field">
            <span>Nombre</span>
            <input value={name} onChange={(event) => setName(event.target.value)} required autoFocus />
          </label>

          <label className="field">
            <span>Host</span>
            <input value={host} onChange={(event) => setHost(event.target.value)} required />
          </label>

          <div className="field-row">
            <label className="field">
              <span>Puerto</span>
              <input
                type="number"
                min={1}
                max={65535}
                value={port}
                onChange={(event) => setPort(event.target.value)}
              />
            </label>
            <label className="field">
              <span>Usuario</span>
              <input value={username} onChange={(event) => setUsername(event.target.value)} required />
            </label>
          </div>

          <div className="field">
            <span id="auth-method-label">Autenticación</span>
            <div className="segmented" role="group" aria-labelledby="auth-method-label">
              {AUTH_METHODS.map((method) => (
                <button
                  type="button"
                  key={method}
                  className={auth === method ? 'segmented-active' : ''}
                  aria-pressed={auth === method}
                  onClick={() => setAuth(method)}
                >
                  {method}
                </button>
              ))}
            </div>
          </div>

          {auth === 'private-key' && (
            <label className="field">
              <span>Clave privada</span>
              <input
                list="ssh-keys-options"
                value={identityFile}
                onChange={(event) => setIdentityFile(event.target.value)}
                placeholder="~/.ssh/id_ed25519"
              />
              <datalist id="ssh-keys-options">
                {keys.map((key) => (
                  <option key={key} value={key} />
                ))}
              </datalist>
              {keys.length === 0 && <em className="hint">No se encontraron claves en ~/.ssh</em>}
            </label>
          )}

          {needsSecret && (
            <label className="field">
              <span>{auth === 'password' ? 'Contraseña' : 'Passphrase (vacío = sin)'}</span>
              <input
                type="password"
                autoComplete="off"
                value={secret}
                onChange={(event) => setSecret(event.target.value)}
              />
            </label>
          )}

          <label className="check">
            <input
              type="checkbox"
              checked={acceptHostKey}
              onChange={(event) => setAcceptHostKey(event.target.checked)}
            />
            <span>Aceptar clave de host desconocida</span>
          </label>

          {error && <p className="form-error" role="alert">{error}</p>}

          <div className="modal-actions">
            <button type="button" className="btn ghost" onClick={onClose}>
              Cancelar
            </button>
            <button type="submit" className="btn primary">
              Guardar
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
