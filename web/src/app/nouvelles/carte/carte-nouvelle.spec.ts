import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Billet } from '../../noyau/contrats';
import { CarteNouvelle } from './carte-nouvelle';

function billet(dessus: Partial<Billet> = {}): Billet {
  return {
    id: 'lancement',
    titre: 'Le launcher est là',
    date: new Date().toISOString(),
    epinglee: false,
    image: null,
    corps: [
      { type: 'titre', niveau: 2, contenu: [{ type: 'texte', texte: 'Un titre' }] },
      {
        type: 'paragraphe',
        contenu: [
          { type: 'texte', texte: 'Le launcher installe le pack ' },
          { type: 'gras', texte: 'et lance le jeu' },
          { type: 'texte', texte: ' en un seul geste.' },
        ],
      },
    ],
    ...dessus,
  };
}

/**
 * La tuile d'un billet, dans ses trois tailles.
 *
 * Le design system les décrit comme des modificateurs de la même chose ; ces
 * tests gardent le fait qu'elles restent une seule chose.
 */
describe('CarteNouvelle', () => {
  function monter(entrees: { billet: Billet; variante?: string; commeLien?: boolean }) {
    const fixture = TestBed.createComponent(CarteNouvelle);
    fixture.componentRef.setInput('billet', entrees.billet);
    if (entrees.variante) {
      fixture.componentRef.setInput('variante', entrees.variante);
    }
    if (entrees.commeLien !== undefined) {
      fixture.componentRef.setInput('commeLien', entrees.commeLien);
    }
    fixture.detectChanges();
    return fixture;
  }

  function tuile(fixture: ReturnType<typeof monter>): HTMLElement {
    return fixture.nativeElement.querySelector('[data-test="carte-nouvelle"]');
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter([])] });
  });

  /**
   * **Le liseré doré n'est porté que par la tuile épinglée.**
   *
   * Le design system n'en autorise qu'un par écran : c'est la seule chose mise
   * en avant, et deux le seraient donc zéro.
   */
  it('la tuile épinglée porte le liseré doré, les autres non', () => {
    const doree = tuile(monter({ billet: billet(), variante: 'epinglee' }));
    expect(doree.className).toContain('hm-glass--gold');
    expect(doree.className).toContain('hm-news--pinned');

    const ordinaire = tuile(monter({ billet: billet(), variante: 'tuile' }));
    expect(ordinaire.className).not.toContain('hm-glass--gold');
  });

  it('la tuile vedette prend sa classe', () => {
    expect(tuile(monter({ billet: billet(), variante: 'vedette' })).className).toContain(
      'hm-news--featured',
    );
  });

  /**
   * **L'extrait vient du premier PARAGRAPHE**, et non des premiers caractères
   * du corps : un titre en tête donnerait un extrait qui ne ressemble à rien.
   */
  it('l’extrait est le premier paragraphe, aplati', () => {
    const fixture = monter({ billet: billet(), variante: 'vedette' });

    const extrait = fixture.nativeElement.querySelector('[data-test="extrait"]');
    expect(extrait.textContent.trim()).toBe(
      'Le launcher installe le pack et lance le jeu en un seul geste.',
    );
  });

  /** Sur une petite tuile, il n'y a pas la place : le titre suffit. */
  it('la petite tuile ne porte pas d’extrait', () => {
    const fixture = monter({ billet: billet(), variante: 'tuile' });

    expect(fixture.nativeElement.querySelector('[data-test="extrait"]')).toBeNull();
  });

  /** Un billet sans paragraphe ne doit pas rendre un extrait vide mais présent. */
  it('sans paragraphe, aucun extrait', () => {
    const fixture = monter({
      billet: billet({ corps: [{ type: 'separateur' }] }),
      variante: 'vedette',
    });

    expect(fixture.nativeElement.querySelector('[data-test="extrait"]')).toBeNull();
  });

  /**
   * **L'illustration passe par une propriété personnalisée.**
   *
   * Angular assainit les valeurs de style qu'il lie, et une `url(data:…)` de
   * plusieurs centaines de kilooctets est exactement le genre de valeur qu'un
   * assainisseur remplace par du vide — sans erreur.
   */
  it('l’illustration arrive par une propriété personnalisée', () => {
    const fixture = monter({ billet: billet({ image: 'data:image/webp;base64,AAAA' }) });

    const media = fixture.nativeElement.querySelector('[data-test="media"]');
    expect(media.style.getPropertyValue('--illustration')).toContain('data:image/webp');
    expect(media.classList.contains('hm-news__media--ph')).toBe(false);
  });

  it('sans illustration, le dégradé de remplacement tient la place', () => {
    const fixture = monter({ billet: billet() });

    const media = fixture.nativeElement.querySelector('[data-test="media"]');
    expect(media.classList.contains('hm-news__media--ph')).toBe(true);
  });

  /**
   * La tuile est un LIEN quand elle mène ailleurs, un BOUTON quand elle ouvre
   * ici. Un lien vers la page où l'on est déjà ne ferait rien du tout.
   */
  it('elle est un lien, ou un bouton qui émet', () => {
    expect(tuile(monter({ billet: billet() })).tagName).toBe('A');

    const fixture = monter({ billet: billet(), commeLien: false });
    expect(tuile(fixture).tagName).toBe('BUTTON');

    let ouvert: Billet | null = null;
    fixture.componentInstance.ouvrir.subscribe((recu) => (ouvert = recu));
    tuile(fixture).click();
    expect(ouvert).not.toBeNull();
  });

  it('un billet épinglé porte son étiquette', () => {
    const fixture = monter({ billet: billet({ epinglee: true }) });

    expect(fixture.nativeElement.querySelector('[data-test="epinglee"]').textContent).toContain(
      'Épinglée',
    );
  });
});
