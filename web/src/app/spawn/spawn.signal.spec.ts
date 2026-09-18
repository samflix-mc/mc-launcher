import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Fenetre } from '../noyau/fenetre';
import { Pont } from '../noyau/pont';
import { Spawn } from './spawn';

/**
 * **Qui a le droit de dire que la fenêtre principale est prête.**
 *
 * Rust attend ce signal pour échanger les fenêtres à la fin d'une connexion.
 * Il a été envoyé pendant des jours par la fenêtre de CONNEXION : elle
 * recevait un événement qui ne lui était pas destiné — voir
 * `transport.cible.spec.ts` — montait Spawn dans ses quatre cent quarante
 * pixels, et annonçait donc qu'une fenêtre était prête. La mauvaise.
 *
 * Le ciblage est corrigé en amont. Cette garde-ci est la seconde ligne : même
 * si une fenêtre dédiée montait Spawn pour une raison qu'on n'a pas prévue,
 * elle ne parlerait pas au nom de la principale.
 */
describe('Spawn, le signal de fenêtre prête', () => {
  let principalePrete: ReturnType<typeof vi.fn>;

  function monter(dansUneFenetreDediee: boolean) {
    principalePrete = vi.fn(async () => {});
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: Pont,
          useValue: {
            disponible: false,
            dansLaFenetre: false,
            principalePrete,
            journal: vi.fn(async () => {}),
          },
        },
        {
          provide: Fenetre,
          useValue: { dansUneFenetreDediee, estPrincipale: !dansUneFenetreDediee },
        },
      ],
    });
    const fixture = TestBed.createComponent(Spawn);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
  });

  it('dans la fenêtre principale, il part', async () => {
    const fixture = monter(false);
    await fixture.whenStable();

    expect(principalePrete).toHaveBeenCalled();
  });

  it('dans une fenêtre dédiée, il ne part PAS', async () => {
    const fixture = monter(true);
    await fixture.whenStable();

    expect(principalePrete).not.toHaveBeenCalled();
  });
});
