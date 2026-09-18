import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { Router } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { Check, Copy, ExternalLink, TriangleAlert } from '../core/icons';
import { Incidents } from '../core/incidents';
import { Log } from '../core/log';
import { WindowService } from '../core/window';
import { Bridge } from '../core/bridge';
import { Session } from '../core/session';

/** Where we are among the design system's three steps. */
type Step = 'invite' | 'code' | 'done' | 'no-license';

/**
 * Opening a Microsoft session — a modal, and nothing behind it.
 *
 * ## It is BLOCKING, and that was the acceptance test's first complaint
 *
 * Before, the sign-in page displayed inside the shell: a menu on the left, a
 * "sign out" button, an empty player badge. It offered to sign out someone
 * who wasn't signed in. Now the shell doesn't exist until the session is
 * playable — see `app.html` — and this page is an `.hm-scrim` over the scene,
 * with a single dialog.
 *
 * ## The three steps, and the fourth that isn't one
 *
 * The design system draws three: invite, enter the code, done. The third
 * doesn't display here: once the session becomes playable, the router guard
 * takes you to Spawn, and a "done" step visible for a tenth of a second isn't
 * worth writing.
 *
 * The fourth state isn't a step: a valid Microsoft account that never bought
 * Minecraft. Both router guards read the SAME predicate — `playable()` —
 * precisely so this case doesn't bounce between them: it stays here, and the
 * screen says so.
 *
 * ## The four seconds
 *
 * Between the moment the player authorizes on Microsoft's side and the moment
 * the launcher notices, a polling interval elapses — measured at roughly four
 * seconds. That's what `.hm-auth__wait` is for: a spinning ring and a
 * sentence, so the wait is announced rather than endured.
 */
@Component({
  selector: 'app-signin',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './signin.html',
  styleUrl: './signin.css',
})
export class SignIn {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly bridge = inject(Bridge);
  private readonly windowService = inject(WindowService);
  private readonly router = inject(Router);
  private readonly trace = inject(Log);

  protected readonly account = this.session.account;
  protected readonly code = this.session.code;
  protected readonly busy = this.session.busy;

  protected readonly Copy = Copy;
  protected readonly Check = Check;
  protected readonly ExternalLink = ExternalLink;
  protected readonly TriangleAlert = TriangleAlert;

  /**
   * The "done" screen, set by hand.
   *
   * It is NOT derived from the session: the moment it becomes playable,
   * everything else is ready to switch over, and a state that would
   * disappear immediately wouldn't register. It's a signal we raise, and
   * that Rust holds for at least a second and a half — see
   * `SIGNED_IN_FLOOR` in `windows.rs`.
   */
  private readonly done = signal(false);

  protected readonly step = computed<Step>(() => {
    if (this.session.noLicense()) {
      return 'no-license';
    }
    if (this.done()) {
      return 'done';
    }
    return this.code() ? 'code' : 'invite';
  });

  /** How many of the three segments are crossed. */
  protected readonly crossed = computed(() => {
    switch (this.step()) {
      case 'done':
        return 3;
      case 'code':
        return 2;
      default:
        return 1;
    }
  });

  protected async signIn(): Promise<void> {
    this.trace.step('sign-in requested: calling "sign_in" on Rust');
    await this.incidents.guard(async () => {
      await this.session.signIn();
      this.trace.step(
        `Microsoft answered — playable=${this.session.playable()}, ` +
          `noLicense=${this.session.noLicense()}`,
      );
      if (!this.session.playable()) {
        // Valid Microsoft account, but no license: we stay here, and the
        // screen says so. This is the page's fourth state.
        return;
      }
      // The "done" screen BEFORE the switch: without it, sign-in succeeds
      // and the window disappears in the same frame, which reads as a
      // crash rather than a success.
      this.done.set(true);
      this.trace.step('"Signed in" step displayed');

      // Inside the sign-in window, we do NOT navigate: we hand off to the
      // main window, which shows itself, rereads its session and goes to
      // Spawn. Here, there's nothing after — the window closes.
      await this.bridge.signInSucceeded();
      this.trace.step('"sign_in_succeeded" returned control');

      // Outside Tauri — a browser in front of the dev server — there's only
      // one tab: the command above did nothing, and it's the router that
      // takes over.
      if (!this.windowService.inADedicatedWindow) {
        this.trace.step('outside dedicated window: the router takes over to Spawn');
        await this.router.navigate(['/spawn']);
      } else {
        this.trace.detail('dedicated window: no navigation, Rust closes the window');
      }
    });
  }

  protected async signOut(): Promise<void> {
    await this.incidents.guard(() => this.session.signOut());
  }

  /** Copy the code rather than retype it. */
  protected async copy(code: string): Promise<void> {
    await navigator.clipboard.writeText(code).catch(() => {});
  }

  /**
   * Reopens the Microsoft page.
   *
   * Rust already tried it once the code arrived; this is the fallback, for
   * when no browser was set as default.
   */
  protected async reopenThePage(url: string): Promise<void> {
    await this.incidents.guard(() => this.bridge.openPage(url));
  }
}
