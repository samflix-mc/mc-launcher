import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Incidents } from './incidents';

describe('Incidents', () => {
  let incidents: Incidents;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    incidents = TestBed.inject(Incidents);
    // The service logs the raw cause to the console. That's intentional —
    // we want both the formatted message and the cause — but a suite that
    // spews red errors ends up hiding the real ones.
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  it('has nothing to show at the start', () => {
    expect(incidents.current()).toBeNull();
    expect(incidents.open()).toBe(false);
  });

  /**
   * What Rust sends is a STRING, and what arrives otherwise comes from the
   * bridge. Without this translation, a player would read
   * "[object Object]".
   */
  it('translates whatever it is given', () => {
    incidents.report('the lock is unreachable');
    expect(incidents.current()).toBe('the lock is unreachable');

    incidents.report(new Error('java 21 not found'));
    expect(incidents.current()).toBe('java 21 not found');
  });

  /**
   * One error at a time, and it's the LAST one that wins.
   *
   * Stacking would ask the player to close them one by one, when the first
   * is almost always the cause of the ones that follow.
   */
  it('keeps the last, not the first', () => {
    incidents.report('first');
    incidents.report('second');

    expect(incidents.current()).toBe('second');
    expect(incidents.open()).toBe(true);
  });

  it('closes', () => {
    incidents.report('something');
    incidents.close();

    expect(incidents.current()).toBeNull();
    expect(incidents.open()).toBe(false);
  });

  it('lets what succeeds through, without reporting anything', async () => {
    const value = await incidents.guard(async () => 42);

    expect(value).toBe(42);
    expect(incidents.open()).toBe(false);
  });

  /**
   * **The case that justifies the wrapper.** Every call redid the same
   * sequence, each time with a chance to forget the `catch` — and a promise
   * rejected into the void, with an error no one sees.
   */
  it('catches what fails, shows it, and renders null', async () => {
    const value = await incidents.guard(async () => {
      throw new Error('the pack disappeared');
    });

    expect(value).toBeNull();
    expect(incidents.current()).toBe('the pack disappeared');
  });

  /**
   * The wrapper clears the previous error BEFORE acting: without this, a
   * successful second attempt would leave the first failure's overlay
   * showing.
   */
  it('clears the previous error before acting', async () => {
    incidents.report('previous failure');

    await incidents.guard(async () => 'it works');

    expect(incidents.open()).toBe(false);
  });
});
