import {
  FONT_OPTIONS,
  LINE_HEIGHTS,
  SCROLLBACK_OPTIONS,
  type CursorStyle,
  type Prefs,
} from './prefs';
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Props = {
  prefs: Prefs;
  onChange: (patch: Partial<Prefs>) => void;
};

function Section({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-section">
      <h3 className="panel-label">{title}</h3>
      {description && <p className="muted small section-description">{description}</p>}
      <div className="settings-fields">{children}</div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="settings-field">
      <span className="field-label">{label}</span>
      {children}
    </label>
  );
}

function Segmented<T extends string | number>({
  value,
  options,
  ariaLabel,
  onSelect,
}: {
  value: T;
  options: Array<{ label: string; value: T }>;
  ariaLabel: string;
  onSelect: (value: T) => void;
}) {
  return (
    <div className="segmented" role="group" aria-label={ariaLabel}>
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          aria-pressed={value === option.value}
          onClick={() => onSelect(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

const CURSOR_CHOICES: Array<{ label: string; value: CursorStyle }> = [
  { label: '▮ Bloque', value: 'block' },
  { label: '_ Subrayado', value: 'underline' },
  { label: '| Barra', value: 'bar' },
];

export default function SettingsView({ prefs, onChange }: Props) {
  return (
    <section className="settings-view" aria-labelledby="settings-title">
      <div className="view-header">
        <div>
          <h2 id="settings-title">Ajustes</h2>
          <p className="muted">
            Tipografía, comportamiento del terminal y telemetría. Los cambios se aplican al
            instante en todas las sesiones.
          </p>
        </div>
      </div>

      <div className="settings-layout">
        <Section title="Tipografía">
          <Field label="Fuente">
            <select
              value={prefs.fontFamily}
              onChange={(event) => onChange({ fontFamily: event.target.value })}
            >
              {FONT_OPTIONS.map((font) => (
                <option key={font.label} value={font.value}>
                  {font.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label={`Tamaño (${prefs.fontSize}px)`}>
            <input
              type="range"
              min={10}
              max={24}
              step={1}
              value={prefs.fontSize}
              aria-label="Tamaño de fuente"
              onChange={(event) => onChange({ fontSize: Number(event.target.value) })}
            />
          </Field>
          <Field label="Interlineado">
            <Segmented
              ariaLabel="Interlineado"
              value={prefs.lineHeight}
              options={LINE_HEIGHTS}
              onSelect={(value) => onChange({ lineHeight: value })}
            />
          </Field>
        </Section>

        <Section title="Comportamiento">
          <Field label="Líneas de historial (scrollback)">
            <select
              value={prefs.scrollback}
              onChange={(event) => onChange({ scrollback: Number(event.target.value) })}
            >
              {SCROLLBACK_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </Field>
          <label className="check">
            <input
              type="checkbox"
              checked={prefs.copyOnSelect}
              onChange={(event) => onChange({ copyOnSelect: event.target.checked })}
            />
            <span>Copiar al seleccionar</span>
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={prefs.rightClickPaste}
              onChange={(event) => onChange({ rightClickPaste: event.target.checked })}
            />
            <span>Pegar con clic derecho</span>
          </label>
        </Section>

        <Section title="Atajos de teclado" description="Los atajos usan Ctrl en Windows/Linux y Cmd en macOS.">
          <div className="shortcut-list">
            <div className="shortcut-row">
              <span>Cambiar a la pestaña 1–9</span>
              <kbd>Ctrl/Cmd + 1–9</kbd>
            </div>
            <div className="shortcut-row">
              <span>Cambiar de pestaña</span>
              <kbd>Ctrl/Cmd + Tab</kbd>
            </div>
            <div className="shortcut-row">
              <span>Cerrar pestaña activa</span>
              <kbd>Ctrl/Cmd + W</kbd>
            </div>
            <div className="shortcut-row">
              <span>Buscar en terminal</span>
              <kbd>Ctrl/Cmd + F</kbd>
            </div>
            <div className="shortcut-row">
              <span>Copiar selección de terminal</span>
              <kbd>Ctrl/Cmd + Shift + C</kbd>
            </div>
            <div className="shortcut-row">
              <span>Limpiar terminal</span>
              <kbd>Ctrl/Cmd + Shift + K</kbd>
            </div>
          </div>
        </Section>

        <Section title="Cursor">
          <Field label="Estilo">
            <Segmented
              ariaLabel="Estilo del cursor"
              value={prefs.cursorStyle}
              options={CURSOR_CHOICES}
              onSelect={(value) => onChange({ cursorStyle: value })}
            />
          </Field>
          <label className="check">
            <input
              type="checkbox"
              checked={prefs.cursorBlink}
              onChange={(event) => onChange({ cursorBlink: event.target.checked })}
            />
            <span>Cursor parpadeante</span>
          </label>
        </Section>

        <Section
          title="Tema"
          description="Graphite Terminal es el tema integrado; los colores del terminal lo siguen automáticamente."
        >
          <div className="theme-chip-row">
            <span className="theme-chip">
              <span className="theme-swatch" aria-hidden="true" />
              Graphite Terminal
            </span>
          </div>
          <div className="term-preview" style={{ fontFamily: prefs.fontFamily }}>
            <span className="preview-prompt">pedro@linuxpc</span>:~${' '}
            <span className="preview-cmd">neofetch --stdout</span>
            {'\n'}OS: sshcli desktop · Theme: graphite{'\n'}
            <span className="preview-prompt">pedro@linuxpc</span>:~$ ▮
          </div>
        </Section>

        <Section title="Telemetría del host">
          <label className="check">
            <input
              type="checkbox"
              checked={prefs.telemetryEnabled}
              onChange={(event) => onChange({ telemetryEnabled: event.target.checked })}
            />
            <span>Mostrar panel de telemetría en las sesiones</span>
          </label>
          <p className="muted small section-note">
            Desactivada por defecto. Al activarla, sshcli ejecuta lecturas de{' '}
            <code>/proc</code> en el host remoto cada 3 segundos mientras el panel esté visible.
          </p>
        </Section>

        <Section title="Explorador remoto">
          <label className="check">
            <input
              type="checkbox"
              checked={prefs.remoteExplorerEnabled}
              onChange={(event) => onChange({ remoteExplorerEnabled: event.target.checked })}
            />
            <span>Mostrar explorador de carpetas en las sesiones</span>
          </label>
          <p className="muted small section-note">
            Desactivado por defecto. Muestra un árbol del directorio actual del shell y sigue los{' '}
            <code>cd</code>. Requiere que tu shell emita la secuencia OSC 7; el propio panel te da{' '}
            el snippet para configurarlo si falta.
          </p>
        </Section>

        <LocalShellSection prefs={prefs} onChange={onChange} />
      </div>
    </section>
  );
}

function LocalShellSection({
  prefs,
  onChange,
}: {
  prefs: Prefs;
  onChange: (patch: Partial<Prefs>) => void;
}) {
  const [detected, setDetected] = useState('');
  const [available, setAvailable] = useState<string[]>([]);

  useEffect(() => {
    invoke<{ detected: string; available: string[] }>('local_shell_detect')
      .then((info) => {
        setDetected(info.detected);
        setAvailable(info.available);
      })
      .catch(() => undefined);
  }, []);

  return (
    <Section title="Terminal local">
      <Field label="Intérprete">
        <select
          value={prefs.localShell}
          onChange={(event) => onChange({ localShell: event.target.value })}
        >
          <option value="">
            Automático{detected ? ` (${detected.split('/').pop()})` : ''}
          </option>
          {available.map((shell) => (
            <option key={shell} value={shell}>
              {shell}
            </option>
          ))}
        </select>
      </Field>
      <p className="muted small section-note">
        Se abre como shell de login con un PTY nativo en tu equipo. «Automático» usa la
        detección ($SHELL → passwd → intérpretes instalados).
      </p>
    </Section>
  );
}
