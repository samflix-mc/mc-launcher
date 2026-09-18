import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  afterNextRender,
  computed,
  inject,
} from '@angular/core';
import { LucideAngularModule } from 'lucide-angular';

import { Check, Clock, Globe, Package, TriangleAlert, Users } from '../core/icons';
import { NewsCard } from '../news/card/news-card';
import { WindowService } from '../core/window';
import { Incidents } from '../core/incidents';
import { Log } from '../core/log';
import { News } from '../core/news';
import { Pack } from '../core/pack';
import { Server } from '../core/server';
import { Bridge } from '../core/bridge';

/** How often the server panel re-probes. */
const SERVER_REFRESH_MS = 30_000;

/** What the server panel shows — the badge's color, and the three facts. */
interface ServerView {
  readonly variant: 'online' | 'offline' | 'unknown';
  /**
   * Is this state SETTLED, or are we still waiting?
   *
   * The design system makes the `unknown` dot pulse, and that pulse means
   * one precise thing: a response is expected. It's right while the first
   * probe is in flight, and it lies for an environment that declares no
   * server at all — preproduction would pulse forever for an answer nobody
   * is coming to give. Both stay grey, because neither is "down"; only one
   * keeps moving.
   */
  readonly settled: boolean;
  readonly state: string;
  readonly address: string;
  readonly players: string;
}

/** What the pack's status badge says, and in what color. */
interface Badge {
  readonly variant: 'online' | 'update' | 'warning' | 'offline' | 'unknown';
  readonly label: string;
  readonly value: string | null;
}

/**
 * The main screen — "Spawn".
 *
 * The name comes from Sam: "home" said nothing about a Minecraft launcher,
 * and "Spawn" is where you arrive. It's used as-is in the navigation, in the
 * routes and here, with no intermediate translation.
 *
 * ## What the page carries, and what it no longer carries
 *
 * The design system lays out two columns up top — the pinned post on the
 * left, the status panels on the right — and LEAVES THE MIDDLE EMPTY: that's
 * where the image shows through, and it's the only place on screen where it
 * truly shows.
 *
 * The play button and the player badge aren't here anymore: they live in the
 * window's bottom bar, which is the shell's. This page no longer decides
 * anything about what launches; it says what IS.
 *
 * ## The cinematic isn't here either anymore
 *
 * It was here during the build, in a panel below the modpack's, and it was
 * removed at acceptance. The reason is sound: the button already carries the
 * overall progress, its percentage and its rate, and the phrase above it
 * NAMES the current step. The list of eleven phases was therefore saying a
 * third time what two elements already said — at the cost of a panel that
 * appeared and disappeared right under the eye, in the exact spot where
 * progress is being followed.
 *
 * ## Why the old version was unreadable
 *
 * It stacked seven conditional blocks in one column: badges, a cinematic, a
 * gauge, three alerts, a verdict, a news card, a button. None had a
 * reserved spot, so everything moved; and nothing said which block answered
 * what. Here, each panel has a fixed spot and a heading, and what has
 * nothing to say doesn't show — without shifting the rest.
 */
@Component({
  selector: 'app-spawn',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NewsCard, LucideAngularModule],
  templateUrl: './spawn.html',
  styleUrl: './spawn.css',
})
export class Spawn {
  private readonly pack = inject(Pack);
  private readonly incidents = inject(Incidents);
  private readonly news = inject(News);
  private readonly bridge = inject(Bridge);
  private readonly windowService = inject(WindowService);
  private readonly log = inject(Log);
  private readonly serverService = inject(Server);
  private readonly destroyRef = inject(DestroyRef);

  protected readonly state = this.pack.state;
  protected readonly report = this.pack.lastReport;

  protected readonly pinned = this.news.pinned;

  protected readonly Package = Package;
  protected readonly Globe = Globe;
  protected readonly Users = Users;

