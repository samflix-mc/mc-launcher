import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  signal,
  viewChildren,
} from '@angular/core';
import { LucideAngularModule, type LucideIconData } from 'lucide-angular';

import {
  FolderOpen,
  MemoryStick,
  Monitor,
  ShieldCheck,
  SlidersHorizontal,
  Terminal,
} from '../core/icons';
import type { Folder, Backdrop, WindowMode, Settings as SettingsView } from '../core/contracts';
import { Incidents } from '../core/incidents';
import { Notifications } from '../core/notifications';
import { Pack } from '../core/pack';
import { Bridge } from '../core/bridge';
import { SettingsService } from '../core/settings';

/** An entry in the left rail. */
interface Group {
  readonly anchor: string;
  readonly label: string;
  readonly icon: LucideIconData;
}

/** The ceiling of the memory slider, in gigabytes. See `bounds::MEMORY`. */
const MEMORY_MAX_GB = 64;

/**
 * The settings, in five groups, with the design system's rail.
 *
 * ## The draft, and why it isn't a detail
 *
 * The controls read a LOCAL STATE, not the service. `(input)` only updates
 * that state; it's `(change)` — the release — that saves.
 *
 * The first version bound everything directly to the service, and saved on
 * `(input)`. Dragging a slider from one end to the other then fired thirty to
 * fifty round trips per second, each one writing the file; the responses came
 * back out of order, the oldest overwrote the most recent, and the displayed
 * number stayed frozen while you dragged. That's exactly what Sam described:
 * "it moves the handle, but it doesn't change the value".
 *
 * The draft separates the two moments: what you SEE follows your finger
 * without going over the network, what you WRITE is sent once, on release.
 * And since Rust returns what it actually wrote — a value clamped to its
 * bounds shows up as such — the draft resynchronizes on its response.
 *
 * ## The rail puts nothing in the URL
 *
 * The entries used to be anchors `#settings-…`, and they didn't work: back
 * when `withHashLocation()` was in use, the hash belonged to the ROUTER, so
 * clicking wrote a URL it tried to resolve as a route — navigation went back
 * to `/spawn`.
 *
 * The fragment is gone since, but the buttons remain, for a reason that
 * outlives it: scrolling within a page isn't navigating. An anchor would
 * leave a history entry that the "back" button would read as a page change.
 * That's also what the design system describes — "the rail's links scroll to
 * the group and mark the one currently in view".
 *
 * ## The mapping between groups, which isn't obvious
 *
 * | Displayed group | What it configures |
 * |---|---|
 * | **Appearance** | `appearance` — the LAUNCHER's window |
 * | **Game window** | `window` — the GAME's window |
 * | **Video** | `game` — the keys merged into `options.txt` |
 * | **Java** | nothing persisted: read from the LOCK |
 * | **Advanced** | `launcher`, plus verification and the folders |
 *
 * "Appearance" and "Window" are easy to mix up on a first read: the first is
 * ours, the second the game's. That's why the titles say so.
 *
 * ## Java isn't configurable
 *
 * The major version is a property of the PACK, written in its lock and
 * verified on every launch. Offering a choice would let the player's machine
 * contradict what the pack has fixed, and the server would settle it with an
 * ejection that doesn't name its cause. The group displays, and offers
 * nothing.
 *
 * ## A single verification
 *
 * There used to be two — "quick" and "full". The quick one only compared
 * sizes: it said "everything is in place" about a corrupted file of the
 * right length, which is precisely the one case worth verifying.
 */
@Component({
  selector: 'app-settings',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LucideAngularModule],
  templateUrl: './settings.html',
  styleUrl: './settings.css',
})
export class Settings {
  private readonly settings = inject(SettingsService);
  private readonly incidents = inject(Incidents);
  private readonly notifications = inject(Notifications);
  private readonly bridge = inject(Bridge);
  private readonly pack = inject(Pack);

  /**
   * What the controls display — the draft.
   *
   * Initialized from the service, and resynchronized by the `effect` below
   * every time it changes: on load, and after every save, since Rust returns
   * what it actually wrote.
   */
  protected readonly view = signal<SettingsView>(this.settings.view());

  protected readonly screen = this.settings.screen;
  protected readonly packState = this.pack.state;

  /** The result of the last verification, or `null`. */
  protected readonly verification = signal<string[] | null>(null);
  protected readonly verificationInProgress = signal(false);

  /** The group currently in view, marked in the rail. */
  protected readonly groupInView = signal<string>('appearance');

  private readonly sections = viewChildren<ElementRef<HTMLElement>>('section');

  protected readonly MEMORY_MAX_GB = MEMORY_MAX_GB;

  protected readonly ShieldCheck = ShieldCheck;
  protected readonly FolderOpen = FolderOpen;
  protected readonly Terminal = Terminal;

  protected readonly groups: readonly Group[] = [
    { anchor: 'appearance', label: 'Appearance', icon: SlidersHorizontal },
    { anchor: 'window', label: 'Game window', icon: Monitor },
    { anchor: 'video', label: 'Video', icon: Monitor },
    { anchor: 'java', label: 'Java', icon: Terminal },
    { anchor: 'advanced', label: 'Advanced', icon: MemoryStick },
  ];

  protected readonly backdrops: readonly { value: Backdrop; label: string }[] = [
    { value: 'spawn', label: 'Spawn' },
    { value: 'nether', label: 'Nether' },
    { value: 'end', label: 'End' },
    { value: 'plain', label: 'Plain (no image)' },
  ];

  protected readonly modes: readonly { value: WindowMode; label: string }[] = [
    { value: 'windowed', label: 'Windowed' },
    { value: 'maximized', label: 'Maximized' },
    { value: 'fullscreen', label: 'Fullscreen' },
  ];

