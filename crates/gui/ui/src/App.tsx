import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Profile = {
  name: string;
  host: string;
  port: number;
  username: string;
};

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Profile[]>('list_profiles')
      .then(setProfiles)
      .catch((reason) => setError(String(reason)));
  }, []);

  return (
    <div className="app">
      <aside className="sidebar">
        <h1 className="brand">sshcli</h1>
        <p className="muted">Conexiones</p>
        {profiles.length === 0 && <p className="muted empty">No hay perfiles todavía.</p>}
        <ul className="profiles">
          {profiles.map((profile) => (
            <li key={profile.name} className="profile">
              <span className="profile-name">{profile.name}</span>
              <span className="profile-endpoint">
                {profile.username}@{profile.host}:{profile.port}
              </span>
            </li>
          ))}
        </ul>
      </aside>
      <main className="workspace">
        <h2>Workspace</h2>
        <p className="muted">Selecciona una conexión para abrir una sesión.</p>
        {error && <p className="error">{error}</p>}
      </main>
    </div>
  );
}

export default App;
