import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import { Download, Play, RefreshCw, Rocket, Square, WifiOff } from '../../core/icons';
import * as format from '../../core/format';
import { Incidents } from '../../core/incidents';
import { Notifications } from '../../core/notifications';
import { Pack } from '../../core/pack';

/** The button's look, in the design system's sense. */
type Look = 'ready' | 'busy' | 'inert';

/**
 * How long the button waits for a stop to be confirmed.
 *
 * Long enough for a deliberate second click, short enough that a button left
 * on "Stop the game?" doesn't turn into a trap three hours later.
 */
const CONFIRMATION_DELAY = 4000;

/**
 * THE button, and what's written above it.
 *
 * ## Why it lives in the shell and not in Spawn
 *
 * Because it isn't a page decision: it's the state of the disk and that of
 * the session that drive it, and both live in root services. The design
 * system places it in the bottom bar of the window, centered; Spawn is only
 * the page that displays it.
 *
 * ## It no longer launches the game on its own
 *
 * It used to say "Update and play", and it did both. Sam called that out
 * during acceptance: clicking to lay down a modpack and watching Minecraft
 * start isn't what was asked for. **As soon as there's something to lay
 * down, the gesture is to lay it down**; playing comes after, with a second
 * click.
 *
 * The rule isn't here: it's `mc_pack::comparison::to_place`, the same one
 * install follows, and the `Action` that Rust returns is its result. This
 * component only draws a label from it — otherwise the button and the
 * install could one day disagree about what's about to happen.
 *
 * ## The states, and what tells them apart
 *
 * They don't come from a single field: the `Action` says what to DO, the
 * drift says what separates what's laid down from what's published, and
 * activity is observed. Conflating them would make it answer "an operation
 * is already running" to someone clicking during their game session, which
 * is the longest-running state of the session.
 *
 * ## The two lines above aren't decorative
 *
 * The first says WHY the button is what it is — "not installed yet",
 * "update available", "offline" — or, during work, what step we're on.
 *
 * The second exists ONLY during work, and it's the other piece of
 * acceptance feedback: "we roughly know what step we're on, but not what's
 * being downloaded, nor at what speed". It carries the current file, the
 * file count, the bytes, the rate and the time remaining — all data that
 * `Tracker` emits five times a second and that nothing displayed.
 */
