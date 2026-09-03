import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Profile as ProfileType } from './types';
import { useDialog } from './use-dialog';

type Props = {
  editing: ProfileType | null;
  knownGroups: string[];
  existingNames: string[];
  onClose: () => void;
  onSaved: () => void;
};

const AUTH_METHODS = ['password', 'private-key', 'none'] as const;

export default function ProfileModal({ editing, knownGroups, existingNames, onClose, onSaved }: Props) {
  const [name, setName] = useState(editing?.name ?? '');
  const [host, setHost] = useState(editing?.host ?? '');
  const [port, setPort] = useState(String(editing?.port ?? 22));
  const [username, setUsername] = useState(editing?.username ?? '');
  const [identityFile, setIdentityFile] = useState(editing?.identity_file ?? '');
  const [auth, setAuth] = useState<string>(
    editing ? editing.authentication.toLowerCase() : 'password',
  );
  const [acceptHostKey, setAcceptHostKey] = useState(editing?.accept_unknown_host_key ?? false);
  const [group, setGroup] = useState(editing?.group ?? '');
  const [tags, setTags] = useState(editing?.tags.join(', ') ?? '');
  const [secret, setSecret] = useState('');
  const [keys, setKeys] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useDialog<HTMLDivElement>(onClose);

  useEffect(() => {
    invoke<string[]>('list_identity_keys').then(setKeys).catch(() => undefined);
  }, []);

  const needsSecret = auth === 'password' || auth === 'private-key';
  const validationErrors = [
    !name.trim() ? 'El nombre es obligatorio.' : null,
    existingNames.some(
      (existing) => existing.toLowerCase() === name.trim().toLowerCase() && existing !== editing?.name,
    )
      ? 'Ya existe una conexión con ese nombre.'
      : null,
    !host.trim() ? 'El host es obligatorio.' : /\s/.test(host.trim()) ? 'El host no puede contener espacios.' : null,
    !Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535
      ? 'El puerto debe estar entre 1 y 65535.'
      : null,
  ].filter((message): message is string => message !== null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (validationErrors.length > 0) {
      setError(validationErrors[0]);
      return;
    }
    const payload = {
      original_name: editing?.name ?? null,
      name: name.trim(),
      host: host.trim(),
      port: Number(port),
      username: username.trim(),
      identity_file: auth === 'private-key' ? identityFile.trim() || null : null,
      authentication: auth,
      accept_unknown_host_key: acceptHostKey,
      group: group.trim() || null,
      tags: tags
        .split(',')
        .map((tag) => tag.trim())
        .filter(Boolean),
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
           <input
             value={name}
             onChange={(event) => setName(event.target.value)}
             required
             autoFocus
             aria-invalid={validationErrors.some((message) => message.includes('nombre'))}
           />
          </label>

          <label className="field">
            <span>Host</span>
           <input
             value={host}
             onChange={(event) => setHost(event.target.value)}
             required
             aria-invalid={validationErrors.some((message) => message.includes('host'))}
           />
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
                aria-invalid={validationErrors.some((message) => message.includes('puerto'))}
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

          <div className="field-row">
            <label className="field">
              <span>Grupo (opcional)</span>
              <input
                list="group-options"
                value={group}
                onChange={(event) => setGroup(event.target.value)}
                placeholder="Production"
              />
              <datalist id="group-options">
                {knownGroups.map((known) => (
                  <option key={known} value={known} />
                ))}
              </datalist>
            </label>
            <label className="field">
              <span>Etiquetas (separadas por coma)</span>
              <input
                value={tags}
                onChange={(event) => setTags(event.target.value)}
                placeholder="web, api"
              />
            </label>
          </div>

           {(error || validationErrors.length > 0) && (
             <p className="form-error" role="alert">
               {error ?? validationErrors[0]}
             </p>
           )}

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
