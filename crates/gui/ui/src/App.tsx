import React, { useCallback, useEffect, useRef, useState } from 'react';
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
import TerminalTab, { focusTerminal } from './TerminalTab';
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
  type PaneNode = { tabId: string } | { id: string; dir: 'row' | 'col'; a: PaneNode; b: PaneNode; ratio: number };
  const [split, setSplit] = useState<PaneNode | null>(null);
  const paneIdRef = useRef(0);

  const paneLeaves = (node: PaneNode): string[] =>
    'tabId' in node ? [node.tabId] : [...paneLeaves(node.a), ...paneLeaves(node.b)];

  const containsLeaf = (node: PaneNode, id: string): boolean =>
    'tabId' in node ? node.tabId === id : containsLeaf(node.a, id) || containsLeaf(node.b, id);

  const replaceLeafId = (node: PaneNode, oldId: string, newId: string): PaneNode =>
    'tabId' in node
      ? node.tabId === oldId ? { tabId: newId } : node
      : { ...node, a: replaceLeafId(node.a, oldId, newId), b: replaceLeafId(node.b, oldId, newId) };

  const removeLeaf = (node: PaneNode, id: string): PaneNode | null => {
    if ('tabId' in node) return node.tabId === id ? null : node;
    const a = removeLeaf(node.a, id);
    const b = removeLeaf(node.b, id);
    if (!a) return b;
    if (!b) return a;
    return { ...node, a, b };
  };
  const [draggingShell, setDraggingShell] = useState<string | null>(null);
  const [dropZone, setDropZone] = useState<string | null>(null);
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
  const lastGroupMemberRef = useRef<string | null>(null);

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
    setSplit((value) => {
      if (!value) return null;
      const result = removeLeaf(value, id);
      return result && 'dir' in result ? result : null;
    });
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
          setSplit((value) =>
            value ? replaceLeafId(value, sessionId, info.id) : null,
          );
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
        setSplit((value) =>
          !value
            ? value
            : replaceLeafId(value, sessionId, id),
        );
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
  if (activeTab && (activeTab.kind === 'terminal' || activeTab.kind === 'local')) {
    if (split && containsLeaf(split, activeTab.id)) {
      for (const leafId of paneLeaves(split)) {
        if (shellTabs.some((tab) => tab.id === leafId)) {
          visibleTerminals.add(leafId);
        }
      }
    } else {
      visibleTerminals.add(activeTab.id);
    }
  }

  const canSplit =
    (activeTab?.kind === 'terminal' || activeTab?.kind === 'local') &&
    shellTabs.length >= 2;

  const toggleSplit = () => {
    if (!canSplit) return;
    if (split) {
      setSplit(null);
      return;
    }
    if (activeTabId) {
      const candidate = shellTabs.find((tab) => tab.id !== activeTabId);
      if (candidate) setSplit({ id: `s${++paneIdRef.current}`, dir: 'row', a: { tabId: activeTabId }, b: { tabId: candidate.id }, ratio: 0.5 });
    }
  };

  const dropZoneRef = useRef<string | null>(null);

  type ResizeState = { nodeId: string; dir: 'row' | 'col'; startX: number; startY: number; startRatio: number };
  const resizeRef = useRef<ResizeState | null>(null);
  const [resizing, setResizing] = useState(false);
  const mainRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!resizing) return;
    const onMove = (e: PointerEvent) => {
      const r = resizeRef.current;
      if (!r || !mainRef.current) return;
      const rect = mainRef.current.getBoundingClientRect();
      const isRow = r.dir === 'row';
      const delta = isRow ? e.clientX - r.startX : e.clientY - r.startY;
      const span = isRow ? rect.width : rect.height;
      const newRatio = Math.min(0.85, Math.max(0.15, r.startRatio + delta / span));
      setSplit((current) => {
        if (!current || 'tabId' in current) return current;
        const update = (n: PaneNode): PaneNode =>
          'tabId' in n ? n : n.id === r.nodeId ? { ...n, ratio: newRatio } : { ...n, a: update(n.a), b: update(n.b) };
        return update(current);
      });
    };
    const onUp = () => {
      resizeRef.current = null;
      setResizing(false);
    };
    document.addEventListener('pointermove', onMove);
    document.addEventListener('pointerup', onUp);
    return () => {
      document.removeEventListener('pointermove', onMove);
      document.removeEventListener('pointerup', onUp);
    };
  }, [resizing]);

  const handleDropZone = (zone: string | null) => {
    const id = draggingShell;
    setDraggingShell(null);
    setDropZone(null);
    dropZoneRef.current = null;
    if (!id || !zone || zone === 'center') return;
    const dir: 'row' | 'col' = zone === 'left' || zone === 'right' ? 'row' : 'col';
    setSplit((current) => {
      if (current && !containsLeaf(current, id)) {
        switch (zone) {
          case 'left':
          case 'up':
            return { id: `s${++paneIdRef.current}`, dir, a: { tabId: id }, b: current, ratio: 0.5 };
          default:
            return { id: `s${++paneIdRef.current}`, dir, a: current, b: { tabId: id }, ratio: 0.5 };
        }
      }
      const partner =
        (activeRef.current !== id ? activeRef.current : null) ??
        shellTabs.find((tab) => tab.id !== id)?.id ?? null;
      if (!partner || partner === id) return current;
      const currentDir = current && 'dir' in current ? current.dir : dir;
      const currentRatio = current && 'ratio' in current ? current.ratio : 0.5;
      switch (zone) {
        case 'left':
        case 'up':
          return { id: `s${++paneIdRef.current}`, dir: currentDir, a: { tabId: id }, b: { tabId: partner }, ratio: currentRatio };
        default:
          return { id: `s${++paneIdRef.current}`, dir: currentDir, a: { tabId: partner }, b: { tabId: id }, ratio: currentRatio };
      }
    });
    setActiveTabId(id);
  };

  type ShellTab = { kind: 'terminal'; id: string; profile: string; connected: boolean } | { kind: 'local'; id: string; profile: string; connected: boolean };

  const groupMembers: ShellTab[] | null =
    split && 'dir' in split
      ? paneLeaves(split)
          .map((id) => shellTabs.find((t) => t.id === id))
          .filter((t): t is ShellTab => Boolean(t && (t.kind === 'terminal' || t.kind === 'local')))
      : null;

  if (activeTabId && groupMembers && groupMembers.some((m) => m.id === activeTabId)) {
    lastGroupMemberRef.current = activeTabId;
  }

  const liveSessions = shellTabs.filter((tab) => tab.connected).length;
  const liveProfileNames = new Set(
    shellTabs.filter((tab) => tab.connected).map((tab) => tab.profile),
  );

  type Rect = { left: number; top: number; width: number; height: number };
  const calcRects = (node: PaneNode, left: number, top: number, w: number, h: number): Map<string, Rect> => {
    if ('tabId' in node) return new Map([[node.tabId, { left, top, width: w, height: h }]]);
    const r = node.ratio;
    if (node.dir === 'row') return new Map([...calcRects(node.a, left, top, w * r, h), ...calcRects(node.b, left + w * r, top, w * (1 - r), h)]);
    return new Map([...calcRects(node.a, left, top, w, h * r), ...calcRects(node.b, left, top + h * r, w, h * (1 - r))]);
  };

  const splitVisible = Boolean(split) && visibleTerminals.size > 0;
  const leafRects = splitVisible ? calcRects(split!, 0, 0, 1, 1) : new Map<string, Rect>();

  const renderDividers = (node: PaneNode): React.ReactNode => {
    if ('tabId' in node) return null;
    const r = node.ratio;
    const isRow = node.dir === 'row';
    const style: React.CSSProperties = isRow
      ? { left: `${r * 100}%`, top: 0, width: 7, height: '100%', cursor: 'col-resize' }
      : { top: `${r * 100}%`, left: 0, height: 7, width: '100%', cursor: 'row-resize' };
    return (
      <React.Fragment key={`div-${node.id}`}>
        <div
          className="split-divider"
          style={style}
          onPointerDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            resizeRef.current = { nodeId: node.id, dir: node.dir, startX: e.clientX, startY: e.clientY, startRatio: node.ratio };
            setResizing(true);
          }}
        />
        {renderDividers(node.a)}
        {renderDividers(node.b)}
      </React.Fragment>
    );
  };

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
              onSelect={(id) => {
                setActiveTabId(id);
                focusTerminal(id);
              }}
              onClose={closeTab}
              onReorder={reorderTabs}
              onAdd={() => setNewConnModalOpen(true)}
              onShellDragStart={(id) => {
                setDraggingShell(id);
                dropZoneRef.current = null;
                setDropZone(null);
              }}
              onShellDragEnd={() => {
                setDraggingShell(null);
                setDropZone(null);
                dropZoneRef.current = null;
              }}
              draggingId={draggingShell}
              groupMembers={groupMembers}
              lastGroupMemberId={lastGroupMemberRef.current}
            />
          )}

        <main
          ref={mainRef}
          className="content"
          style={{ position: 'relative' }}
          onDragOver={(event) => {
            if (!draggingShell) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = 'move';
            const zone =
              (event.target as HTMLElement).closest<HTMLElement>('[data-dropzone]')
                ?.dataset.dropzone ?? null;
            if (zone !== dropZoneRef.current) {
              dropZoneRef.current = zone;
              setDropZone(zone);
            }
          }}
          onDrop={(event) => {
            if (!draggingShell) return;
            event.preventDefault();
            const rect = event.currentTarget.getBoundingClientRect();
            const x = event.clientX - rect.left;
            const y = event.clientY - rect.top;
            let zone: string | null = null;
            if (x < rect.width / 3) {
              zone = 'left';
            } else if (x > (rect.width * 2) / 3) {
              zone = 'right';
            } else {
              zone = y < rect.height / 2 ? 'up' : 'down';
            }
            handleDropZone(zone);
          }}
        >
          {shellTabs.map((tab) => {
            if (tab.kind !== 'terminal' && tab.kind !== 'local') return null;
            const isActive = activeTab?.id === tab.id;
            const visible = visibleTerminals.has(tab.id);
            const rect = leafRects.get(tab.id);
            const inSplit = Boolean(rect);
            return (
              <div
                key={tab.id}
                className="pane-slot"
                style={{
                  display: visible ? undefined : 'none',
                  ...(inSplit
                    ? {
                        position: 'absolute' as const,
                        left: `${rect!.left * 100}%`,
                        top: `${rect!.top * 100}%`,
                        width: `${rect!.width * 100}%`,
                        height: `${rect!.height * 100}%`,
                      }
                    : {}),
                }}
                onClick={() => { setActiveTabId(tab.id); focusTerminal(tab.id); }}
              >
                {split && containsLeaf(split, tab.id) ? (
                  <div className="pane-head">
                    <span className={`tab-dot ${tab.connected ? 'live' : 'dead'}`} aria-hidden="true" />
                    <span className="pane-title">{tab.profile}</span>
                    <button
                      type="button"
                      className="pane-close"
                      aria-label={`Cerrar ${tab.profile}`}
                      onClick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
                    >
                      ✕
                    </button>
                  </div>
                ) : null}
                <TerminalTab
                  sessionId={tab.id}
                  connected={tab.connected}
                  visible={visible}
                  prefs={prefs}
                  transport={tab.kind === 'local' ? 'local' : 'ssh'}
                  onClose={closeTab}
                  onReconnect={reconnect}
                  onCwd={rememberCwd}
                />
              </div>
            );
          })}

          {split && splitVisible && renderDividers(split)}

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

          {draggingShell && view === 'session' && shellTabs.length > 1 && (
            <div className="drop-overlay" aria-hidden="true">
              <div
                data-dropzone="left"
                className={`drop-zone z-left ${dropZone === 'left' ? 'hover' : ''}`}
              >
                Izquierda
              </div>
              <div
                data-dropzone="right"
                className={`drop-zone z-right ${dropZone === 'right' ? 'hover' : ''}`}
              >
                Derecha
              </div>
              <div
                data-dropzone="up"
                className={`drop-zone z-up ${dropZone === 'up' ? 'hover' : ''}`}
              >
                Arriba
              </div>
              <div
                data-dropzone="down"
                className={`drop-zone z-down ${dropZone === 'down' ? 'hover' : ''}`}
              >
                Abajo
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
        splitActive={split !== null}
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
