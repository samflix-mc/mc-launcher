import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { Session } from '../core/session';
import { SignIn } from './signin';

/**
 * The sign-in page, in its three states plus one.
 *
 * The fourth isn't a step: a valid Microsoft account that never bought
 * Minecraft. Both router guards read the SAME predicate so this case doesn't
 * bounce between them; it stays here, and the screen says so.
 */
describe('SignIn', () => {
  let session: Session;

  function mount() {
    const fixture = TestBed.createComponent(SignIn);
    fixture.detectChanges();
    return fixture;
  }

  function read(fixture: ReturnType<typeof mount>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    session = TestBed.inject(Session);
  });

  it('at the start, it invites you to sign in', () => {
    const fixture = mount();

    expect(read(fixture, 'sign-in')).toContain('Sign in with Microsoft');
    expect(fixture.nativeElement.querySelector('[data-test="code"]')).toBeNull();
  });

  /**
   * The three steps are the "loading bar" the acceptance test asked for:
   * they say where we are in a sequence that takes time, instead of letting
   * it look like nothing is happening.
   */
  it('the first step is crossed as soon as it opens', () => {
    const fixture = mount();

    const steps = [...fixture.nativeElement.querySelector('[data-test="step"]').children];
    expect(steps).toHaveLength(3);
    expect(steps.filter((s: Element) => s.classList.contains('hm-auth__step--done'))).toHaveLength(
      1,
    );
  });

  it('once the code arrives, it shows it and announces the wait', () => {
    session.code.set({
      code: 'FKRD-QXZB',
      url: 'https://www.microsoft.com/link',
      directUrl: 'https://www.microsoft.com/link?otc=FKRD-QXZB',
    });
    const fixture = mount();

    expect(read(fixture, 'code')).toBe('FKRD-QXZB');
    expect(read(fixture, 'wait')).toContain('Waiting for Microsoft');
    expect(read(fixture, 'reopen')).toContain('microsoft.com/link');
  });

  it('at the code step, two of the three steps are crossed', () => {
    session.code.set({ code: 'A', url: 'https://x', directUrl: 'https://x' });
    const fixture = mount();

    const steps = [...fixture.nativeElement.querySelector('[data-test="step"]').children];
    expect(steps.filter((s: Element) => s.classList.contains('hm-auth__step--done'))).toHaveLength(
      2,
    );
  });

  /**
   * The code is what we're asking to be copied: it must be selectable,
   * where the rest of the window doesn't allow it.
   */
  it('the code is selectable, and copiable', () => {
    session.code.set({ code: 'A', url: 'https://x', directUrl: 'https://x' });
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="code-well"]').classList).toContain(
      'hm-selectionnable',
    );
    expect(fixture.nativeElement.querySelector('[data-test="copy-code"]')).not.toBeNull();
  });

  /**
   * **"Signed in but no license" is a STATE, not a redirect.**
   *
   * Saying so costs three lines; not saying so costs eight hundred
   * megabytes downloaded just to learn afterward that no server will
   * accept the account.
   */
  it('an account with no license is a state, and it says so', () => {
    session.account.set({ username: 'thesam1798', uuid: '0123', ownsTheGame: false });
    const fixture = mount();

    expect(read(fixture, 'no-license')).toContain("doesn't own Minecraft");
    expect(read(fixture, 'no-license')).toContain('thesam1798');
    expect(fixture.nativeElement.querySelector('[data-test="switch-account"]')).not.toBeNull();
  });

  /** In this state, the steps have nothing left to say: the rest is elsewhere. */
  it('with no license, the steps bar disappears', () => {
    session.account.set({ username: 'x', uuid: '0', ownsTheGame: false });
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="step"]')).toBeNull();
  });
});
