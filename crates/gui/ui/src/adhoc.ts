export const PASSWORD_REQUIRED = 'sshcli:password-required';

export type AdhocOutcome =
  | { status: 'ok' }
  | { status: 'password-required' }
  | { status: 'error'; message: string };

export function isValidAdhocTarget(target: string): boolean {
  return target.includes('@') && !target.startsWith('@') && !target.endsWith('@');
}

export async function attemptAdhoc(
  run: () => Promise<void>,
  hadPassword: boolean,
): Promise<AdhocOutcome> {
  try {
    await run();
    return { status: 'ok' };
  } catch (reason) {
    const message = String(reason);
    if (!message.includes(PASSWORD_REQUIRED)) {
      return { status: 'error', message };
    }
    if (hadPassword) {
      return { status: 'error', message: 'Contraseña incorrecta o credenciales rechazadas.' };
    }
    return { status: 'password-required' };
  }
}