  /**
   * The screen size, when known.
   *
   * This is the WORK AREA, desktop panels subtracted — not the raw size: a
   * window the size of the screen would slide under the taskbar.
   */
  protected readonly screenSize = computed(() => {
    const e = this.screen();
    return e ? `${e.width} × ${e.height}` : null;
  });

  protected readonly memoryGb = computed(() => {
    const mb = this.view().launcher.memoryMb;
    return mb === null ? null : Math.round((mb / 1024) * 10) / 10;
  });

  constructor() {
    // The draft follows the service, never the other way around. That's what
    // makes a value clamped to its bounds by Rust show up right away.
    effect(() => this.view.set(this.settings.view()));

    // And the screen follows the draft: dragging the scrim's slider must
    // brighten the image UNDER THE FINGER, while the write only happens on
    // release. Without this, you'd be tuning a darkening blindly.
    effect(() => this.settings.reflect(this.view()));

    // The rail marks the group in view, including when scrolling with the
    // wheel and not by a click. Without this, it would only mark the last one
    // clicked — meaning it would lie from the very first wheel scroll.
    effect((onCleanup) => {
      const sections = this.sections();
      // `IntersectionObserver` is missing from jsdom, where the suites run.
      // Its absence must break nothing: the rail then marks the last group
      // clicked, which is exactly its behavior from before the observer.
      if (sections.length === 0 || typeof IntersectionObserver === 'undefined') {
        return;
      }
      const observer = new IntersectionObserver(
        (entries) => {
          const visible = entries
            .filter((entry) => entry.isIntersecting)
            .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
          const group = (visible?.target as HTMLElement | undefined)?.dataset['group'];
          if (group) {
            this.groupInView.set(group);
          }
        },
        // A band across the upper third of the zone: that's where the eye
        // lands, and it's what keeps a group barely entering from the bottom
        // from taking the mark while you're still reading the previous one.
        { rootMargin: '0px 0px -66% 0px', threshold: [0, 0.25, 0.5] },
      );
      for (const section of sections) {
        observer.observe(section.nativeElement);
      }
      onCleanup(() => observer.disconnect());
    });
  }

  /** The travel percentage of a slider — the design system reads it as `--p`. */
  protected travel(value: number, low: number, high: number): string {
    return `${((value - low) / (high - low)) * 100}%`;
  }

  /**
   * Scrolls to a group.
   *
   * No URL is touched: no anchor, so nothing the router could mistake for a
   * route. `block: 'start'` with the scroll margin set in CSS stops the title
   * below the edge of the zone rather than flush against it.
   */
  protected goTo(anchor: string): void {
    this.groupInView.set(anchor);
    const target = this.sections().find(
      (element) => element.nativeElement.dataset['group'] === anchor,
    );
    target?.nativeElement.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // --- The changes -----------------------------------------------------
  //
  // Two moments, and that's the whole point:
  //
  //   `(input)`  → `change…`: the draft alone. No network, no disk.
  //   `(change)` → `save()`: one write, on release.
  //
  // A `<select>` and a checkbox only have one moment — `(change)` does both
  // at once, since there's no continuous gesture to track.

  protected changeBackdrop(value: string): void {
    this.view.update((v) => ({
      ...v,
      appearance: { ...v.appearance, backdrop: value as Backdrop },
    }));
    void this.save();
  }

  protected changeScrim(value: string): void {
    this.view.update((v) => ({ ...v, appearance: { ...v.appearance, scrim: Number(value) } }));
  }

  protected changeMode(value: string): void {
    this.view.update((v) => ({ ...v, window: { ...v.window, mode: value as WindowMode } }));
    void this.save();
  }

  protected changeSize(field: 'width' | 'height', value: string): void {
    this.view.update((v) => ({ ...v, window: { ...v.window, [field]: Number(value) } }));
    void this.save();
  }

  protected changeGame(field: string, value: string | number): void {
    this.view.update((v) => ({ ...v, game: { ...v.game, [field]: Number(value) } }));
  }

  protected switchGame(field: string, value: boolean): void {
    this.view.update((v) => ({ ...v, game: { ...v.game, [field]: value } }));
    void this.save();
  }

  protected changeMemory(value: string): void {
    this.view.update((v) => ({
      ...v,
      launcher: { ...v.launcher, memoryMb: Math.round(Number(value) * 1024) },
    }));
  }

  protected changeMinimize(value: boolean): void {
    this.view.update((v) => ({ ...v, launcher: { ...v.launcher, minimizeOnLaunch: value } }));
    void this.save();
  }

  /** Writes the draft. Called on release, never during the gesture. */
  protected async save(): Promise<void> {
    await this.incidents.guard(() => this.settings.save(this.view()));
  }

  // --- The Advanced group ---------------------------------------------------

  /**
   * "Verify files" — the gesture that left the main screen.
   *
   * It used to be there, it isn't anymore, and this is where it resurfaces:
   * without this group, it would have no graphical entry point left.
   */
  protected async verify(): Promise<void> {
    this.verificationInProgress.set(true);
    try {
      const issues = await this.incidents.guard(() => this.bridge.verifyFiles(true));
      this.verification.set(issues ?? null);
      if (issues) {
        this.notifications.notify(
          issues.length === 0 ? 'success' : 'warning',
          issues.length === 0 ? 'Verification complete' : `${issues.length} issue(s)`,
          issues.length === 0 ? 'Everything is in place.' : 'The next launch will catch it up.',
        );
      }
    } finally {
      this.verificationInProgress.set(false);
    }
  }

  protected async openFolder(what: Folder): Promise<void> {
    await this.incidents.guard(() => this.bridge.openFolder(what));
  }
}
