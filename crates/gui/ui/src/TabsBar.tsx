import { useRef } from 'react';
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
  const tabRefs = useRef(new Map<string, HTMLDivElement | null>());

  const jump = (index: number) => {
    const target = tabs[index];
    if (!target) return;
    onSelect(target.id);
    requestAnimationFrame(() => tabRefs.current.get(target.id)?.focus());
  };

  const onKeyDown = (event: React.KeyboardEvent, index: number) => {
    switch (event.key) {
      case 'ArrowRight':
        event.preventDefault();
        jump((index + 1) % tabs.length);
        break;
      case 'ArrowLeft':
        event.preventDefault();
        jump((index - 1 + tabs.length) % tabs.length);
        break;
      case 'Home':
        event.preventDefault();
        jump(0);
        break;
      case 'End':
        event.preventDefault();
        jump(tabs.length - 1);
        break;
    }
  };

  return (
    <div className="tabbar" role="tablist" aria-label="Sesiones abiertas">
      {tabs.map((tab, index) => {
        const name = tabLabel(tab, seen);
        const state =
          tab.kind === 'terminal' ? (tab.connected ? '' : ' (desconectado)') : '';
        const dotClass =
          tab.kind === 'terminal'
            ? tab.connected
              ? 'tab-dot live'
              : 'tab-dot dead'
            : 'tab-dot tool';
        return (
          <div
            key={tab.id}
            ref={(el) => {
              tabRefs.current.set(tab.id, el);
            }}
            role="tab"
            aria-selected={tab.id === activeId}
            tabIndex={tab.id === activeId ? 0 : -1}
            className={`tab ${tab.id === activeId ? 'tab-active' : ''}`}
            aria-label={`${name}${state}`}
            title={`${name}${state}`}
            onClick={() => onSelect(tab.id)}
            onMouseDown={(event) => {
              if (event.button === 1) {
                event.preventDefault();
                onClose(tab.id);
              }
            }}
            onKeyDown={(event) => onKeyDown(event, index)}
          >
            <span className={dotClass} aria-hidden="true" />
            <span className="tab-label">{name}</span>
            <button
              type="button"
              className="tab-close"
              aria-label={`Cerrar ${name}`}
              title={`Cerrar ${name}`}
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
