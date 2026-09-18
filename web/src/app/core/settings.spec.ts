import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { SettingsService } from './settings';

describe('SettingsService', () => {
  let settings: SettingsService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    settings = TestBed.inject(SettingsService);
    document.documentElement.removeAttribute('data-backdrop');
    document.documentElement.style.removeProperty('--scrim');
  });

  /**
   * The front's defaults mirror Rust's, and that's an accepted chore: the
   * alternative — showing no setting until Rust has answered — would make
   * the whole settings page flash on every open.
   */
  it('starts from the same defaults as Rust', () => {
    const view = settings.view();

    expect(view.schema).toBe(1);
    expect(view.appearance.backdrop).toBe('spawn');
    expect(view.appearance.scrim).toBe(0.55);
    expect(view.window.mode).toBe('windowed');
  });

  /**
   * **Appearance is set on `<html>`, from this service.**
   *
   * Not by a component style: Angular's emulated encapsulation rewrites
   * selectors, and a `:root { … }` written in a component would become
   * `:root[_ngcontent-abc] { … }` — which designates nothing. The test
   * therefore targets the root element, and that's the only place it can.
   */
  it('sets the backdrop and the scrim on the root element', async () => {
    await settings.load();

    const root = document.documentElement;
    expect(root.dataset['backdrop']).toBe('spawn');
    expect(root.style.getPropertyValue('--scrim')).toBe('0.55');
  });

  /**
   * Outside Tauri, `save` keeps what it's given and applies it: there's no
   * one to bound it, and the page must stay usable under `ng serve`.
   */
  it('applies what is saved', async () => {
    const wanted = {
      ...settings.view(),
      appearance: { backdrop: 'nether' as const, scrim: 0.8 },
    };

    await settings.save(wanted);

    expect(settings.view().appearance.backdrop).toBe('nether');
    expect(document.documentElement.dataset['backdrop']).toBe('nether');
    expect(document.documentElement.style.getPropertyValue('--scrim')).toBe('0.8');
  });

  /**
   * `update` replaces ONLY the given section: the others survive. Without
   * this, changing the backdrop would reset the render distance to its
   * default, without anything saying so.
   */
  it('changes only the section it is given', async () => {
    await settings.update({ appearance: { backdrop: 'end', scrim: 0.6 } });

    expect(settings.view().appearance.backdrop).toBe('end');
    // Untouched.
    expect(settings.view().game.renderDistance).toBe(12);
    expect(settings.view().launcher.memoryMb).toBe(4096);
  });
});
