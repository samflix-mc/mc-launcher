import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Incidents } from '../../noyau/incidents';
import { Incident } from './incident';

/**
 * Le dialogue d'incident.
 *
 * Ce qui s'est passé, puis la marche à suivre ; le détail technique — ce que
 * Rust a rendu — est ce qu'on demande de recopier, donc sélectionnable.
 */
describe('Incident', () => {
  let incidents: Incidents;

  function monter() {
    const fixture = TestBed.createComponent(Incident);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    incidents = TestBed.inject(Incidents);
  });

  it('sans incident, rien ne s’affiche', () => {
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).toBeNull();
  });

  it('un incident montre son détail, et ce qu’il faut en faire', () => {
    incidents.signaler('connexion Microsoft : le code a expiré');
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="message"]').textContent).toContain(
      'le code a expiré',
    );
  });

  /**
   * Le détail est ce qu'on demande au joueur de recopier : il doit pouvoir le
   * sélectionner, là où le reste de la fenêtre ne le permet pas.
   */
  it('le détail est sélectionnable', () => {
    incidents.signaler('une erreur');
    const fixture = monter();

    expect(
      fixture.nativeElement
        .querySelector('[data-test="message"]')
        .classList.contains('hm-selectionnable'),
    ).toBe(true);
  });

  it('fermer referme', () => {
    incidents.signaler('une erreur');
    const fixture = monter();

    fixture.nativeElement.querySelector('[data-test="fermer"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="incident"]')).toBeNull();
  });
});