@Component({
  selector: 'app-playbar',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './playbar.html',
  styleUrl: './playbar.css',
})
export class Playbar {
  private readonly pack = inject(Pack);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);

  protected readonly state = this.pack.state;
  protected readonly button = this.pack.button;
  protected readonly progress = this.pack.progress;
  protected readonly overallProgress = this.pack.overallProgress;
  protected readonly currentStep = this.pack.currentStep;

  protected readonly WifiOff = WifiOff;

  /** Ready to click, busy, or blocked. */
  protected readonly look = computed<Look>(() => {
    switch (this.button()) {
      case 'install':
      case 'play':
        return this.blockedBy() ? 'inert' : 'ready';
      case 'busy':
      case 'unknown':
        return 'busy';
      // **Clickable, and that's an acceptance addition.** `play` only
      // returns once the session ends: a Minecraft stuck on a loading
      // screen used to leave the launcher stuck there, with no way out but
      // the task manager.
      case 'in-game':
        return 'ready';
    }
  });

  /**
   * What's blocking a click, or `null`.
   *
   * Offline AND nothing installed: there's nothing to launch and nothing to
   * download. Offline with a pack laid down, on the other hand, plays just
   * fine — the launcher just couldn't check.
   */
  protected readonly blockedBy = computed(() => {
    const seen = this.state();
    if (!seen) {
      return null;
    }
    if (seen.offline && !seen.installed) {
      return 'offline' as const;
    }
    return null;
  });

  /**
   * The button's label.
   *
   * None of them say "and play" anymore: what the button announces is
   * exactly what it does, and nothing more.
   */
  protected readonly label = computed(() => {
    switch (this.button()) {
      case 'unknown':
        return 'Checking…';
      case 'install':
        switch (this.state()?.drift) {
          case 'update':
            return 'Update';
          case 'reinstall':
            return 'Reinstall';
          default:
            return 'Install';
        }
      case 'play':
        return 'Play';
      case 'busy':
        return 'Installing…';
      case 'in-game':
        return this.stopRequested() ? 'Stop the game?' : 'Playing';
    }
  });

  /**
   * The icon, when there is one.
   *
   * `null` during work: the fill bar then holds the spot, and two activity
   * markers side by side say nothing more.
   */
  protected readonly icon = computed<LucideIconData | null>(() => {
    switch (this.button()) {
      case 'install':
        return this.state()?.drift === 'absent' ? Download : RefreshCw;
      case 'play':
        return Play;
      case 'in-game':
        return this.stopRequested() ? Square : Rocket;
      default:
        return null;
    }
  });

  /**
   * What's written after the separator, in the button itself.
   *
   * **The percentage stays shown between batches**, and that's a fix: it
   * used to show only during an active download, so it disappeared during
   * mod resolution, jar inspection and the NeoForge installer — that is,
   * for a good chunk of the time. The rate, though, only means something
   * when something is actually coming down.
   */
  protected readonly meta = computed(() => {
    if (!this.installing()) {
      return null;
    }
    const percent = `${Math.round(this.overallProgress())} %`;
    const seen = this.progress();
    return seen?.active && seen.rate > 0 ? `${percent} · ${format.rate(seen.rate)}` : percent;
  });

  /** The line on top, and its tone. */
  protected readonly hint = computed(() => {
    if (this.blockedBy() === 'offline') {
      return { text: "Offline — can't fetch the pack.", danger: true };
    }

    if (this.button() === 'in-game') {
      return this.stopRequested()
        ? {
            text: "Click again to force a stop — the session won't be saved.",
            danger: true,
          }
        : { text: 'The game is running. Click to stop it if it stops responding.', danger: false };
    }

    if (this.installing()) {
      const step = this.currentStep();
      const where = step ? `${step.label} — step ${step.number} of ${step.total}` : 'Preparing';
      const note = this.progress()?.note;
      return { text: note ? `${where} · ${note}` : where, danger: false };
    }

    const pack = this.state();
    if (!pack) {
      return { text: "Reading what's installed…", danger: false };
    }
    if (pack.offline) {
      return { text: "Offline — updates aren't checked.", danger: false };
    }

    switch (pack.drift) {
      case 'absent':
        return { text: `Not installed yet${this.versionSuffix()}`, danger: false };
      case 'up-to-date':
        return { text: `Up to date${this.versionSuffix()}`, danger: false };
      case 'update':
        return { text: `Update available${this.versionSuffix()}`, danger: false };
      case 'reinstall':
        return {
          text: 'Full reinstall — your worlds and settings are kept',
          danger: false,
        };
      case 'unknown':
        return { text: 'Checking the pack…', danger: false };
    }
  });

  /**
   * The second line: what's REALLY happening, right now.
   *
   * `null` outside of work — a button at rest has no detail to carry — and
   * made of segments that are omitted when unknown, rather than showing
   * "0 B of 0 B" or an orphaned middle dot.
   */
  protected readonly detail = computed(() => {
    const seen = this.progress();
    if (!this.installing() || !seen) {
      return null;
    }

    const segments: string[] = [];

    const name = seen.file ? fileName(seen.file) : null;
    if (name) {
      segments.push(name);
    }
    if (seen.filesTotal > 0) {
      segments.push(`${seen.files} of ${seen.filesTotal} files`);
    }
    if (seen.total > 0) {
      segments.push(`${format.bytes(seen.bytes)} of ${format.bytes(seen.total)}`);
    }
    if (seen.active && seen.rate > 0) {
      segments.push(format.rate(seen.rate));
    }
    if (seen.remaining !== null) {
      segments.push(`${format.duration(seen.remaining)} remaining`);
    }

    return segments.length > 0 ? segments.join(' · ') : null;
  });

  /**
   * The bar's fill, as a percentage.
   *
   * **It follows the GLOBAL progress, not that of the current batch.** The
   * distinction matters: between two batches, a single batch's progress
   * jumps to a hundred percent while the step is still working — which
   * lied. The global progress, instead, is bounded by the phase reached:
   * between two batches it holds steady, which is the truth.
   *
   * Indeterminate as long as we know nothing at all: during verification,
   * or before the first progress event.
   */
  protected readonly fill = computed(() => {
    if (!this.installing() || !this.progress()) {
      return null;
    }
    const percent = Math.round(this.overallProgress());
    return percent > 0 ? `${percent}%` : null;
  });

  /**
   * Is a gesture in progress?
   *
   * **Distinct from "the button is spinning"**: between the screen showing
   * up and `pack_state()`'s response, the button spins too, but nothing is
   * being laid down. Conflating the two would show "0%", a "Preparing" step
   * and an empty detail line on every startup, instead of the line saying
   * we're reading the disk.
   */
  private readonly installing = computed(() => this.button() === 'busy');

  /**
   * Has a stop already been requested once?
   *
   * **Two steps rather than a modal.** Killing the game loses whatever
   * wasn't saved, and this button sits at the center of the bottom bar: one
   * click too many happens fast there. A modal would be heavier than it
   * looks — it would need closing with the keyboard, pulling out of the
   * flow, given focus — where the button itself can ask the question.
   *
   * The request falls back on its own: a button left on "Stop the game?"
   * for an hour-long session would be a trap rather than a guard.
   */
  private readonly stopRequested = signal(false);

  /**
   * THE click.
   *
   * Two gestures behind one button, and it's Rust's `Action` that decides:
   * if there's something to lay down, we lay it down; otherwise we play.
   */
  protected async act(): Promise<void> {
    if (this.look() !== 'ready') {
      return;
    }
    if (this.button() === 'in-game') {
      this.stop();
      return;
    }
    if (this.button() === 'install') {
      await this.install();
      return;
    }
    await this.play();
  }

  /**
   * First click: we ask. Second: we stop.
   *
   * The report on the interrupted session will arrive via `play()`, which is
   * still in flight — it isn't this gesture's job to announce it.
   */
  private stop(): void {
    if (!this.stopRequested()) {
      this.stopRequested.set(true);
      setTimeout(() => this.stopRequested.set(false), CONFIRMATION_DELAY);
      return;
    }
    this.stopRequested.set(false);
    void this.incidents.guard(() => this.pack.stopGame());
  }

  private async install(): Promise<void> {
    const outcome = await this.incidents.guard(() => this.pack.install());
    if (!outcome) {
      return;
    }
    if (outcome.missing.length > 0) {
      this.notifications.notify(
        'warning',
        `${outcome.missing.length} mod(s) missing`,
        outcome.missing.join(', '),
      );
      return;
    }
    this.notifications.notify('success', 'Pack installed', outcome.verdict);
  }

  private async play(): Promise<void> {
    const outcome = await this.incidents.guard(() => this.pack.play());
    if (!outcome) {
      return;
    }
    if (outcome.missing.length > 0) {
      this.notifications.notify(
        'warning',
        `${outcome.missing.length} mod(s) missing`,
        outcome.missing.join(', '),
      );
    } else if (outcome.caughtUp) {
      this.notifications.notify('success', 'Pack updated', outcome.verdict);
    }
  }

  /** " · 1.4.2", or nothing. No orphaned middle dot when there's no version. */
  private versionSuffix(): string {
    const version = this.state()?.version;
    return version ? ` · ${version}` : '';
  }
}

