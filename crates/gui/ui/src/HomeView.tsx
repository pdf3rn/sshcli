import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Profile } from './types';

type Props = {
  profiles: Profile[];
  liveProfiles: ReadonlySet<string>;
  onConnect: (name: string) => void;
  onCreate: () => void;
  onBrowseAll: () => void;
  onImported: () => void;
};

function formatLastUsed(secs: number | null): string {
  if (!secs) return 'Sin usar';
  const diffSeconds = Date.now() / 1000 - secs;
  if (diffSeconds < 90) return 'Ahora mismo';
  if (diffSeconds < 3600) return `Hace ${Math.floor(diffSeconds / 60)} min`;
  if (diffSeconds < 86400) return `Hace ${Math.floor(diffSeconds / 3600)} h`;
  const days = Math.floor(diffSeconds / 86400);
  return days === 1 ? 'Hace 1 día' : `Hace ${days} días`;
}

const RECENT_LIMIT = 6;

export default function HomeView({
  profiles,
  liveProfiles,
  onConnect,
  onCreate,
  onBrowseAll,
  onImported,
}: Props) {
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const recents = [...profiles]
    .sort((a, b) => (b.last_used ?? 0) - (a.last_used ?? 0))
    .slice(0, RECENT_LIMIT);
  const favorites = profiles
    .filter((profile) => profile.favorite)
    .sort((a, b) => a.name.localeCompare(b.name));

  const renderCard = (profile: Profile) => (
    <li key={profile.name}>
      <button type="button" className="recent-card" onClick={() => onConnect(profile.name)}>
        <span className="recent-name">
          {liveProfiles.has(profile.name) && <span className="live-dot" aria-hidden="true" />}
          {profile.name}
          {profile.favorite && (
            <span className="fav-star" aria-label="Favorito">
              ★
            </span>
          )}
          <span className="recent-chevron" aria-hidden="true">
            ›
          </span>
        </span>
        <span className="recent-endpoint">
          {profile.username}@{profile.host}
        </span>
        <span className="recent-meta">
          {formatLastUsed(profile.last_used)}
          {profile.group ? ` · ${profile.group}` : ''}
        </span>
      </button>
    </li>
  );

  const handleImportFile = async (file: File) => {
    setImportMessage(null);
    try {
      const content = await file.text();
      const imported = await invoke<number>('import_profiles', { content });
      setImportMessage(
        imported === 0
          ? 'Nada que importar: todos los perfiles ya existen.'
          : `${imported} ${imported === 1 ? 'perfil importado' : 'perfiles importados'}.`,
      );
      if (imported > 0) onImported();
    } catch (reason) {
      setImportMessage(`Error al importar: ${reason}`);
    }
  };

  const handleExport = async () => {
    setImportMessage(null);
    try {
      const content = await invoke<string>('export_profiles');
      const blob = new Blob([content], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = 'sshcli-profiles.toml';
      anchor.click();
      URL.revokeObjectURL(url);
      setImportMessage('Configuración exportada como sshcli-profiles.toml.');
    } catch (reason) {
      setImportMessage(`Error al exportar: ${reason}`);
    }
  };

  return (
    <section className="home-view" aria-labelledby="home-title">
      <div className="home-hero">
        <h2 id="home-title">Terminal Gateway</h2>
        <p className="hero-prompt" aria-hidden="true">
          root@sshcli:~# await connections
        </p>
        <p className="muted">
          Tus conexiones SSH, SFTP y túneles en un único espacio de trabajo.
        </p>
      </div>

      {favorites.length > 0 && (
        <div className="home-section">
          <h3 className="panel-label">Favoritos</h3>
          <ul className="recent-grid">{favorites.map(renderCard)}</ul>
        </div>
      )}

      <div className="home-section">
        <div className="home-section-header">
          <h3 className="panel-label">Conexiones recientes</h3>
          <button type="button" className="btn small ghost" onClick={onBrowseAll}>
            Ver todas →
          </button>
        </div>
        {recents.length === 0 ? (
          <p className="muted">Aún no hay conexiones. Crea tu primer perfil para empezar.</p>
        ) : (
          <ul className="recent-grid">{recents.map(renderCard)}</ul>
        )}
      </div>

      <div className="home-section">
        <h3 className="panel-label">Acciones rápidas</h3>
        <div className="quick-actions">
          <button type="button" className="action-card" onClick={onCreate}>
            <span className="action-glyph" aria-hidden="true">
              +
            </span>
            Nueva conexión
          </button>
          <button
            type="button"
            className="action-card"
            onClick={() => fileInputRef.current?.click()}
          >
            <span className="action-glyph" aria-hidden="true">
              ⇪
            </span>
            Importar config
          </button>
          <button type="button" className="action-card" onClick={() => void handleExport()}>
            <span className="action-glyph" aria-hidden="true">
              ⇩
            </span>
            Exportar config
          </button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".toml,text/plain"
            hidden
            onChange={(event) => {
              const file = event.target.files?.[0];
              event.target.value = '';
              if (file) void handleImportFile(file);
            }}
          />
        </div>
        {importMessage && (
          <p className="muted small" role="status">
            {importMessage}
          </p>
        )}
      </div>
    </section>
  );
}
