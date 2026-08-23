import { useMemo, useState } from 'react';
import type { Profile } from './types';
import { attemptAdhoc, isValidAdhocTarget } from './adhoc';
import { useDialog } from './use-dialog';

type Props = {
  profiles: Profile[];
  liveProfiles: ReadonlySet<string>;
  connecting: boolean;
  onConnectProfile: (name: string) => void;
  onConnectAdhoc: (target: string, password?: string) => Promise<void>;
  onClose: () => void;
};

export default function NewConnectionModal({
  profiles,
  liveProfiles,
  connecting,
  onConnectProfile,
  onConnectAdhoc,
  onClose,
}: Props) {
  const [query, setQuery] = useState('');
  const [adhocTarget, setAdhocTarget] = useState('');
  const [adhocPassword, setAdhocPassword] = useState('');
  const [passwordVisible, setPasswordVisible] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useDialog<HTMLDivElement>(onClose);

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const sorted = [...profiles].sort(
      (a, b) => Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name),
    );
    if (!needle) return sorted;
    return sorted.filter((profile) => {
      const haystack = [
        profile.name,
        profile.username,
        profile.host,
        profile.group ?? '',
        profile.tags.join(' '),
      ]
        .join(' ')
        .toLowerCase();
      return haystack.includes(needle);
    });
  }, [profiles, query]);

  const connectProfile = (name: string) => {
    onClose();
    onConnectProfile(name);
  };

  const handleAdhoc = async () => {
    setError(null);
    const target = adhocTarget.trim();
    if (!isValidAdhocTarget(target)) {
      setError('Formato esperado: usuario@host[:puerto]');
      return;
    }
    const outcome = await attemptAdhoc(
      async () => {
        await onConnectAdhoc(target, passwordVisible ? adhocPassword || undefined : undefined);
        onClose();
      },
      passwordVisible,
    );
    if (outcome.status === 'ok') return;
    if (outcome.status === 'password-required') {
      setPasswordVisible(true);
      setError('Este servidor requiere contraseña.');
      requestAnimationFrame(() =>
        dialogRef.current?.querySelector<HTMLInputElement>('.modal-adhoc-password')?.focus(),
      );
    } else {
      setError(outcome.message);
    }
  };

  const submitFirstMatch = (event: React.FormEvent) => {
    event.preventDefault();
    if (matches.length > 0) connectProfile(matches[0].name);
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-connection-title"
        className="modal modal-new-connection"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="new-connection-title">Nueva conexión</h2>

        <form onSubmit={submitFirstMatch} className="modal-section">
          <label className="field-label" htmlFor="new-conn-search">
            Conexiones guardadas
          </label>
          <input
            id="new-conn-search"
            type="search"
            value={query}
            placeholder="Buscar por nombre, host, grupo o etiqueta…"
            aria-label="Buscar conexiones guardadas"
            autoComplete="off"
            onChange={(event) => setQuery(event.target.value)}
          />
          {profiles.length === 0 ? (
            <p className="muted small">Aún no hay perfiles guardados.</p>
          ) : matches.length === 0 ? (
            <p className="muted small">Sin coincidencias para «{query.trim()}».</p>
          ) : (
            <ul className="modal-profile-list">
              {matches.map((profile) => (
                <li key={profile.name}>
                  <button
                    type="button"
                    className="modal-profile-row"
                    disabled={connecting}
                    onClick={() => connectProfile(profile.name)}
                  >
                    <span className="recent-name">
                      {liveProfiles.has(profile.name) && (
                        <span className="live-dot" aria-hidden="true" />
                      )}
                      {profile.name}
                      {profile.favorite && <span className="fav-star">★</span>}
                    </span>
                    <span className="modal-profile-endpoint">
                      {profile.username}@{profile.host}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <p className="muted small">Enter conecta la primera coincidencia.</p>
        </form>

        <form
          onSubmit={(event) => {
            event.preventDefault();
            void handleAdhoc();
          }}
          className="modal-section modal-divider"
        >
          <span className="field-label">O conexión rápida</span>
          {passwordVisible && (
            <input
              type="password"
              className="modal-adhoc-password"
              value={adhocPassword}
              placeholder="Contraseña requerida por el servidor"
              aria-label="Contraseña para conexión rápida"
              autoComplete="current-password"
              onChange={(event) => setAdhocPassword(event.target.value)}
            />
          )}
          <div className="modal-adhoc-row">
            <input
              type="text"
              value={adhocTarget}
              placeholder="usuario@host:puerto"
              aria-label="Conexión rápida usuario@host"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => {
                setAdhocTarget(event.target.value);
                setError(null);
                if (!event.target.value.trim()) setPasswordVisible(false);
              }}
            />
            <button type="submit" className="btn primary small" disabled={connecting}>
              Conectar
            </button>
          </div>
          {error && (
            <p className="modal-error" role="alert">
              {error}
            </p>
          )}
        </form>

        <div className="modal-actions">
          <button type="button" className="btn ghost" onClick={onClose}>
            Cancelar
          </button>
        </div>
      </div>
    </div>
  );
}
