import { useCallback, useState } from 'react';

export type CursorStyle = 'block' | 'underline' | 'bar';

export type Prefs = {
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  scrollback: number;
  copyOnSelect: boolean;
  rightClickPaste: boolean;
  cursorStyle: CursorStyle;
  cursorBlink: boolean;
  telemetryEnabled: boolean;
  telemetryPanelOpen: boolean;
  remoteExplorerEnabled: boolean;
  remoteExplorerOpen: boolean;
  localShell: string;
};

export const FONT_OPTIONS = [
  { label: 'JetBrains Mono', value: "'JetBrains Mono', monospace" },
  { label: 'Fira Code', value: "'Fira Code', monospace" },
  { label: 'Cascadia Mono', value: "'Cascadia Mono', monospace" },
  { label: 'Menlo / Monaco', value: "Menlo, Monaco, monospace" },
  { label: 'Mono del sistema', value: 'monospace' },
];

export const LINE_HEIGHTS = [
  { label: 'Compacta (1.0)', value: 1 },
  { label: 'Normal (1.2)', value: 1.2 },
  { label: 'Relajada (1.5)', value: 1.5 },
];

export const SCROLLBACK_OPTIONS = [
  { label: '1.000 líneas', value: 1000 },
  { label: '5.000 líneas', value: 5000 },
  { label: '10.000 líneas', value: 10000 },
  { label: 'Sin límite', value: 1000000 },
];

export const DEFAULT_PREFS: Prefs = {
  fontFamily: "'JetBrains Mono', monospace",
  fontSize: 13,
  lineHeight: 1,
  scrollback: 5000,
  copyOnSelect: false,
  rightClickPaste: false,
  cursorStyle: 'block',
  cursorBlink: true,
  telemetryEnabled: false,
  telemetryPanelOpen: false,
  remoteExplorerEnabled: false,
  remoteExplorerOpen: false,
  localShell: '',
};

const STORAGE_KEY = 'sshcli.prefs.v1';

export function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_PREFS;
    return { ...DEFAULT_PREFS, ...(JSON.parse(raw) as Partial<Prefs>) };
  } catch {
    return DEFAULT_PREFS;
  }
}

export function usePrefs(): [Prefs, (patch: Partial<Prefs>) => void] {
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const updatePrefs = useCallback((patch: Partial<Prefs>) => {
    setPrefs((current) => {
      const next = { ...current, ...patch };
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      } catch {
        return current;
      }
      return next;
    });
  }, []);
  return [prefs, updatePrefs];
}
