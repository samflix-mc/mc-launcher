import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Notifications as Service } from '../../core/notifications';
import { Notifications } from './notifications';

/**
 * The two visible levels: toasts, and the center.
 *
 * A single component carries both, because they share the same vocabulary —
 * same tone, same icon, same title. These tests keep that sharing.
 */
describe('Notifications (component)', () => {
  let service: Service;

  function mount() {
    const fixture = TestBed.createComponent(Notifications);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(Service);
  });

  it('with nothing to say, it draws no toast', () => {
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="toasts"]')).toBeNull();
  });

  it('a notified notice appears as a toast, with its title and its detail', () => {
    service.notify('success', 'Pack updated', '128 mods');
    const fixture = mount();

    const toast = fixture.nativeElement.querySelector('[data-test="toast"]');
    expect(toast.textContent).toContain('Pack updated');
    expect(toast.textContent).toContain('128 mods');
    expect(toast.dataset.ton).toBe('success');
  });

  /**
   * A danger toast calls for a decision: it's announced as an alert, where
   * the others settle for a status that a screen reader doesn't interrupt
   * to read.
   */
  it('a danger toast is an alert, the others a status', () => {
    service.notify('danger', 'Failed');
    service.notify('info', 'For your information');
    const fixture = mount();

    const roles = [...fixture.nativeElement.querySelectorAll('[data-test="toast"]')].map(
      (element: Element) => element.getAttribute('role'),
    );
    expect(roles).toContain('alert');
    expect(roles).toContain('status');
  });

  it('the center is closed until the bell has been clicked', () => {
    service.notify('info', 'One');
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="center"]')).toBeNull();
  });

  it('open, the center lists what has been said', () => {
    service.archive('danger', 'Something went wrong', 'the detail');
    service.togglePanel();
    const fixture = mount();

    const rows = fixture.nativeElement.querySelectorAll('[data-test="row"]');
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain('the detail');
  });

  /** An empty center says so, rather than showing a frame with no content. */
  it('an empty center says so', () => {
    service.togglePanel();
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="center-empty"]').textContent).toContain(
      'Nothing to report',
    );
  });

  it('clear all empties the center', () => {
    service.archive('info', 'One');
    service.togglePanel();
    const fixture = mount();

    fixture.nativeElement.querySelector('[data-test="clear-all"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelectorAll('[data-test="row"]').length).toBe(0);
  });

  /** Hiding a toast removes it from the screen, never from the center. */
  it('hiding a toast leaves it in the center', () => {
    service.notify('info', 'One');
    const fixture = mount();

    fixture.nativeElement.querySelector('[data-test="close-toast"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="toast"]')).toBeNull();
    expect(service.log().length).toBe(1);
  });
});
