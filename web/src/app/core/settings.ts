import { Injectable, inject, signal } from '@angular/core';

import type { Screen, Settings as SettingsView } from './contracts';
import { Bridge } from './bridge';

/**
 * The front's defaults, identical to Rust's.
 *
 * They serve before the first response, and outside the Tauri window.
 * Keeping them aligned is a chore; the alternative — showing no setting
 * until Rust has answered — would make the whole settings page flash on
 * every open.
 */
const DEFAULTS: SettingsView = {
  schema: 1,
  game: { renderDistance: 12, simulationDistance: 10, maxFps: 120, guiScale: 0, vsync: true },
  window: { mode: 'windowed', width: 1280, height: 720 },
  launcher: { memoryMb: 4096, minimizeOnLaunch: true },
  appearance: { backdrop: 'spawn', scrim: 0.55 },
};

/**
 * What the player has set.
 *
 * ## Appearance is applied HERE, on `<html>`
 *
 * The backdrop and the scrim are set as custom properties on the root
 * element, from this service, and not by a component style. The reason is
 * mechanical: Angular's emulated encapsulation rewrites selectors, and a
 * `:root { … }` written in a component would become
 * `:root[_ngcontent-abc] { … }` — which designates nothing.
 */
@Injectable({ providedIn: 'root' })
export class SettingsService {
  private readonly bridge = inject(Bridge);

  readonly view = signal<SettingsView>(DEFAULTS);
  readonly screen = signal<Screen | null>(null);

  /** Re-reads the settings and applies them to the window. */
  async load(): Promise<void> {
    if (!this.bridge.available) {
      this.apply(DEFAULTS);
      return;
    }
    const [settings, screen] = await Promise.all([this.bridge.settings(), this.bridge.screen()]);
    this.view.set(settings);
    this.screen.set(screen);
    this.apply(settings);
  }

  /**
   * Saves, and takes WHATEVER RUST RENDERS BACK as true.
   *
   * If a value was pulled back within its bounds, the screen must show it
   * right away: keeping what we sent would leave a slider at a position the
   * file doesn't carry, and the player would believe they set 200 where the
   * game will receive 32.
   */
  async save(settings: SettingsView): Promise<void> {
    if (!this.bridge.available) {
      this.view.set(settings);
      this.apply(settings);
      return;
    }
    const written = await this.bridge.saveSettings(settings);
    this.view.set(written);
    this.apply(written);
  }

  /** Changes one section, and saves. */
  async update(partial: Partial<SettingsView>): Promise<void> {
    await this.save({ ...this.view(), ...partial });
  }

  /**
   * Sets what a setting changes in the DOM, without writing anything.
   *
   * Public because the settings page uses it to PREVIEW: dragging the scrim
   * slider must lighten the image under the finger, while the write only
   * happens on release. Without this, you'd be setting a darkening level
   * blind.
   */
  reflect(settings: SettingsView): void {
    this.apply(settings);
  }

  private apply(settings: SettingsView): void {
    const root = document.documentElement;
    root.dataset['backdrop'] = settings.appearance.backdrop;
    root.style.setProperty('--scrim', String(settings.appearance.scrim));
  }
}
