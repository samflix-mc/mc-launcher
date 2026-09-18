import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Billet, Fil } from '../noyau/contrats';
import { Nouvelles } from '../noyau/nouvelles';
import { PageNouvelles } from './nouvelles';

function billet(dessus: Partial<Billet> = {}): Billet {
  return {
    id: 'un',
    titre: 'Le launcher est là',
    date: new Date().toISOString(),
    epinglee: false,
    image: null,
    corps: [{ type: 'paragraphe', contenu: [{ type: 'texte', texte: 'Un billet.' }] }],
    ...dessus,
  };
}

function fil(dessus: Partial<Fil> = {}): Fil {
  return { billets: [billet()], horsLigne: false, ecartes: [], ...dessus };
}

/**
 * Le fil de nouvelles : une tuile vedette, puis une grille.
 *
 * L'écart assumé avec le design system est la DESTINATION : chez lui une tuile
 * ouvre le site du serveur, ici elle ouvre un dialogue de lecture — le corps
 * est analysé par Rust en arbre typé, et c'est la condition à laquelle le CSP
 * a été desserré.
 */
describe('PageNouvelles', () => {
  let nouvelles: Nouvelles;

  function monter() {
    const fixture = TestBed.createComponent(PageNouvelles);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
    nouvelles = TestBed.inject(Nouvelles);
  });

  it('un fil vide le dit, sans prétendre à une panne', () => {
    nouvelles.fil.set(fil({ billets: [] }));
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="aucune"]').textContent).toContain(
      'Rien de publié',
    );
  });

  /**
   * **La vedette apparaît AUSSI dans la grille.**
   *
   * C'est la règle du design system : retirer le billet mis en avant ferait un
   * trou dans la chronologie, et l'on chercherait longtemps pourquoi il manque.
   */
  it('la vedette reste dans la grille', () => {
    nouvelles.fil.set(fil({ billets: [billet({ id: 'a' }), billet({ id: 'b' })] }));
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="vedette"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelectorAll('[data-test="tuile"]').length).toBe(2);
  });

  /**
   * Un fil servi depuis la copie doit le dire. Une page de nouvelles d'hier
   * vaut mieux qu'une page vide, mais une page qui mentirait sur sa fraîcheur
   * serait pire que les deux.
   */
  it('un fil hors ligne l’annonce', () => {
    nouvelles.fil.set(fil({ horsLigne: true }));
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="hors-ligne"]').textContent).toContain(
      'hors ligne',
    );
  });

  /** Un fil à moitié fautif se remarque, discrètement, pour qui publie. */
  it('les billets écartés sont comptés', () => {
    nouvelles.fil.set(fil({ ecartes: ['date illisible', 'titre absent'] }));
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="ecartes"]').textContent).toContain('2');
  });

  it('cliquer une tuile ouvre le billet ici, pas ailleurs', () => {
    nouvelles.fil.set(fil());
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="lecture"]')).toBeNull();

    fixture.nativeElement.querySelector('[data-test="vedette"] button').click();
    fixture.detectChanges();

    const lecture = fixture.nativeElement.querySelector('[data-test="lecture"]');
    expect(lecture).not.toBeNull();
    expect(lecture.textContent).toContain('Le launcher est là');
  });

  it('le dialogue de lecture se referme', () => {
    nouvelles.fil.set(fil());
    const fixture = monter();
    fixture.nativeElement.querySelector('[data-test="vedette"] button').click();
    fixture.detectChanges();

    fixture.nativeElement.querySelector('[data-test="fermer-lecture"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="lecture"]')).toBeNull();
  });

  /**
   * Trois tuiles de remplacement plutôt qu'un texte : la grille garde sa forme,
   * et rien ne saute quand les vraies arrivent.
   */
  it('pendant le chargement, la grille garde sa forme', () => {
    nouvelles.chargement.set(true);
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="chargement"]')).not.toBeNull();
  });
});
