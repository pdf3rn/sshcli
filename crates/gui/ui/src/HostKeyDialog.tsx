import { useDialog } from './use-dialog';

type Props = {
  host: string;
  port: number;
  key: string;
  changed: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

export default function HostKeyDialog({
  host,
  port,
  key,
  changed,
  onConfirm,
  onCancel,
}: Props) {
  const dialogRef = useDialog<HTMLDivElement>(onCancel);

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="host-key-dialog-title"
        className="modal prompt-modal"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="host-key-dialog-title">
          {changed ? '¡ADVERTENCIA: host key cambió!' : 'Host key no reconocida'}
        </h2>
        {changed ? (
          <p className="muted small prompt-description">
            La clave del host <strong>{host}:{port}</strong> ha cambiado desde la
            última conexión. Esto puede indicar un <strong>posible ataque de
            intermediario (MITM)</strong> o que el servidor reinstaló sus claves.
            Continúa solo si confías en el servidor actual.
          </p>
        ) : (
          <p className="muted small prompt-description">
            La autenticidad de host <strong>{host}:{port}</strong> no se puede
            establecer. No está en la lista de hosts conocidos. Confía solo si
            esperas conectarte a este servidor por primera vez.
          </p>
        )}
        <div className="field">
          <span>Fingerprint</span>
          <code className="host-key-fingerprint">{key}</code>
        </div>
        <div className="modal-actions">
          <button type="button" className="btn ghost" onClick={onCancel}>
            Cancelar
          </button>
          <button
            type="button"
            className={`btn ${changed ? 'danger' : 'primary'}`}
            onClick={onConfirm}
          >
            {changed ? 'Confiar a pesar del cambio' : 'Confiar y conectar'}
          </button>
        </div>
      </div>
    </div>
  );
}
