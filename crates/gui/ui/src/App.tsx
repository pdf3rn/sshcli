import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProfileModal from './ProfileModal';
import NewConnectionModal from './NewConnectionModal';
import ConnectionsView from './ConnectionsView';
import HomeView from './HomeView';
import RemoteExplorerPanel from './RemoteExplorerPanel';
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
  const [newConnModalOpen, setNewConnModalOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [splitId, setSplitId] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [tabCwds, setTabCwds] = useState<Record<string, string>>({});

  const rememberCwd = useCallback((sessionId: string, path: string) => {
    setTabCwds((current) => ({ ...current, [sessionId]: path }));
  }, []);
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
          (tab.kind === 'terminal' || tab.kind === 'local') && tab.id === event.payload.id
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

  const connectingRef = useRef(false);

  const connect = useCallback(async (profileName: string) => {
    if (connectingRef.current) return;
    connectingRef.current = true;
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
      connectingRef.current = false;
      setConnecting(false);
    }
  }, []);

  const adhocLabel = (target: string) => {
    const at = target.indexOf('@');
    const host = target.slice(at + 1).split(':')[0];
    return `${target.slice(0, at)}@${host}`;
  };

  const connectAdhoc = useCallback(async (target: string, password?: string) => {
    if (connectingRef.current) return;
    connectingRef.current = true;
    setConnecting(true);
    try {
      const id = await invoke<string>('ssh_connect_adhoc', {
        target,
        password,
        columns: 120,
        rows: 40,
      });
      setTabs((current) => [
        ...current,
        { kind: 'terminal', id, profile: adhocLabel(target), connected: true },
      ]);
      setActiveTabId(id);
      setView('session');
    } finally {
      connectingRef.current = false;
      setConnecting(false);
    }
  }, []);

  const openLocalTab = useCallback(async () => {
    if (connectingRef.current) return;
    connectingRef.current = true;
    setConnecting(true);
    try {
      const info = await invoke<{ id: string; profile: string }>('local_shell_start', {
        columns: 120,
        rows: 40,
        shell: prefs.localShell || undefined,
      });
      setTabs((current) => [
        ...current,
        { kind: 'local', id: info.id, profile: info.profile, connected: true },
      ]);
      setActiveTabId(info.id);
      setView('session');
    } catch (reason) {
      setError(String(reason));
    } finally {
      connectingRef.current = false;
      setConnecting(false);
    }
  }, [prefs.localShell]);

  const reorderTabs = useCallback((dragId: string, targetId: string) => {
    setTabs((current) => {
      const from = current.findIndex((tab) => tab.id === dragId);
      const to = current.findIndex((tab) => tab.id === targetId);
      if (from === -1 || to === -1 || from === to) return current;
      const next = [...current];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      return next;
    });
  }, []);

  const closeTab = useCallback((id: string) => {
    const current = tabsRef.current;
    const index = current.findIndex((tab) => tab.id === id);
    if (index === -1) return;
    const tab = current[index];
    if (tab.kind === 'terminal') {
      invoke('ssh_close', { id }).catch(() => undefined);
    } else if (tab.kind === 'local') {
      invoke('local_close', { id }).catch(() => undefined);
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

  const reconnect = useCallback(
    async (sessionId: string) => {
      const tab = tabsRef.current.find((item) => item.id === sessionId);
      if (!tab || (tab.kind !== 'terminal' && tab.kind !== 'local')) return;
      if (tab.kind === 'local') {
        invoke('local_close', { id: sessionId }).catch(() => undefined);
        try {
          const info = await invoke<{ id: string; profile: string }>('local_shell_start', {
            columns: 120,
            rows: 40,
            shell: prefs.localShell || undefined,
          });
          setTabs((current) =>
            current.map((item) =>
              item.id === sessionId && item.kind === 'local'
                ? { kind: 'local', id: info.id, profile: info.profile, connected: true }
                : item,
            ),
          );
          setActiveTabId(info.id);
          setSplitId((value) => (value === sessionId ? null : value));
        } catch (reason) {
          setError(String(reason));
        }
        return;
      }
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
    },
    [prefs.localShell],
  );

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
  const shellTabs = tabs.filter(
    (tab): tab is Extract<Tab, { kind: 'terminal' }> | Extract<Tab, { kind: 'local' }> =>
      tab.kind === 'terminal' || tab.kind === 'local',
  );

  const visibleTerminals = new Set<string>();
  if (activeTab?.kind === 'terminal' || activeTab?.kind === 'local') {
    visibleTerminals.add(activeTab.id);
    if (
      splitId &&
      splitId !== activeTab.id &&
      shellTabs.some((tab) => tab.id === splitId)
    ) {
      visibleTerminals.add(splitId);
    }
  }

  const canSplit =
    (activeTab?.kind === 'terminal' || activeTab?.kind === 'local') &&
    shellTabs.length >= 2;

  const toggleSplit = () => {
    if (!canSplit) return;
    if (splitId) {
      setSplitId(null);
      return;
    }
    const candidate = shellTabs.find((tab) => tab.id !== activeTabId);
    if (candidate) setSplitId(candidate.id);
  };

  const liveSessions = shellTabs.filter((tab) => tab.connected).length;
  const liveProfileNames = new Set(
    shellTabs.filter((tab) => tab.connected).map((tab) => tab.profile),
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
            onConnectAdhoc={connectAdhoc}
            connecting={connecting}
            onCreate={openCreate}
            onBrowseAll={() => setView('connections')}
            onImported={refresh}
            onOpenLocal={() => void openLocalTab()}
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

        <div className="main" style={{ display: view === 'session' ? undefined : 'none' }}>
          {tabs.length > 0 && (
            <TabsBar
              tabs={tabs}
              activeId={activeTabId}
              onSelect={setActiveTabId}
              onClose={closeTab}
              onReorder={reorderTabs}
              onAdd={() => setNewConnModalOpen(true)}
            />
          )}

        <main className={`content ${visibleTerminals.size > 1 ? 'split' : ''}`}>
          {shellTabs.map((tab) => (
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
                transport={tab.kind === 'local' ? 'local' : 'ssh'}
                onClose={closeTab}
                onReconnect={reconnect}
                onCwd={rememberCwd}
              />
            </div>
          ))}

          {activeTab?.kind === 'sftp' && (
            <SftpPanel profile={activeTab.profile} />
          )}
          {activeTab?.kind === 'terminal' && prefs.telemetryEnabled && prefs.telemetryPanelOpen && (
            <TelemetryPanel profile={activeTab.profile} />
          )}
          {activeTab?.kind === 'terminal' &&
            prefs.remoteExplorerEnabled &&
            prefs.remoteExplorerOpen && (
              <RemoteExplorerPanel
                key={activeTab.id}
                sessionId={activeTab.id}
                cwd={tabCwds[activeTab.id]}
              />
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
      </div>

      <StatusBar
        liveSessions={liveSessions}
        connecting={connecting}
        canSplit={canSplit}
        splitActive={splitId !== null}
        onToggleSplit={toggleSplit}
        telemetryAvailable={
          activeTab?.kind === 'terminal' &&
          prefs.telemetryEnabled === true &&
          profiles.some((profile) => profile.name === activeTab.profile)
        }
        telemetryOpen={prefs.telemetryPanelOpen}
        onToggleTelemetry={() => updatePrefs({ telemetryPanelOpen: !prefs.telemetryPanelOpen })}
        explorerAvailable={activeTab?.kind === 'terminal' && prefs.remoteExplorerEnabled === true}
        explorerOpen={prefs.remoteExplorerOpen}
        onToggleExplorer={() =>
          updatePrefs({ remoteExplorerOpen: !prefs.remoteExplorerOpen })
        }
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

      {newConnModalOpen && (
        <NewConnectionModal
          profiles={profiles}
          liveProfiles={liveProfileNames}
          connecting={connecting}
          onConnectProfile={(name) => void connect(name)}
          onConnectAdhoc={connectAdhoc}
          onOpenLocal={() => void openLocalTab()}
          onClose={() => setNewConnModalOpen(false)}
        />
      )}
    </div>
  );
}

export default App;
