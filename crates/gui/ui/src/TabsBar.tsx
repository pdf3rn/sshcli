import type { Tab } from './types';

type Props = {
  tabs: Tab[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
};

function tabLabel(tab: Tab, seen: Map<string, number>): string {
  if (tab.kind !== 'terminal') return `${tab.profile} · ${tab.kind === 'sftp' ? 'SFTP' : 'Túneles'}`;
  const count = (seen.get(tab.profile) ?? 0) + 1;
  seen.set(tab.profile, count);
  return count === 1 ? tab.profile : `${tab.profile} ·${count}`;
}

export default function TabsBar({ tabs, activeId, onSelect, onClose }: Props) {
  const seen = new Map<string, number>();

  return (
    <div className="tabbar">
      {tabs.map((tab) => {
        const dotClass =
          tab.kind === 'terminal'
            ? tab.connected
              ? 'tab-dot live'
              : 'tab-dot dead'
            : 'tab-dot tool';
        return (
          <div
            key={tab.id}
            className={`tab ${tab.id === activeId ? 'tab-active' : ''}`}
            onClick={() => onSelect(tab.id)}
            onMouseDown={(event) => {
              if (event.button === 1) {
                event.preventDefault();
                onClose(tab.id);
              }
            }}
            title={
              tab.kind === 'terminal'
                ? `${tab.profile}${tab.connected ? '' : ' (desconectado)'}`
                : tabLabel(tab, new Map())
            }
          >
            <span className={dotClass} />
            <span className="tab-label">{tabLabel(tab, seen)}</span>
            <button
              className="tab-close"
              title="Cerrar pestaña"
              onClick={(event) => {
                event.stopPropagation();
                onClose(tab.id);
              }}
            >
              ✕
            </button>
          </div>
        );
      })}
    </div>
  );
}
