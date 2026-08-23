import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ProfileModal from './ProfileModal';
import './styles.css';

type Profile = {
  name: string;
  host: string;
  port: number;
  username: string;
  identity_file: string | null;
  authentication: 'None' | 'Password' | 'PrivateKey';
  accept_unknown_host_key: boolean;
};

type ModalState = { open: boolean; editing: Profile | null };

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ open: false, editing: null });
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const refresh = () =>
    invoke<Profile[]>('list_profiles')
      .then(setProfiles)
      .catch((reason) => setError(String(reason)));

  useEffect(() => {
    refresh();
  }, []);

  const openCreate = () => setModal({ open: true, editing: null });
  const openEdit = (profile: Profile) => setModal({ open: true, editing: profile });

  const handleDelete = (name: string) => {
    if (confirmDelete === name) {
      invoke('delete_profile', { name })
        .then(() => {
          setConfirmDelete(null);
          setSelected((current) => (current === name ? null : current));
          return refresh();
        })
        .catch((reason) => setError(String(reason)));
    } else {
      setConfirmDelete(name);
    }
  };

  const selectedProfile = profiles.find((profile) => profile.name === selected) ?? null;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="sidebar-header">
          <h1 className="brand">sshcli</h1>
          <button className="icon-btn" title="Nueva conexión" onClick={openCreate}>
            +
          </button>
        </div>
        <p className="muted small">Conexiones ({profiles.length})</p>
        {profiles.length === 0 && <p className="muted empty">No hay perfiles todavía.</p>}
        <ul className="profiles">
          {profiles.map((profile) => (
            <li
              key={profile.name}
              className={`profile ${selected === profile.name ? 'active' : ''}`}
              onClick={() => setSelected(profile.name)}
            >
              <span className="profile-row">
                <span className="profile-name">{profile.name}</span>
                <button
                  className="icon-btn small"
                  title="Editar"
                  onClick={(event) => {
                    event.stopPropagation();
                    openEdit(profile);
                  }}
                >
                  ✎
                </button>
                <button
                  className={`icon-btn small ${confirmDelete === profile.name ? 'danger' : ''}`}
                  title={confirmDelete === profile.name ? 'Confirmar borrado' : 'Borrar'}
                  onClick={(event) => {
                    event.stopPropagation();
                    handleDelete(profile.name);
                  }}
                >
                  {confirmDelete === profile.name ? '✓' : '✕'}
                </button>
              </span>
              <span className="profile-endpoint">
                {profile.username}@{profile.host}:{profile.port}
              </span>
            </li>
          ))}
        </ul>
      </aside>

      <main className="workspace">
        {selectedProfile ? (
          <div className="details">
            <h2>{selectedProfile.name}</h2>
            <p className="muted">Sesión SSH — próximamente en la Fase 3.</p>
            <dl className="detail-grid">
              <dt>Endpoint</dt>
              <dd>
                {selectedProfile.host}:{selectedProfile.port}
              </dd>
              <dt>Usuario</dt>
              <dd>{selectedProfile.username}</dd>
              <dt>Autenticación</dt>
              <dd>{selectedProfile.authentication}</dd>
              <dt>Clave</dt>
              <dd>{selectedProfile.identity_file ?? '—'}</dd>
            </dl>
          </div>
        ) : (
          <div className="details">
            <h2>Workspace</h2>
            <p className="muted">Selecciona una conexión para ver sus detalles.</p>
          </div>
        )}
        {error && (
          <button className="toast error" onClick={() => setError(null)}>
            {error} (cerrar)
          </button>
        )}
      </main>

      {modal.open && (
        <ProfileModal
          editing={modal.editing}
          onClose={() => setModal({ open: false, editing: null })}
          onSaved={() => {
            setModal({ open: false, editing: null });
            return refresh();
          }}
        />
      )}
    </div>
  );
}

export default App;
