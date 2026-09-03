import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProfileModal from './ProfileModal';
import NewConnectionModal from './NewConnectionModal';
import PromptDialog from './PromptDialog';
import HostKeyDialog from './HostKeyDialog';
import { parseHostKeyError, type HostKeyPrompt } from './hostkey';
import ConnectionsView from './ConnectionsView';
import HomeView from './HomeView';
import SettingsView from './SettingsView';
import StatusBar from './StatusBar';
import TerminalDockview, { type DockviewActions } from './TerminalDockview';
import TopBar from './TopBar';
import type { Profile, Tab, View } from './types';
import { usePrefs } from './prefs';
import { PASSWORD_REQUIRED } from './adhoc';
import './styles.css';

type ModalState = { open: boolean; editing: Profile | null };

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ open: false, editing: null });
  const [newConnModalOpen, setNewConnModalOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [duplicateRequest, setDuplicateRequest] = useState<Profile | null>(null);
  const [closeRequest, setCloseRequest] = useState<string | null>(null);
  const [passwordPromptProfile, setPasswordPromptProfile] = useState<string | null>(null);
  const [hostKeyPrompt, setHostKeyPrompt] = useState<HostKeyPrompt | null>(null);
  const hostKeyRetryRef = useRef<(() => void) | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
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

  useEffect(() => {
    const preventNativeMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    window.addEventListener('contextmenu', preventNativeMenu, { capture: true });
    return () => window.removeEventListener('contextmenu', preventNativeMenu, true);
  }, []);
  const dockviewActionsRef = useRef<DockviewActions | null>(null);
  const [dockviewReady, setDockviewReady] = useState(false);
  const [splitActive, setSplitActive] = useState(false);

  useEffect(() => {
    setDockviewReady(dockviewActionsRef.current !== null);
  }, [tabs]);

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

  const connect = useCallback(async (profileName: string, password?: string) => {
    if (connectingRef.current) return;
    connectingRef.current = true;
    setConnecting(true);
    try {
      const id = await invoke<string>('ssh_connect', {
        profileName,
        password: password ?? null,
        columns: 120,
        rows: 40,
      });
      if (password) {
        await invoke('save_profile_secret', { name: profileName, secret: password }).catch((reason) => {
          setError(`Conectado, pero no se pudo guardar la contraseña: ${String(reason)}`);
        });
      }
      setTabs((current) => [
        ...current,
        { kind: 'terminal', id, profile: profileName, connected: true },
      ]);
      setActiveTabId(id);
      setView('session');
    } catch (reason) {
      if (!password && String(reason).includes(PASSWORD_REQUIRED)) {
        setPasswordPromptProfile(profileName);
        return;
      }
      const hostKey = parseHostKeyError(String(reason));
      if (hostKey) {
        hostKeyRetryRef.current = () => void connect(profileName);
        setHostKeyPrompt(hostKey);
        return;
      }
      setError(String(reason));
    } finally {
      connectingRef.current = false;
      setConnecting(false);
    }
  }, []);

  const adhocLabel = (target: string) => {
    const at = target.indexOf('@');
    const hostport = target.slice(at + 1);
    const host = hostport.startsWith('[')
      ? hostport.slice(0, hostport.indexOf(']') + 1)
      : hostport.split(':')[0];
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
    } catch (reason) {
      const hostKey = parseHostKeyError(String(reason));
      if (hostKey) {
        hostKeyRetryRef.current = () => void connectAdhoc(target);
        setHostKeyPrompt(hostKey);
        return;
      }
      setError(String(reason));
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

  const finalizeCloseTab = useCallback((id: string) => {
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
    setActiveTabId((active) => {
      if (active !== id) return active;
      const fallback = next[Math.min(index, next.length - 1)];
      return fallback ? fallback.id : null;
    });
  }, []);

  const closeTab = useCallback((id: string) => {
    const tab = tabsRef.current.find((item) => item.id === id);
    if (!tab || (tab.kind !== 'terminal' && tab.kind !== 'local')) return;
    if (tab.connected) {
      setCloseRequest(id);
      return;
    }
    finalizeCloseTab(id);
  }, [finalizeCloseTab]);

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
        } catch (reason) {
          setError(String(reason));
        }
        return;
      }
      setConnecting(true);
      try {
        const id = await invoke<string>('ssh_connect', {
          profileName: tab.profile,
          password: null,
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
        } catch (reason) {
          if (String(reason).includes(PASSWORD_REQUIRED)) {
            setPasswordPromptProfile(tab.profile);
            return;
          }
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
      } else if (/^[1-9]$/.test(key) && tabsRef.current.length >= Number(key)) {
        event.preventDefault();
        setActiveTabId(tabsRef.current[Number(key) - 1].id);
        setView('session');
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
            onDuplicate={setDuplicateRequest}
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
          {tabs.length > 0 ? (
            <TerminalDockview
              tabs={tabs}
              activeId={activeTabId}
              prefs={prefs}
              tabCwds={tabCwds}
              onAdd={() => setNewConnModalOpen(true)}
              onSelect={setActiveTabId}
              onClose={closeTab}
              onReconnect={reconnect}
              onCwd={rememberCwd}
              onError={setError}
              onDockviewReady={(actions) => {
                dockviewActionsRef.current = actions;
                setDockviewReady(actions !== null);
              }}
              onLayoutChange={(groupCount) => setSplitActive(groupCount > 1)}
            />
          ) : (
            <main className="content">
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
            </main>
          )}
        </div>
      </div>

      <StatusBar
        liveSessions={liveSessions}
        connecting={connecting}
        canSplit={dockviewReady && tabs.length > 1}
        splitActive={splitActive}
        onToggleSplit={() => dockviewActionsRef.current?.toggleSplit()}
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
          onOpenPanel={openPanel}
          onConnectAdhoc={connectAdhoc}
          onOpenLocal={() => void openLocalTab()}
          onClose={() => setNewConnModalOpen(false)}
        />
      )}

      {passwordPromptProfile && (
        <PromptDialog
          title={`Contraseña para ${passwordPromptProfile}`}
          description="No hay una contraseña guardada para este perfil. Se guardará si la conexión funciona."
          label="Contraseña"
          inputType="password"
          trimValue={false}
          confirmLabel="Conectar"
          requireValue
          onConfirm={(password) => {
            const profileName = passwordPromptProfile;
            setPasswordPromptProfile(null);
            void connect(profileName, password);
          }}
          onCancel={() => setPasswordPromptProfile(null)}
        />
      )}

      {hostKeyPrompt && (
        <HostKeyDialog
          host={hostKeyPrompt.host}
          port={hostKeyPrompt.port}
          key={hostKeyPrompt.key}
          changed={hostKeyPrompt.changed}
          onConfirm={() => {
            const prompt = hostKeyPrompt;
            const retry = hostKeyRetryRef.current;
            setHostKeyPrompt(null);
            hostKeyRetryRef.current = null;
            void invoke('ssh_trust_host_key', {
              host: prompt.host,
              port: prompt.port,
              key: prompt.key,
            })
              .then(() => {
                if (retry) retry();
              })
              .catch((reason) => setError(String(reason)));
          }}
          onCancel={() => {
            setHostKeyPrompt(null);
            hostKeyRetryRef.current = null;
          }}
        />
      )}

      {closeRequest && (
        <PromptDialog
          title="¿Cerrar esta sesión?"
          description="La sesión activa se cerrará y se perderá su estado actual."
          confirmLabel="Cerrar sesión"
          danger
          onConfirm={() => {
            const id = closeRequest;
            setCloseRequest(null);
            finalizeCloseTab(id);
          }}
          onCancel={() => setCloseRequest(null)}
        />
      )}

      {duplicateRequest && (
        <PromptDialog
          title={`Duplicar ${duplicateRequest.name}`}
          description="Se copiarán la configuración y las credenciales guardadas."
          label="Nombre del nuevo perfil"
          initialValue={`${duplicateRequest.name} copia`}
          confirmLabel="Duplicar"
          requireValue
          onConfirm={(name) => {
            const sourceName = duplicateRequest.name;
            setDuplicateRequest(null);
            void invoke('duplicate_profile', { sourceName, name })
              .then(() => refresh())
              .catch((reason) => setError(String(reason)));
          }}
          onCancel={() => setDuplicateRequest(null)}
        />
      )}
    </div>
  );
}

export default App;
