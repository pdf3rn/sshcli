import { useMemo, useState } from 'react';
import type { Profile } from './types';
import { ConnectIcon, EditIcon, SftpIcon, StarIcon, TrashIcon, TunnelsIcon } from './icons';

type Props = {
  profiles: Profile[];
  liveProfiles: ReadonlySet<string>;
  connecting: boolean;
  onConnect: (name: string) => void;
  onOpenPanel: (kind: 'sftp' | 'tunnels', name: string) => void;
  onEdit: (profile: Profile) => void;
  onDelete: (name: string) => void;
  onToggleFavorite: (name: string) => void;
  onCreate: () => void;
};

function formatLastUsed(secs: number | null): string {
  if (!secs) return '—';
  const diffSeconds = Date.now() / 1000 - secs;
  if (diffSeconds < 90) return 'Ahora mismo';
  if (diffSeconds < 3600) {
    const minutes = Math.floor(diffSeconds / 60);
    return `Hace ${minutes} min`;
  }
  if (diffSeconds < 86400) {
    const hours = Math.floor(diffSeconds / 3600);
    return `Hace ${hours} h`;
  }
  const days = Math.floor(diffSeconds / 86400);
  return days === 1 ? 'Hace 1 día' : `Hace ${days} días`;
}

function tagClass(tag: string): string {
  const normalized = tag.toLowerCase();
  if (normalized.includes('prod')) return 'tag-chip tag-danger';
  if (normalized.includes('stag') || normalized.includes('dev')) return 'tag-chip tag-info';
  return 'tag-chip';
}