  /**
   * The server panel, drawn from what [`Server`](../core/server) last
   * probed.
   *
   * **Three states, not two.** `unknown` covers both the moment before the
   * first probe answers AND an environment with no declared server at all
   * — preproduction, typically. Neither is "down": a red badge on either
   * would tell a player a connection was refused when none was ever
   * attempted. `online` and `offline` only appear once a probe has actually
   * run and come back one way or the other.
   *
   * **Player counts only ever come from a successful probe.** `players` and
   * `slots` on the bridge's answer are `null` for every state but `online`
   * — carrying over a stale count while offline, or guessing one while
   * still checking, would claim knowledge the launcher doesn't have.
   */
  protected readonly server = computed<ServerView>(() => {
    const current = this.serverService.status();
    if (!current) {
      return { variant: 'unknown', settled: false, state: 'Checking', address: '—', players: '—' };
    }
    switch (current.state) {
      case 'online': {
        const players =
          current.players !== null && current.slots !== null
            ? `${current.players} / ${current.slots}`
            : '—';
        return {
          variant: 'online',
          settled: true,
          state: 'Online',
          address: current.host,
          players,
        };
      }
      case 'offline':
        return {
          variant: 'offline',
          settled: true,
          state: 'Offline',
          address: current.host,
          players: '—',
        };
      case 'undeclared':
        return {
          variant: 'unknown',
          settled: true,
          state: 'No server declared',
          address: '—',
          players: '—',
        };
    }
  });
  protected readonly Check = Check;
  protected readonly Clock = Clock;
  protected readonly TriangleAlert = TriangleAlert;

  /** The pack's status, as a badge. */
  protected readonly badge = computed<Badge>(() => {
    const current = this.state();
    if (!current) {
      return { variant: 'unknown', label: 'Checking', value: null };
    }
    if (current.offline) {
      return { variant: 'offline', label: 'Offline', value: current.version };
    }
    switch (current.drift) {
      case 'up-to-date':
        return { variant: 'online', label: 'Up to date', value: current.version };
      case 'update':
        return { variant: 'update', label: 'Update', value: current.version };
      case 'reinstall':
        return { variant: 'update', label: 'Reinstall', value: current.version };
      case 'absent':
        return { variant: 'warning', label: 'Not installed', value: current.version };
      case 'unknown':
        return { variant: 'unknown', label: 'Checking', value: current.version };
    }
  });

  protected readonly missing = computed(() => this.report()?.missing ?? []);
  protected readonly drifts = computed(() => this.report()?.drifts ?? []);
  protected readonly purge = computed(() => this.report()?.purge ?? []);

  /** Is there anything to say about the last gesture? */
  protected readonly hasReport = computed(
    () =>
      this.report() !== null &&
      (this.missing().length > 0 || this.drifts().length > 0 || this.purge().length > 0),
  );

  constructor() {
    // **This is where the signal showing the main window departs from.**
    //
    // From `afterNextRender`, and not from navigation: `navigate` returns
    // control once the route is ACTIVATED, which precedes the first pixel
    // by several frames. Showing the window at that point would make it
    // appear on a screen that's still empty — exactly the defect this is
    // meant to remove.
    //
    // Rust only waits for this signal during a sign-in; at ordinary
    // startup, it doesn't listen for it, and sending it costs nothing.
    //
    // **But only the main window has the right to say it.** A dedicated
    // window mounting this page — which shouldn't happen anymore, but
    // did — would tell Rust that the MAIN one is ready when it has rendered
    // nothing at all: Rust would switch right away, and we'd fall back into
    // the screen filling itself in before your eyes.
    afterNextRender(() => {
      if (this.windowService.inADedicatedWindow) {
        this.log.concern('Spawn mounted in a DEDICATED window: "main_ready" is not sent');
        return;
      }
      this.log.step('Spawn drawn: "main_ready" sent');
      void this.bridge.mainReady().catch(() => {});
    });

    // The pack is opened by the SHELL — the title bar and the play button
    // depend on it, and they outlive this page. We just ask again for its
    // state: the disk may have changed while we were elsewhere.
    void this.incidents.guard(() => this.pack.refresh());
    // The feed isn't part of what the screen waits for: an empty tile is
    // better than a screen waiting on the network to show its button.
    void this.news.load().catch(() => {});

    // The server panel: probed right away, then every thirty seconds for
    // as long as this page stays mounted. Not routed through
    // `incidents.guard` — that overlay blocks the whole screen, which is
    // the wrong reaction to a background poll, and the command it calls
    // doesn't reject on an unreachable server anyway.
    void this.serverService.refresh().catch(() => {});
    const interval = setInterval(() => {
      void this.serverService.refresh().catch(() => {});
    }, SERVER_REFRESH_MS);
    // Without this, a second mount of Spawn — or a route that comes and
    // goes — would leave the previous timer running forever, each one
    // still probing on its own schedule.
    this.destroyRef.onDestroy(() => clearInterval(interval));
  }
}
