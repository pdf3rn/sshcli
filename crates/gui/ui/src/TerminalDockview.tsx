import { useEffect, useMemo, useRef } from 'react';
import {
  DockviewReact,
  type DockviewApi,
  type DockviewReadyEvent,
  type IDockviewHeaderActionsProps,
  type IDockviewPanelProps,
} from 'dockview-react';
import 'dockview-react/dist/styles/dockview.css';
import RemoteExplorerPanel from './RemoteExplorerPanel';
import SftpPanel from './SftpPanel';
import TelemetryPanel from './TelemetryPanel';
import TerminalTab, { focusTerminal } from './TerminalTab';
import TunnelPanel from './TunnelPanel';
import { PlusIcon } from './icons';
import type { Prefs } from './prefs';
import type { Tab } from './types';

type DockParams = {
  tab: Tab;
  prefs: Prefs;
  cwd?: string;
  onAdd: () => void;
  onClose: (id: string) => void;
  onReconnect: (id: string) => void;
  onCwd: (id: string, path: string) => void;
};

type Props = {
  tabs: Tab[];
  activeId: string | null;
  prefs: Prefs;
  tabCwds: Record<string, string>;
  onAdd: () => void;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onReconnect: (id: string) => void;
  onCwd: (id: string, path: string) => void;
  onDockviewReady: (actions: DockviewActions | null) => void;
  onLayoutChange: (groupCount: number) => void;
};

export type DockviewActions = {
  toggleSplit: () => void;
};

function panelTitle(tab: Tab): string {
  if (tab.kind === 'sftp') return `${tab.profile} · SFTP`;
  if (tab.kind === 'tunnels') return `${tab.profile} · Túneles`;
  return tab.profile;
}

function DockPanel({ params }: IDockviewPanelProps<DockParams>) {
  const { tab, prefs, cwd, onClose, onReconnect, onCwd } = params;
  if (tab.kind === 'terminal' || tab.kind === 'local') {
    const transport = tab.kind === 'local' ? 'local' : 'ssh';
    return (
      <div className="dock-panel-shell">
        <TerminalTab
          sessionId={tab.id}
          connected={tab.connected}
          visible
          prefs={prefs}
          transport={transport}
          onClose={onClose}
          onReconnect={onReconnect}
          onCwd={onCwd}
        />
        {tab.kind === 'terminal' && prefs.telemetryEnabled && prefs.telemetryPanelOpen && (
          <TelemetryPanel profile={tab.profile} />
        )}
        {tab.kind === 'terminal' && prefs.remoteExplorerEnabled && prefs.remoteExplorerOpen && (
          <RemoteExplorerPanel sessionId={tab.id} cwd={cwd} />
        )}
      </div>
    );
  }
  if (tab.kind === 'sftp') return <SftpPanel profile={tab.profile} />;
  return <TunnelPanel profile={tab.profile} onClose={() => onClose(tab.id)} />;
}

function HeaderActions({ panels }: IDockviewHeaderActionsProps) {
  const params = panels[0]?.params as DockParams | undefined;
  return (
    <button
      type="button"
      className="dockview-add"
      aria-label="Nueva conexión"
      title="Nueva conexión"
      onClick={() => params?.onAdd()}
    >
      <PlusIcon size={14} />
    </button>
  );
}

export default function TerminalDockview({
  tabs,
  activeId,
  prefs,
  tabCwds,
  onAdd,
  onSelect,
  onClose,
  onReconnect,
  onCwd,
  onDockviewReady,
  onLayoutChange,
}: Props) {
  const apiRef = useRef<DockviewApi | null>(null);
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;

  const makeParams = (tab: Tab): DockParams => ({
    tab,
    prefs,
    cwd: tabCwds[tab.id],
    onAdd,
    onClose,
    onReconnect,
    onCwd,
  });

  const components = useMemo(() => ({ session: DockPanel }), []);

  const syncPanels = (api: DockviewApi) => {
    const ids = new Set(tabs.map((tab) => tab.id));
    for (const tab of tabs) {
      const existing = api.getPanel(tab.id);
      if (existing) {
        existing.setTitle(panelTitle(tab));
        existing.api.updateParameters(makeParams(tab));
      } else {
        api.addPanel({
          id: tab.id,
          title: panelTitle(tab),
          component: 'session',
          params: makeParams(tab),
          renderer: 'always',
          inactive: tab.id !== activeId,
          minimumWidth: 240,
          minimumHeight: 140,
        });
      }
    }
    for (const panel of api.panels) {
      if (!ids.has(panel.id)) api.removePanel(panel);
    }
    if (activeId) api.getPanel(activeId)?.api.setActive();
  };

  useEffect(() => {
    const api = apiRef.current;
    if (!api) return;
    syncPanels(api);
  }, [tabs, activeId, prefs, tabCwds]);

  const onReady = (event: DockviewReadyEvent) => {
    apiRef.current = event.api;
    syncPanels(event.api);
    const reportLayout = () => onLayoutChange(event.api.groups.length);
    onDockviewReady({
      toggleSplit: () => {
        const active = event.api.activePanel;
        if (!active) return;
        if (event.api.groups.length > 1) {
          const targetGroup = active.api.group;
          for (const panel of event.api.panels) {
            if (panel.api.group !== targetGroup) {
              panel.api.moveTo({ group: targetGroup, position: 'center', skipSetActive: true });
            }
          }
        } else {
          active.api.moveTo({ group: active.api.group, position: 'right' });
        }
        reportLayout();
      },
    });
    reportLayout();
    event.api.onDidLayoutChange(reportLayout);
    event.api.onDidActivePanelChange((active) => {
      const id = active.panel?.id;
      if (!id || !tabsRef.current.some((tab) => tab.id === id)) return;
      onSelect(id);
      focusTerminal(id);
    });
    event.api.onDidRemovePanel((panel) => {
      if (tabsRef.current.some((tab) => tab.id === panel.id)) onClose(panel.id);
    });
  };

  return (
    <DockviewReact
      className="dockview-theme-abyss sshcli-dockview"
      components={components}
      dndStrategy="pointer"
      leftHeaderActionsComponent={HeaderActions}
      onReady={onReady}
    />
  );
}
