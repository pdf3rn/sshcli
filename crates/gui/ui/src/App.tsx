import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProfileModal from './ProfileModal';
import SftpPanel from './SftpPanel';
import StatusBar from './StatusBar';
import TabsBar from './TabsBar';
import TerminalTab from './TerminalTab';
import TopBar from './TopBar';
import TunnelPanel from './TunnelPanel';
import ViewPlaceholder from './ViewPlaceholder';
import type { Profile, Tab, View } from './types';
import './styles.css';

type ModalState = { open: boolean; editing: Profile | null };

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ open: false, editing: null });
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [splitId, setSplitId] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [profilesLoaded, setProfilesLoaded] = useState(false);
  const [view, setView] = useState<View>('home');

  const tabsRef = useRef<Tab[]>([]);
  tabsRef.current = tabs;
  const activeRef = useRef<string | null>(null);
  activeRef.current = activeTabId;
  const selectedRef = useRef<string | null>(null);
  selectedRef.current = selected;

  const refresh = useCallback(
    () =>
      invoke<Profile[]>('list_profiles')
        .then(setProfiles)
        .catch((reason) => setError(String(reason)))
        .finally(() => setProfilesLoaded(true)),
    [],
  );

  useEffect(() => {
    refresh();
    const unlisten = listen<{ id: string; status: string }>('ssh-status', (event) => {
      if (event.payload.status !== 'closed') return;
      setTabs((current) =>
        current.map((tab) =>
          tab.kind === 'terminal' && tab.id === event.payload.id
            ? { ...tab, connected: false }
            : tab,
        ),
      );
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

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

  const connect = useCallback(async (profileName: string) => {
    setConnecting(true);
    try {
      const id = await invoke<string>('ssh_connect', {
        profileName,
        columns: 120,
        rows: 40,
      });
      setTabs((current) => [
        ...current,
        { kind: 'terminal', id, profile: profileName, connected: true },
      ]);
      setActiveTabId(id);
      setSelected(profileName);
      setView('session');
    } catch (reason) {
      setError(String(reason));
    } finally {
      setConnecting(false);
    }
  }, []);

  const closeTab = useCallback((id: string) => {
    const current = tabsRef.current;
    const index = current.findIndex((tab) => tab.id === id);
    if (index === -1) return;
    const tab = current[index];
    if (tab.kind === 'terminal') {
      invoke('ssh_close', { id }).catch(() => undefined);
    }
    const next = current.filter((item) => item.id !== id);
    setTabs(next);
    setSplitId((value) => (value === id ? null : value));
    setActiveTabId((active) => {
      if (active !== id) return active;
      const fallback = next[Math.min(index, next.length - 1)];
      return fallback ? fallback.id : null;
    });
  }, []);

  const reconnect = useCallback(async (sessionId: string) => {
    const tab = tabsRef.current.find((item) => item.id === sessionId);
    if (!tab || tab.kind !== 'terminal') return;
    setConnecting(true);
    try {
      const id = await invoke<string>('ssh_connect', {
        profileName: tab.profile,
        columns: 120,
        rows: 40,
      });
      setTabs((current) =>
        current.map((item) =>
          item.id === sessionId && item.kind === 'terminal'
            ? { kind: 'terminal', id, profile: item.profile, connected: true }
            : item,
        ),
      );
      setActiveTabId(id);
      setSplitId((value) => (value === sessionId ? null : value));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setConnecting(false);
    }
  }, []);

  const openPanel = useCallback((kind: 'sftp' | 'tunnels', profileName: string) => {
    const id = `${kind}:${profileName}`;
    setTabs((current) =>
      current.some((tab) => tab.id === id)
        ? current
        : [
            ...current,
            kind === 'sftp'
              ? { kind: 'sftp' as const, id, profile: profileName }
              : { kind: 'tunnels' as const, id, profile: profileName },
          ],
    );
    setActiveTabId(id);
    setSelected(profileName);
    setView('session');
  }, []);

  const cycleTabs = useCallback((offset: number) => {
    const list = tabsRef.current;
    if (list.length === 0) return;
    const index = list.findIndex((tab) => tab.id === activeRef.current);
    const next = list[(((index === -1 ? 0 : index) + offset % list.length) % list.length + list.length) % list.length];
    setActiveTabId(next.id);
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key === 't' && !event.shiftKey) {
        event.preventDefault();
        const name = selectedRef.current;
        if (name) void connect(name);
        else setModal({ open: true, editing: null });
      } else if (key === 'w') {
        event.preventDefault();
        if (activeRef.current) closeTab(activeRef.current);
      } else if ((key === 'tab' && !event.shiftKey) || key === 'pagedown') {
        event.preventDefault();
        cycleTabs(1);
      } else if ((key === 'tab' && event.shiftKey) || key === 'pageup') {
        event.preventDefault();
        cycleTabs(-1);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [connect, closeTab, cycleTabs]);

  const selectedProfile = profiles.find((profile) => profile.name === selected) ?? null;
  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? null;
  const terminalTabs = tabs.filter((tab): tab is Extract<Tab, { kind: 'terminal' }> => tab.kind === 'terminal');

  const visibleTerminals = new Set<string>();
  if (activeTab?.kind === 'terminal') {
    visibleTerminals.add(activeTab.id);
    if (
      splitId &&
      splitId !== activeTab.id &&
      terminalTabs.some((tab) => tab.id === splitId)
    ) {
      visibleTerminals.add(splitId);
    }
  }

  const canSplit =
    activeTab?.kind === 'terminal' && terminalTabs.length >= 2;

  const toggleSplit = () => {
    if (!canSplit) return;
    if (splitId) {
      setSplitId(null);
      return;
    }
    const candidate = terminalTabs.find((tab) => tab.id !== activeTabId);
    if (candidate) setSplitId(candidate.id);
  };

  const liveSessions = terminalTabs.filter((tab) => tab.connected).length;

  return (
    <div className="app">
      <TopBar view={view} liveSessions={liveSessions} onNavigate={setView} />

      {error && (
        <div className="toast error" role="alert">
          <span className="toast-message">{error}</span>
          <button
            type="button"
            className="toast-close"
            aria-label="Descartar error"
            onClick={() => setError(null)}
          >
            ✕
          </button>
        </div>
      )}

      <div className="workspace">
        {view === 'home' && (
          <ViewPlaceholder
            title="Inicio"
            description="Acceso rápido a tus conexiones recientes y acciones frecuentes."
          />
        )}
        {view === 'connections' && (
          <ViewPlaceholder
            title="Conexiones"
            description="Gestiona tus perfiles SSH en una tabla con grupos, etiquetas y búsqueda."
          />
        )}
        {view === 'settings' && (
          <ViewPlaceholder
            title="Ajustes"
            description="Tipografía, comportamiento del terminal y telemetría del host."
          />
        )}

        {view === 'session' && (
          <>
      <aside className="sidebar">
        <div className="sidebar-header">
          <h1 className="brand">sshcli</h1>
          <button className="icon-btn" title="Nueva conexión" aria-label="Nueva conexión" onClick={openCreate}>
            +
          </button>
        </div>
        <p className="muted small">Conexiones ({profiles.length})</p>
        {!profilesLoaded ? (
          <ul className="profiles" aria-busy="true" aria-label="Cargando conexiones">
            {[0, 1, 2].map((index) => (
              <li key={index} className="profile-skeleton shimmer" aria-hidden="true" />
            ))}
          </ul>
        ) : profiles.length === 0 ? (
          <p className="muted empty">No hay perfiles todavía.</p>
        ) : (
        <ul className="profiles">
          {profiles.map((profile) => {
            const liveSessions = terminalTabs.filter(
              (tab) => tab.profile === profile.name && tab.connected,
            ).length;
            return (
              <li
                key={profile.name}
                className={`profile ${selected === profile.name ? 'active' : ''}`}
              >
                <button
                  type="button"
                  className="profile-select"
                  onClick={() => setSelected(profile.name)}
                  onDoubleClick={() => void connect(profile.name)}
                >
                  <span className="profile-name">
                    {liveSessions > 0 && <span className="live-dot" aria-hidden="true" />}
                    {profile.name}
                    {liveSessions > 1 && ` ·${liveSessions}`}
                    {liveSessions > 0 && (
                      <span className="sr-only">
                        {' '}({liveSessions === 1 ? '1 sesión activa' : `${liveSessions} sesiones activas`})
                      </span>
                    )}
                  </span>
                  <span className="profile-endpoint">
                    {profile.username}@{profile.host}:{profile.port}
                  </span>
                </button>
                <div className="profile-actions">
                  <button
                    type="button"
                    className="icon-btn small profile-connect"
                    aria-label={`Conectar a ${profile.name}`}
                    title="Conectar"
                    disabled={connecting}
                    onClick={() => void connect(profile.name)}
                  >
                    →
                  </button>
                  <button
                    type="button"
                    className="icon-btn small"
                    aria-label={`Editar ${profile.name}`}
                    title="Editar"
                    onClick={(event) => {
                      event.stopPropagation();
                      openEdit(profile);
                    }}
                  >
                    ✎
                  </button>
                  <button
                    type="button"
                    className={`icon-btn small ${confirmDelete === profile.name ? 'danger' : ''}`}
                    aria-label={
                      confirmDelete === profile.name
                        ? `Confirmar borrado de ${profile.name}`
                        : `Borrar ${profile.name}`
                    }
                    title={confirmDelete === profile.name ? 'Confirmar borrado' : 'Borrar'}
                    onClick={(event) => {
                      event.stopPropagation();
                      handleDelete(profile.name);
                    }}
                  >
                    {confirmDelete === profile.name ? '✓' : '✕'}
                  </button>
                </div>
              </li>
            );
          })}
          </ul>
        )}
      </aside>

      <div className="main">
        <div className="topbar">
          <button className="btn small" onClick={openCreate}>
            + Conexión
          </button>
          <button
            className="btn small"
            disabled={!canSplit}
            title={
              canSplit
                ? splitId
                  ? 'Volver a un solo panel'
                  : 'Dividir en dos paneles'
                : 'Abre al menos dos sesiones para dividir'
            }
            onClick={toggleSplit}
          >
            {splitId ? 'Unir vista' : 'Dividir'}
          </button>
          <button
            className="btn small"
            disabled={!activeTab}
            onClick={() => activeTab && closeTab(activeTab.id)}
          >
            Cerrar pestaña
          </button>
          {connecting && <span className="muted small topbar-note">Conectando…</span>}
        </div>

        {tabs.length > 0 && (
          <TabsBar
            tabs={tabs}
            activeId={activeTabId}
            onSelect={setActiveTabId}
            onClose={closeTab}
          />
        )}

        <main className={`content ${visibleTerminals.size > 1 ? 'split' : ''}`}>
          {terminalTabs.map((tab) => (
            <div
              key={tab.id}
              className="pane-slot"
              style={{ display: visibleTerminals.has(tab.id) ? undefined : 'none' }}
            >
              <TerminalTab
                sessionId={tab.id}
                profile={tab.profile}
                connected={tab.connected}
                visible={visibleTerminals.has(tab.id)}
                onClose={closeTab}
                onReconnect={reconnect}
              />
            </div>
          ))}

          {activeTab?.kind === 'sftp' && (
            <SftpPanel profile={activeTab.profile} onClose={() => closeTab(activeTab.id)} />
          )}
          {activeTab?.kind === 'tunnels' && (
            <TunnelPanel profile={activeTab.profile} onClose={() => closeTab(activeTab.id)} />
          )}

          {!activeTab &&
            (selectedProfile ? (
              <div className="details">
                <h2>{selectedProfile.name}</h2>
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
                <div className="detail-actions">
                  <button
                    className="btn primary connect"
                    disabled={connecting}
                    onClick={() => void connect(selectedProfile.name)}
                  >
                    {connecting ? 'Conectando…' : 'Conectar'}
                  </button>
                  <button
                    className="btn connect"
                    onClick={() => openPanel('sftp', selectedProfile.name)}
                  >
                    SFTP
                  </button>
                  <button
                    className="btn connect"
                    onClick={() => openPanel('tunnels', selectedProfile.name)}
                  >
                    Túneles
                  </button>
                </div>
              </div>
            ) : (
              <div className="details">
                <h2>Workspace</h2>
                <p className="muted">
                  Selecciona una conexión y pulsa el botón <strong>→</strong> (o doble clic sobre
                  ella) para abrir una sesión SSH.
                </p>
                <p className="muted small">
                  SFTP y túneles se abren como pestañas desde la vista de cada perfil.
                </p>
              </div>
            ))}

        </main>
          </div>
          </>
        )}
      </div>

      <StatusBar liveSessions={liveSessions} />

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
