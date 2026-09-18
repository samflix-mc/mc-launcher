import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Incidents } from '../../core/incidents';
import { Incident } from './incident';

/**
 * The incident dialog.
 *
 * What happened, then what to do about it; the technical detail — what Rust
 * returned — is what we ask to be copied, hence selectable.
 */
describe('Incident', () => {
  let incidents: Incidents;

  function mount() {
    const fixture = TestBed.createComponent(Incident);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    incidents = TestBed.inject(Incidents);
  });

  it('with no incident, nothing shows', () => {
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).toBeNull();
  });

  it('an incident shows its detail, and what to do about it', () => {
    incidents.report('Microsoft sign-in: the code expired');
    const fixture = mount();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="message"]').textContent).toContain(
      'the code expired',
    );
  });

  /**
   * The detail is what we ask the player to copy: it must be selectable,
   * where the rest of the window doesn't allow it.
   */
  it('the detail is selectable', () => {
    incidents.report('an error');
    const fixture = mount();

    expect(
      fixture.nativeElement
        .querySelector('[data-test="message"]')
        .classList.contains('hm-selectionnable'),
    ).toBe(true);
  });

  it('close closes it', () => {
    incidents.report('an error');
    const fixture = mount();

    fixture.nativeElement.querySelector('[data-test="close"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).toBeNull();
  });
});
