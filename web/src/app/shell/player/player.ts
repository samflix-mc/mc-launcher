import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { Router } from '@angular/router';
import { LucideAngularModule } from 'lucide-angular';

import { ChevronDown, LogOut, RefreshCw, User } from '../../core/icons';
import { Incidents } from '../../core/incidents';
import { Notifications } from '../../core/notifications';
import { Session } from '../../core/session';
import { playerHead } from '../../core/bridge';

/**
 * The signed-in player, bottom right, and their account menu.
 *
 * ## The head is an IMAGE, not an icon
 *
 * A head render by UUID, served by `mc-heads.net` — which is why `img-src`
 * names it in the CSP, and it's the only remote host the window reaches.
 * `image-rendering: pixelated`: a sixteen-pixel texture blown up to forty
 * must stay a texture, not a watercolor.
 *
 * Until it arrives, the design system's neutral cube holds the place. No
 * initials: they'd suggest an avatar when what's coming is a game head.
 *
 * ## What is NOT shown
 *
 * The UUID, the Microsoft address, the token. The design system forbids it,
 * and the reason fits in one sentence: those are the three things a player
 * would paste into a Discord channel if the interface put them in view.
 */
@Component({
  selector: 'app-player',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './player.html',
  styleUrl: './player.css',
})
export class Player {
  private readonly session = inject(Session);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);
  private readonly router = inject(Router);

  protected readonly account = this.session.account;
  protected readonly busy = this.session.busy;

  /** Is the account menu open? */
  protected readonly menuOpen = signal(false);

  protected readonly head = computed(() => {
    const account = this.account();
    return account ? playerHead(account.uuid) : null;
  });

  /** Did the head fail to load? We then fall back to the cube. */
  protected readonly headMissing = signal(false);

  protected readonly ChevronDown = ChevronDown;
  protected readonly LogOut = LogOut;
  protected readonly RefreshCw = RefreshCw;
  protected readonly User = User;

  protected toggleMenu(): void {
    this.menuOpen.update((open) => !open);
  }

  protected closeMenu(): void {
    this.menuOpen.set(false);
  }

  /**
   * Asks Rust for the status again.
   *
   * Useful when a license was just purchased, or when the token was renewed
   * in the background: without this gesture, the launcher would need to
   * restart for the window to notice.
   */
  protected async refresh(): Promise<void> {
    this.closeMenu();
    await this.incidents.guard(async () => {
      await this.session.open();
      this.notifications.notify('info', 'Session refreshed', this.account()?.username ?? null);
    });
  }

  protected async signOut(): Promise<void> {
    this.closeMenu();
    await this.incidents.guard(async () => {
      const username = this.account()?.username ?? null;
      await this.session.signOut();
      this.notifications.notify('info', 'Signed out', username);
      // The `/spawn` guard would redirect anyway, but waiting on it would
      // leave a page talking about an account that no longer exists for the
      // length of a render. We go there ourselves.
      await this.router.navigate(['/signin']);
    });
  }
}