/**
 * The bare name of a path.
 *
 * Rust may announce a full path, and an info line showing
 * `/home/…/shared/libraries/net/neoforged/…/neoforge-21.1.250-universal.jar`
 * overflows the window and can't be read. Free function: it doesn't depend
 * on anything from the component, and that's what makes it testable on its
 * own.
 */
export function fileName(path: string): string | null {
  const parts = path.split(/[\\/]/);
  const name = parts.at(-1) || path;
  return isDigest(name) ? null : name;
}

/**
 * Is this "name" in fact a digest?
 *
 * Minecraft addresses its assets by their SHA-1: the three thousand nine
 * hundred objects of a version live under `objects/ab/ab3f9e…`, and their
 * file name IS the digest. Announcing it shows forty hexadecimal
 * characters that change five times a second and say nothing — the batch
 * counter, the rate and the time remaining, all sitting right next to it,
 * say everything it doesn't.
 *
 * The rule is on the DISPLAY side rather than in Rust because that's what
 * it is: a decision about what a human can read. Rust announces the name
 * of what it writes, which is correct and is what the log needs.
 *
 * Thirty-two characters as the floor: MD5 is the shortest digest the
 * launcher meets, and no jar or library is named with thirty-two
 * hexadecimal characters and nothing else — they all carry a version, a
 * dash or an extension.
 */
function isDigest(name: string): boolean {
  return /^[0-9a-f]{32,}$/.test(name);
}
