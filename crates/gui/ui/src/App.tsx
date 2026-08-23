import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProfileModal from './ProfileModal';
import ConnectionsView from './ConnectionsView';
import HomeView from './HomeView';
import SettingsView from './SettingsView';
import SftpPanel from './SftpPanel';
import TelemetryPanel from './TelemetryPanel';
import StatusBar from './StatusBar';
import TabsBar from './TabsBar';
import TerminalTab from './TerminalTab';
import TopBar from './TopBar';
import TunnelPanel from './TunnelPanel';
import ViewPlaceholder from './ViewPlaceholder';
import type { Profile, Tab, View } from './types';
import { usePrefs } from './prefs';
import './styles.css';

type ModalState = { open: boolean; editing: Profile | null };

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ open: false, editing: null });
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [splitId, setSplitId] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [view, setView] = useState<View>('home');
  const [prefs, updatePrefs] = usePrefs();

  const tabsRef = useRef<Tab[]>([]);
  tabsRef.current = tabs;
  const activeRef = useRef<string | null>(null);
  activeRef.current = activeTabId;

  const refresh = useCallback(
    () =>
      invoke<Profile[]>('list_profiles')
        .then(setProfiles)
        .catch((reason) => setError(String(reason))),
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
        setView('connections');
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

  const knownGroups = Array.from(
    new Set(profiles.map((profile) => profile.group).filter((group): group is string => !!group)),
  ).sort();
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
  const liveProfileNames = new Set(
    terminalTabs.filter((tab) => tab.connected).map((tab) => tab.profile),
  );

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
          <HomeView
            profiles={profiles}
            liveProfiles={liveProfileNames}
            onConnect={(name) => void connect(name)}
            onCreate={openCreate}
            onBrowseAll={() => setView('connections')}
            onImported={refresh}
          />
        )}
        {view === 'connections' && (
          <ConnectionsView
            profiles={profiles}
            liveProfiles={liveProfileNames}
            connecting={connecting}
            onConnect={(name) => void connect(name)}
            onOpenPanel={openPanel}
            onEdit={openEdit}
            onDelete={handleDelete}
            onToggleFavorite={(name) => {
              void invoke<boolean>('toggle_favorite', { name })
                .then(() => refresh())
                .catch((reason) => setError(String(reason)));
            }}
            onCreate={openCreate}
          />
        )}
        {view === 'settings' && <SettingsView prefs={prefs} onChange={updatePrefs} />}

        {view === 'session' && (
          <>
      <div className="main">
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
              data-profile={tab.profile}
              style={{ display: visibleTerminals.has(tab.id) ? undefined : 'none' }}
            >
              <TerminalTab
                sessionId={tab.id}
                connected={tab.connected}
                visible={visibleTerminals.has(tab.id)}
                prefs={prefs}
                onClose={closeTab}
                onReconnect={reconnect}
              />
            </div>
          ))}

          {activeTab?.kind === 'sftp' && (
            <SftpPanel profile={activeTab.profile} onClose={() => closeTab(activeTab.id)} />
          )}
          {activeTab?.kind === 'terminal' && prefs.telemetryEnabled && prefs.telemetryPanelOpen && (
            <TelemetryPanel profile={activeTab.profile} />
          )}
          {activeTab?.kind === 'tunnels' && (
            <TunnelPanel profile={activeTab.profile} onClose={() => closeTab(activeTab.id)} />
          )}

          {!activeTab && (
            <div className="details">
              <h2>Sesiones</h2>
              <p className="muted">
                No hay sesiones abiertas. Conéctate desde la vista de{' '}
                <strong>Conexiones</strong> o crea un perfil nuevo.
              </p>
              <div className="detail-actions">
                <button
                  type="button"
                  className="btn primary"
                  onClick={() => setView('connections')}
                >
                  Ir a Conexiones
                </button>
                <button type="button" className="btn" onClick={openCreate}>
                  Nueva conexión
                </button>
              </div>
            </div>
          )}

        </main>
          </div>
          </>
        )}
      </div>

      <StatusBar
        liveSessions={liveSessions}
        connecting={connecting}
        canSplit={canSplit}
        splitActive={splitId !== null}
        onToggleSplit={toggleSplit}
        telemetryAvailable={activeTab?.kind === 'terminal' && prefs.telemetryEnabled === true}
        telemetryOpen={prefs.telemetryPanelOpen}
        onToggleTelemetry={() => updatePrefs({ telemetryPanelOpen: !prefs.telemetryPanelOpen })}
      />

      {modal.open && (
        <ProfileModal
          editing={modal.editing}
          knownGroups={knownGroups}
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
