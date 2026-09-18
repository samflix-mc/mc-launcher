import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Amorce } from './amorce';
import { PHRASES, unePhrase } from './phrases';

/**
 * L'écran de démarrage, dans la fenêtre.
 *
 * Il porte le MÊME dessin que la fenêtre d'écran de démarrage de Tauri et que
 * l'amorce statique d'`index.html` : aucun passage entre les trois ne se voit,
 * et il n'y a qu'un dessin à tenir à jour.
 */
describe('Amorce', () => {
  function monter(phrase?: string) {
    const fixture = TestBed.createComponent(Amorce);
    if (phrase !== undefined) {
      fixture.componentRef.setInput('phrase', phrase);
    }
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
  });

  it('elle porte le nom du launcher et une phrase', () => {
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="nom"]').textContent.trim()).not.toBe(
      '',
    );
    expect(fixture.nativeElement.querySelector('[data-test="phrase"]').textContent.trim()).not.toBe(
      '',
    );
  });

  /** L'appelant peut dire quelque chose de précis plutôt qu'une phrase au sort. */
  it('une phrase donnée l’emporte sur le tirage', () => {
    const fixture = monter('Vérification de la session…');

    expect(fixture.nativeElement.querySelector('[data-test="phrase"]').textContent).toContain(
      'Vérification de la session…',
    );
  });

  /**
   * **La barre est indéterminée**, et c'est une affirmation qu'on peut tenir :
   * à cet instant, l'amorce attend le RÉSEAU, et personne ne sait combien de
   * temps cela prend. Une barre qui monterait à un rythme inventé mentirait.
   */
  it('la barre ne prétend pas savoir où l’on en est', () => {
    const fixture = monter();

    expect(
      fixture.nativeElement
        .querySelector('[data-test="progression"]')
        .classList.contains('hm-progress__fill--indeterminate'),
    ).toBe(true);
  });

  /** Elle annonce son attente aux technologies d'assistance. */
  it('elle est annoncée comme un statut', () => {
    const amorce = monter().nativeElement.querySelector('[data-test="amorce"]');

    expect(amorce.getAttribute('role')).toBe('status');
    expect(amorce.getAttribute('aria-live')).toBe('polite');
  });
});

describe('unePhrase', () => {
  it('rend toujours une phrase de la liste', () => {
    for (let essai = 0; essai < 50; essai += 1) {
      expect(PHRASES).toContain(unePhrase());
    }
  });
});
