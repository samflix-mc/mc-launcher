import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Account } from './contracts';
import { Session } from './session';

function account(ownsTheGame: boolean): Account {
  return {
    username: 'thesam1798',
    uuid: '0123-4567',
    ownsTheGame,
  };
}

describe('Session', () => {
  let session: Session;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    session = TestBed.inject(Session);
  });

  /**
   * `undefined` is not `null`, and the difference carries both guards:
   * "haven't asked yet" is not "no one is signed in". Conflating them would
   * redirect to sign-in before even asking Rust, on every startup.
   */
  it('claims nothing before having asked', () => {
    expect(session.known()).toBe(false);
    expect(session.playable()).toBe(false);
    expect(session.noLicense()).toBe(false);
  });

  it('knows when no one is signed in', () => {
    session.account.set(null);

    expect(session.known()).toBe(true);
    expect(session.playable()).toBe(false);
    expect(session.noLicense()).toBe(false);
  });

  /**
   * **THE predicate for both guards.** A single one, and that's
   * deliberate: two different predicates would make an account connected
   * without a license bounce forever between `/signin` and `/spawn`.
   */
  it('is only playable with the license', () => {
    session.account.set(account(true));
    expect(session.playable()).toBe(true);
    expect(session.noLicense()).toBe(false);
  });

  /**
   * The case that isn't theoretical: a valid Microsoft account that never
   * bought Minecraft. It's a STATE to display, not a redirect.
   */
  it('distinguishes "no license" from "not signed in"', () => {
    session.account.set(account(false));

    expect(session.known()).toBe(true);
    expect(session.playable()).toBe(false);
    expect(session.noLicense()).toBe(true);
  });

  /**
   * Outside the Tauri window — `ng serve`, a test suite — there's no one to
   * ask. Returning `null` rather than staying at `undefined` keeps the
   * screen from waiting for an answer that will never come.
   */
  it('outside Tauri, concludes there is no account', async () => {
    await session.open();

    expect(session.known()).toBe(true);
    expect(session.account()).toBeNull();
  });
});
