import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * How we talk to Rust.
 *
 * ## Why two transports
 *
 * In the window, it's `invoke`. But the window has neither hot reload, nor
 * an inspector in production, nor a way to put itself into a chosen state:
 * working on the interface there costs a full build per attempt, and rare
 * states — three missing mods, an installation stopped halfway — can't be
 * triggered.
 *
 * Outside the window, we therefore talk to the development server, which
 * serves the SAME commands over HTTP. The front end then runs in an
 * ordinary browser. See `crates/mc-app/src/dev/`.
 *
 * ## What this file does NOT do
 *
 * It knows no commands. It carries a name and arguments, and returns what
 * comes back. The list of commands lives in `Bridge`, and only there:
 * that's what guarantees both transports serve the same contract.
 */

/** The development server's address. See `dev::PORT`. */
const DEV_SERVER = 'http://127.0.0.1:1421';

/** True in the Tauri window, false in an ordinary browser. */
export const IN_TAURI = isTauri();

/**
 * Calls a command, through whichever transport fits.
 *
 * Errors come back the same way on both sides: `invoke` rejects with the
 * error value — a string — and the development server returns that same
 * string as JSON with a failure code. `errorMessage` therefore handles both
 * without knowing anything about the transport.
 */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (IN_TAURI) {
    return invoke<T>(command, args);
  }

  const response = await fetch(`${DEV_SERVER}/command/${command}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args ?? {}),
  });

  const payload = await response.json();
  if (!response.ok) {
    throw payload;
  }
  return payload as T;
}

/**
 * The development server's event stream — **a single one for everyone**.
 *
 * ## Why it's shared, and what not sharing it cost
 *
 * Each subscription used to open its own SSE connection, and an SSE
 * connection never closes on its own. The launcher opens three — the
 * device code, progress, session opening — and a browser allows only **six
 * simultaneous connections per origin** over HTTP/1.1.
 *
 * Two tabs were therefore enough to consume all six, and every following
 * request waited for a slot that never came. The symptom: the boot screen
 * that never clears, without a single console line — the requests weren't
 * failing, they hadn't left yet.
 *
 * A single stream, demultiplexed by name, brings the cost down to ONE
 * connection regardless of the number of subscribers. It's what the server
 * already does on its side, for that matter: it has only one broadcast
 * channel, and each message carries its own name.
 */
let stream: EventSource | null = null;

/** Who listens to what. A single name can have several subscribers. */
const subscribers = new Map<string, Set<(payload: unknown) => void>>();

function sharedStream(): EventSource {
  if (stream) {
    return stream;
  }
  const source = new EventSource(`${DEV_SERVER}/events`);
  source.onmessage = (message) => {
    try {
      const received = JSON.parse(message.data) as { event: string; payload: unknown };
      for (const receive of subscribers.get(received.event) ?? []) {
        receive(received.payload);
      }
    } catch {
      // A message we can't read doesn't have to break the stream: the next
      // one carries the full state, not a delta.
    }
  };
  stream = source;
  return source;
}

/**
 * Subscribes to an event.
 *
 * In the window, `listen` opens a channel per name. Over HTTP, there's only
 * one stream — see above — and each message carries the name of its event:
 * that's where it gets demultiplexed, so the caller sees no difference.
 */
export async function listen<T>(event: string, receive: (payload: T) => void): Promise<UnlistenFn> {
  if (IN_TAURI) {
    // **The target is OUR window, and that's not a detail.**
    //
    // `listen()` without options registers with `{ kind: 'Any' }`, and on
    // the Rust side `match_any_or_filter` lets through to an `Any` listener
    // ANY event, including ones emitted toward a named window:
    //
    // ```rust
    // *target == EventTarget::Any || filter.map(|f| f(target)).unwrap_or(true)
    // ```
    // (`tauri-2.11.5/src/event/listener.rs`)
    //
    // In other words, `emit_to("main", …)` ALSO arrived in the sign-in
    // window. That's what made it navigate to Spawn and render it in four
    // hundred and forty pixels, two seconds before the real main window
    // showed itself.
    //
    // By targeting our own label, we receive emissions that name us and
    // GLOBAL emissions — `emit()` goes out unfiltered — and nothing else.
    return tauriListen<T>(event, (received) => receive(received.payload), {
      target: getCurrentWindow().label,
    });
  }

  sharedStream();
  const listener = receive as (payload: unknown) => void;
  const forEvent = subscribers.get(event) ?? new Set();
  forEvent.add(listener);
  subscribers.set(event, forEvent);

  return () => {
    forEvent.delete(listener);
    // The connection stays open as long as a subscriber remains, and closes
    // when the last one leaves: reopening it costs a round trip, keeping it
    // open for no one costs one of the browser's six slots.
    if ([...subscribers.values()].every((set) => set.size === 0)) {
      stream?.close();
      stream = null;
    }
  };
}

/**
 * Opens a URL outside the application.
 *
 * In the window, Tauri's plugin; elsewhere, the browser itself — where
 * we're already running.
 */
export async function openOutsideApp(url: string): Promise<void> {
  if (IN_TAURI) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    return openUrl(url);
  }
  window.open(url, '_blank', 'noopener');
}