export default function ConnectionsView({
  profiles,
  liveProfiles,
  connecting,
  onConnect,
  onOpenPanel,
  onEdit,
  onDelete,
  onToggleFavorite,
  onCreate,
}: Props) {
  const [query, setQuery] = useState('');
  const [activeGroup, setActiveGroup] = useState<string>('__all__');

  const groups = useMemo(() => {
    const counts = new Map<string, number>();
    for (const profile of profiles) {
      const key = profile.group ?? '';
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return Array.from(counts.entries())
      .filter(([key]) => key !== '')
      .map(([key, count]) => ({ name: key, count }))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [profiles]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return profiles.filter((profile) => {
      if (activeGroup === '__ungrouped__') {
        if (profile.group) return false;
      } else if (activeGroup !== '__all__' && profile.group !== activeGroup) {
        return false;
      }
      if (!needle) return true;
      const haystack = [
        profile.name,
        profile.host,
        profile.username,
        profile.group ?? '',
        ...profile.tags,
      ]
        .join(' ')
        .toLowerCase();
      return haystack.includes(needle);
    });
  }, [profiles, query, activeGroup]);

  const ordered = useMemo(
    () =>
      [...filtered].sort((a, b) => Number(b.favorite) - Number(a.favorite)),
    [filtered],
  );

  const groupItems = [
    { id: '__all__', label: 'Todas las conexiones', count: profiles.length },
    ...groups.map((group) => ({ id: group.name, label: group.name, count: group.count })),
    ...(profiles.some((profile) => !profile.group)
      ? [{ id: '__ungrouped__', label: 'Sin grupo', count: profiles.filter((p) => !p.group).length }]
      : []),
  ];

  return (
    <section className="connections-view" aria-labelledby="connections-title">
      <div className="view-header">
        <div>
          <h2 id="connections-title">Conexiones</h2>
          <p className="muted">Gestiona y organiza tus hosts SSH.</p>
        </div>
        <div className="view-header-actions">
          <input
            type="search"
            className="search-input"
            placeholder="Buscar hosts…"
            aria-label="Buscar conexiones"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <button type="button" className="btn primary" onClick={onCreate}>
            Nueva conexión
          </button>
        </div>
      </div>

      <div className="conn-layout">
        <nav className="group-panel" aria-label="Grupos de conexiones">
          <h3 className="panel-label">Grupos</h3>
          <ul className="group-list">
            {groupItems.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className={`group-item ${activeGroup === item.id ? 'group-active' : ''}`}
                  aria-pressed={activeGroup === item.id}
                  onClick={() => setActiveGroup(item.id)}
                >
                  <span>{item.label}</span>
                  <span className="group-count">{item.count}</span>
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <div className="conn-table-wrap">
          {profiles.length === 0 ? (
            <div className="pane-empty conn-empty">
              <p className="muted">No hay perfiles todavía.</p>
              <button type="button" className="btn primary" onClick={onCreate}>
                Crear la primera conexión
              </button>
            </div>
          ) : filtered.length === 0 ? (
            <p className="muted conn-empty">Sin resultados para «{query}».</p>
          ) : (
            <table className="conn-table">
              <thead>
                <tr>
                  <th scope="col" className="col-status">
                    Estado
                  </th>
                  <th scope="col">Nombre</th>
                  <th scope="col">Host</th>
                  <th scope="col">Etiquetas</th>
                  <th scope="col">Última conexión</th>
                  <th scope="col" className="col-actions">
                    Acciones
                  </th>
                </tr>
              </thead>
              <tbody>
                {ordered.map((profile) => {
                  const live = liveProfiles.has(profile.name);
                  return (
                    <tr
                      key={profile.name}
                      className={live ? 'row-live' : ''}
                      onDoubleClick={() => onConnect(profile.name)}
                    >
                      <td className="col-status">
                        <span
                          className={`status-dot ${live ? 'ok pulse' : ''}`}
                          role="img"
                          aria-label={live ? 'Sesión activa' : 'Inactivo'}
                        />
                      </td>
                      <td className="cell-name">
                        {live && <span className="sr-only">(activa) </span>}
                        {profile.name}
                      </td>
                      <td className="cell-host">
                        {profile.username}@
                        <span className="host-value">{profile.host}</span>
                        <span className="port-chip">:{profile.port}</span>
                      </td>
                      <td>
                        <div className="tag-list">
                          {profile.group && (
                            <span className={`tag-chip tag-group`}>{profile.group}</span>
                          )}
                          {profile.tags.map((tag) => (
                            <span key={tag} className={tagClass(tag)}>
                              {tag}
                            </span>
                          ))}
                        </div>
                      </td>
                      <td className="cell-last">{formatLastUsed(profile.last_used)}</td>
                      <td className="col-actions">
                        <div className="row-actions">
                          <button
                            type="button"
                            className={`icon-btn small star-btn ${profile.favorite ? 'on' : ''}`}
                            aria-pressed={profile.favorite}
                            aria-label={
                              profile.favorite
                                ? `Quitar ${profile.name} de favoritos`
                                : `Añadir ${profile.name} a favoritos`
                            }
                            title={
                              profile.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos'
                            }
                            onClick={() => onToggleFavorite(profile.name)}
                          >
                            <StarIcon size={14} filled={profile.favorite} />
                          </button>
                          <button
                            type="button"
                            className="icon-btn small"
                            aria-label={`Abrir SFTP de ${profile.name}`}
                            title="Abrir SFTP"
                            onClick={() => onOpenPanel('sftp', profile.name)}
                          >
                            <SftpIcon size={14} />
                          </button>
                          <button
                            type="button"
                            className="icon-btn small"
                            aria-label={`Abrir túneles de ${profile.name}`}
                            title="Abrir túneles"
                            onClick={() => onOpenPanel('tunnels', profile.name)}
                          >
                            <TunnelsIcon size={14} />
                          </button>
                          <button
                            type="button"
                            className="icon-btn small row-connect"
                            aria-label={`Conectar a ${profile.name}`}
                            title="Conectar"
                            disabled={connecting}
                            onClick={() => onConnect(profile.name)}
                          >
                            <ConnectIcon size={14} />
                          </button>
                          <button
                            type="button"
                            className="icon-btn small"
                            aria-label={`Editar ${profile.name}`}
                            title="Editar"
                            onClick={() => onEdit(profile)}
                          >
                            <EditIcon size={14} />
                          </button>
                          <button
                            type="button"
                            className="icon-btn small"
                            aria-label={`Borrar ${profile.name}`}
                            title="Borrar (doble clic para confirmar)"
                            onClick={() => onDelete(profile.name)}
                          >
                            <TrashIcon size={14} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </section>
  );
}
