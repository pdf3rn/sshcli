import { useState } from 'react';
import { useDialog } from './use-dialog';

type Props = {
  title: string;
  description?: string;
  label?: string;
  initialValue?: string;
  confirmLabel?: string;
  danger?: boolean;
  requireValue?: boolean;
  onConfirm: (value: string) => void;
  onCancel: () => void;
};

export default function PromptDialog({
  title,
  description,
  label,
  initialValue = '',
  confirmLabel = 'Aceptar',
  danger = false,
  requireValue = false,
  onConfirm,
  onCancel,
}: Props) {
  const [value, setValue] = useState(initialValue);
  const dialogRef = useDialog<HTMLDivElement>(onCancel);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const trimmed = value.trim();
    if (requireValue && !trimmed) return;
    onConfirm(trimmed);
  };

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="prompt-dialog-title"
        className="modal prompt-modal"
        onClick={(event) => event.stopPropagation()}
      >
        <form onSubmit={submit} className="prompt-form">
          <h2 id="prompt-dialog-title">{title}</h2>
          {description && <p className="muted small prompt-description">{description}</p>}
          {label && (
            <label className="field">
              <span>{label}</span>
              <input
                autoFocus
                value={value}
                onChange={(event) => setValue(event.target.value)}
                required={requireValue}
              />
            </label>
          )}
          <div className="modal-actions">
            <button type="button" className="btn ghost" onClick={onCancel}>
              Cancelar
            </button>
            <button type="submit" className={`btn ${danger ? 'danger' : 'primary'}`}>
              {confirmLabel}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
