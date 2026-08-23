import type { View } from './types';

type Props = {
  view: View;
  liveSessions: number;
  onNavigate: (view: View) => void;
};

const ITEMS: Array<{ id: View; label: string }> = [
  { id: 'home', label: 'Inicio' },
  { id: 'connections', label: 'Conexiones' },
  { id: 'session', label: 'Sesión' },
  { id: 'settings', label: 'Ajustes' },
];

export default function TopBar({ view, liveSessions, onNavigate }: Props) {
  return (
    <header className="app-topbar">
      <h1 className="brand-top">sshcli</h1>
      <nav className="topbar-nav" aria-label="Principal">
        {ITEMS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`nav-btn ${view === item.id ? 'nav-active' : ''}`}
            aria-current={view === item.id ? 'page' : undefined}
            onClick={() => onNavigate(item.id)}
          >
            {item.label}
            {item.id === 'session' && liveSessions > 0 && (
              <span className="nav-badge" aria-label={`${liveSessions} sesiones activas`}>
                {liveSessions}
              </span>
            )}
          </button>
        ))}
      </nav>
    </header>
  );
}
