type Props = {
  liveSessions: number;
};

export default function StatusBar({ liveSessions }: Props) {
  return (
    <footer className="statusbar">
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
      <span
        className="status-shortcuts"
        role="note"
        title="Atajos: Ctrl+T nueva sesión · Ctrl+W cerrar pestaña · Ctrl+Tab / Ctrl+PgUp-PgDn cambiar pestaña"
      >
        Ctrl+T nueva · Ctrl+W cerrar · Ctrl+Tab cambiar
      </span>
    </footer>
  );
}
