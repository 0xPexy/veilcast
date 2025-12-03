// Minimal browser-friendly stub for pino used by bb.js deps.
// Provides pino() returning console-like methods.
type Logger = {
  info: (...args: any[]) => void;
  warn: (...args: any[]) => void;
  error: (...args: any[]) => void;
  debug: (...args: any[]) => void;
  trace: (...args: any[]) => void;
  child: () => Logger;
};

export const levels = {
  values: {
    fatal: 60,
    error: 50,
    warn: 40,
    info: 30,
    debug: 20,
    trace: 10,
    silent: Infinity,
  },
};

const base: Logger = {
  info: (...a) => console.info(...a),
  warn: (...a) => console.warn(...a),
  error: (...a) => console.error(...a),
  debug: (...a) => console.debug(...a),
  trace: (...a) => console.trace(...a),
  child: () => base,
};

export const pino = function pinoStub() {
  return base;
};

(pino as any).levels = levels;

export default pino;
