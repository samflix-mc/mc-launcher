import { Injectable, inject, signal } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { Bridge } from './bridge';

/**
 * The title bar buttons.
 *
 * ## Why this service exists
 *
 * `decorations: false` removes the system bar: minimize, maximize and close
 * no longer have a button, and it's up to us to give them back. The
 * corresponding permissions are NOT in `core:window:default` — they're
 * declared by name in `capabilities/default.json`, and their absence shows up
 * as an ACL refusal you only see in a packaged build.
 *
 * ## What we did NOT have to write
 *
 * Resizing from the edges. `tauri-runtime-wry` wires up its own handler on
 * the WebView under Linux, with a five-pixel band multiplied by the scale
 * factor. Adding DOM zones for it would duplicate work, and fight with it.
 *
 * The trade-off is a LAYOUT CONSTRAINT, not code: no click target starts
 * within eight pixels of an edge. Without it, dragging the window from the
 * top resizes it instead of moving it — the GTK handler runs before WebKit
 * dispatches the `mousedown`.
 */
@Injectable({ providedIn: 'root' })
export class WindowService {
  private readonly bridge = inject(Bridge);

  /** Is the window maximized? To change the button's icon. */
  readonly maximized = signal(false);

  /**
   * The label of the window this front runs in, or `null` outside Tauri.
   *
   * The launcher opens two that load the SAME Angular application: "main"
   * and "signin". They don't do the same job — one opens the sign-in window
   * when the session is missing, the other IS that window — and nothing in
   * the URL tells them apart, since they share the route.
   *
   * Read once, at startup: a label doesn't change.
   */
  readonly label: string | null = this.bridge.inWindow ? getCurrentWindow().label : null;

  /** Are we in the main window? False in the sign-in one. */
  readonly isMain = this.label === 'main';

  /**
   * Are we in a window the launcher closes on its own?
   *
   * The sign-in window is one: when the session opens, Rust closes it and
   * shows the main one. Navigating elsewhere in it would amount to drawing
   * Spawn in a four-hundred-and-forty-pixel window for however long it takes
   * to disappear.
   */
  readonly inADedicatedWindow = this.label !== null && this.label !== 'main';

  async minimize(): Promise<void> {
    if (!this.bridge.inWindow) {
      return;
    }
    await getCurrentWindow().minimize();
  }

  async toggleMaximized(): Promise<void> {
    if (!this.bridge.inWindow) {
      return;
    }
    const win = getCurrentWindow();
    await win.toggleMaximize();
    this.maximized.set(await win.isMaximized());
  }

  async close(): Promise<void> {
    if (!this.bridge.inWindow) {
      return;
    }
    await getCurrentWindow().close();
  }

  /** Re-reads the state at startup: the window can open already maximized. */
  async watch(): Promise<void> {
    if (!this.bridge.inWindow) {
      return;
    }
    this.maximized.set(await getCurrentWindow().isMaximized());
  }
}
