import { ActivityIcon, ColumnsIcon, FolderIcon } from './icons';

type Props = {
  liveSessions: number;
  connecting: boolean;
  canSplit: boolean;
  splitActive: boolean;
  onToggleSplit: () => void;
  telemetryAvailable: boolean;
  telemetryOpen: boolean;
  onToggleTelemetry: () => void;
  explorerAvailable: boolean;
  explorerOpen: boolean;
  onToggleExplorer: () => void;
};

export default function StatusBar({
  liveSessions,
  connecting,
  canSplit,
  splitActive,
  onToggleSplit,
  telemetryAvailable,
  telemetryOpen,
  onToggleTelemetry,
  explorerAvailable,
  explorerOpen,
  onToggleExplorer,
}: Props) {
  const splitLabel = canSplit
    ? splitActive
      ? 'Volver a un solo panel'
      : 'Dividir en dos paneles'
    : 'Abre al menos dos sesiones para dividir';
  return (
    <footer className="statusbar">
      <div className="statusbar-left">
        <span className="status-item">
          <span
            className={`status-dot ${liveSessions > 0 ? 'ok' : ''}`}
            aria-hidden="true"
          />
          <span>
            {liveSessions === 0
              ? 'Sin sesiones'
              : `${liveSessions} ${liveSessions === 1 ? 'sesión activa' : 'sesiones activas'}`}
          </span>
        </span>
        {connecting && (
          <span className="muted small" role="status">
            Conectando…
          </span>
        )}
      </div>
      <div className="statusbar-right">
        <button
          type="button"
          className={`icon-btn ${splitActive ? 'on' : ''}`}
          disabled={!canSplit}
          title={splitLabel}
          aria-label={splitLabel}
          onClick={onToggleSplit}
        >
          <ColumnsIcon />
        </button>
        {explorerAvailable && (
          <button
            type="button"
            className={`icon-btn telemetry-toggle ${explorerOpen ? 'on' : ''}`}
            aria-pressed={explorerOpen}
            title={explorerOpen ? 'Ocultar explorador remoto' : 'Mostrar explorador remoto'}
            aria-label={explorerOpen ? 'Ocultar explorador remoto' : 'Mostrar explorador remoto'}
            onClick={onToggleExplorer}
          >
            <FolderIcon />
          </button>
        )}
        {telemetryAvailable && (
          <button
            type="button"
            className={`icon-btn telemetry-toggle ${telemetryOpen ? 'on' : ''}`}
            aria-pressed={telemetryOpen}
            title={telemetryOpen ? 'Ocultar telemetría' : 'Mostrar telemetría'}
            aria-label={telemetryOpen ? 'Ocultar telemetría' : 'Mostrar telemetría'}
            onClick={onToggleTelemetry}
          >
            <ActivityIcon />
          </button>
        )}
      </div>
    </footer>
  );
}
