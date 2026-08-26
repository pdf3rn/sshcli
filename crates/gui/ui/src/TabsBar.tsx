import { useRef, useMemo } from 'react';
import type { Tab } from './types';
import { PlusIcon } from './icons';

type ShellTab = { kind: 'terminal'; id: string; profile: string; connected: boolean } | { kind: 'local'; id: string; profile: string; connected: boolean };

type Props = {
  tabs: Tab[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onReorder: (dragId: string, targetId: string) => void;
  onAdd: () => void;
  onShellDragStart?: (id: string) => void;
  onShellDragEnd?: () => void;
  draggingId?: string | null;
  groupMembers?: ShellTab[] | null;
  lastGroupMemberId?: string | null;
};

type BarItem =
  | { kind: 'tab'; tab: Tab; key: string }
  | { kind: 'group'; members: ShellTab[]; key: string };

function tabLabel(tab: Tab, seen: Map<string, number>): string {
  if (tab.kind === 'sftp' || tab.kind === 'tunnels') {
    return `${tab.profile} · ${tab.kind === 'sftp' ? 'SFTP' : 'Túneles'}`;
  }
  const count = (seen.get(tab.profile) ?? 0) + 1;
  seen.set(tab.profile, count);
  return count === 1 ? tab.profile : `${tab.profile} ·${count}`;
}

export default function TabsBar({
  tabs,
  activeId,
  onSelect,
  onClose,
  onReorder,
  onAdd,
  onShellDragStart,
  onShellDragEnd,
  draggingId,
  groupMembers,
  lastGroupMemberId,
}: Props) {
  const seen = new Map<string, number>();
  const tabRefs = useRef(new Map<string, HTMLDivElement | null>());
  const dragId = useRef<string | null>(null);

  const groupIds = useMemo(
    () => (groupMembers && groupMembers.length > 1 ? new Set(groupMembers.map((t) => t.id)) : null),
    [groupMembers],
  );

  const items: BarItem[] = useMemo(() => {
    if (!groupIds || !groupMembers) return tabs.map((t) => ({ kind: 'tab' as const, tab: t, key: t.id }));
    const result: BarItem[] = [];
    let inserted = false;
    for (const tab of tabs) {
      if (groupIds.has(tab.id)) {
        if (!inserted) {
          result.push({ kind: 'group', members: groupMembers, key: `group:${groupMembers[0].id}` });
          inserted = true;
        }
        continue;
      }
      result.push({ kind: 'tab', tab, key: tab.id });
    }
    return result;
  }, [tabs, groupIds, groupMembers]);

  const groupTargetId = (members: ShellTab[]): string => {
    if (lastGroupMemberId && members.some((m) => m.id === lastGroupMemberId)) return lastGroupMemberId;
    return members[0].id;
  };

  const jump = (index: number) => {
    const item = items[index];
    if (!item) return;
    const targetId = item.kind === 'group' ? groupTargetId(item.members) : item.tab.id;
    onSelect(targetId);
    requestAnimationFrame(() => tabRefs.current.get(item.key)?.focus());
  };

  const move = (item: BarItem, delta: number) => {
    if (item.kind === 'group') return;
    const from = items.findIndex((i) => i.kind === 'tab' && i.tab.id === item.tab.id);
    const to = from + delta;
    if (from === -1 || to < 0 || to >= items.length) return;
    const target = items[to];
    if (target.kind === 'group') return;
    onReorder(item.tab.id, target.tab.id);
  };

  const onKeyDown = (event: React.KeyboardEvent, index: number) => {
    const current = items[index];
    if (event.altKey && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
      event.preventDefault();
      event.stopPropagation();
      move(current, event.key === 'ArrowLeft' ? -1 : 1);
      return;
    }
    switch (event.key) {
      case 'ArrowRight':
        event.preventDefault();
        jump((index + 1) % items.length);
        break;
      case 'ArrowLeft':
        event.preventDefault();
        jump((index - 1 + items.length) % items.length);
        break;
      case 'Home':
        event.preventDefault();
        jump(0);
        break;
      case 'End':
        event.preventDefault();
        jump(items.length - 1);
        break;
    }
  };

  return (
    <div className="tabbar" role="tablist" aria-label="Sesiones abiertas">
      {items.map((item, index) => {
        if (item.kind === 'group') {
          const isActive = item.members.some((m) => m.id === activeId);
          const label = item.members.map((m) => m.profile).join(' | ');
          return (
            <div
              key={item.key}
              ref={(el) => { tabRefs.current.set(item.key, el); }}
              role="tab"
              aria-selected={isActive}
              tabIndex={isActive ? 0 : -1}
              className={`tab tab-group ${isActive ? 'tab-active' : ''}`}
              aria-label={`Grupo: ${label}`}
              title={`Grupo: ${label}`}
              onClick={() => onSelect(groupTargetId(item.members))}
              onKeyDown={(event) => onKeyDown(event, index)}
            >
              <span className="tab-group-glyph" aria-hidden="true">⧉</span>
              <span className="tab-label">{label}</span>
              <button
                type="button"
                className="tab-close"
                aria-label={`Cerrar grupo ${label}`}
                title={`Cerrar grupo ${label}`}
                onClick={(event) => {
                  event.stopPropagation();
                  onClose(groupTargetId(item.members));
                }}
              >
                ✕
              </button>
            </div>
          );
        }

        const tab = item.tab;
        const name = tabLabel(tab, seen);
        const isShell = tab.kind === 'terminal' || tab.kind === 'local';
        const state = isShell ? (tab.connected ? '' : ' (desconectado)') : '';
        const dotClass = isShell
          ? tab.connected
            ? 'tab-dot live'
            : 'tab-dot dead'
          : 'tab-dot tool';
        return (
          <div
            key={tab.id}
            ref={(el) => { tabRefs.current.set(tab.id, el); }}
            role="tab"
            aria-selected={tab.id === activeId}
            tabIndex={tab.id === activeId ? 0 : -1}
            className={`tab ${tab.id === activeId ? 'tab-active' : ''} ${draggingId === tab.id ? 'dragging' : ''}`}
            aria-label={`${name}${state}`}
            title={`${name}${state}`}
            draggable
            onDragStart={(event) => {
              dragId.current = tab.id;
              event.dataTransfer.effectAllowed = 'move';
              event.dataTransfer.setData('text/plain', tab.id);
              if (tab.id !== activeId && (tab.kind === 'terminal' || tab.kind === 'local')) {
                onShellDragStart?.(tab.id);
              }
            }}
            onDragEnd={() => {
              dragId.current = null;
              if (tab.kind === 'terminal' || tab.kind === 'local') {
                onShellDragEnd?.();
              }
            }}
            onDragOver={(event) => {
              if (!dragId.current || dragId.current === tab.id) return;
              event.preventDefault();
              event.dataTransfer.dropEffect = 'move';
              event.currentTarget.classList.add('drop-target');
            }}
            onDragLeave={(event) => {
              event.currentTarget.classList.remove('drop-target');
            }}
            onDrop={(event) => {
              event.preventDefault();
              event.currentTarget.classList.remove('drop-target');
              if (dragId.current && dragId.current !== tab.id) onReorder(dragId.current, tab.id);
              dragId.current = null;
            }}
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
      <button
        type="button"
        className="tab-add"
        aria-label="Nueva conexión"
        title="Nueva conexión"
        onClick={onAdd}
      >
        <PlusIcon size={14} />
      </button>
    </div>
  );
}
