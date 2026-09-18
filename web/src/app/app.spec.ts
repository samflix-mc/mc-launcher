import { TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { App } from './app';
import { ROUTES } from './routes';
import { Session } from './noyau/session';

/**
 * La coque.
 *
 * Trois mises en page, et c'est la route qui décide : la fenêtre de connexion,
 * la page à trois rangs — navigation, contenu, barre du bas — et celle à deux,
 * qui laisse tomber la barre du bas et donne sa hauteur au corps.
 */
describe('App', () => {
  let router: Router;
  let session: Session;

  /**
   * Monte la coque ET passe l'amorce.
   *
   * L'amorce tient un PLANCHER de quatre cents millisecondes — sans lui, une
   * session déjà en cache la ferait clignoter le temps d'une image. Un test qui
   * ne l'attendrait pas ne verrait jamais que l'écran de démarrage, et
   * conclurait que la coque ne se dessine pas.
   */
  async function monter() {
    const fixture = TestBed.createComponent(App);
    fixture.detectChanges();
    await new Promise((suite) => setTimeout(suite, 600));
    await fixture.whenStable();
    fixture.detectChanges();
    return fixture;
  }

  /**
   * Va sur une route et laisse la coque se redessiner.
   *
   * Le compte est reposé JUSTE AVANT : hors de tout backend, `session.ouvrir()`
   * remet le compte à nul — c'est son comportement voulu, et `demarrer()`
   * l'appelle au montage. Sans ce rappel, les gardes trouveraient une session
   * vide et renverraient tout vers `/connexion`.
   */
  async function aller(fixture: Awaited<ReturnType<typeof monter>>, url: string, connecte = true) {
    session.compte.set(
      connecte ? { pseudo: 'thesam1798', uuid: '0123', possedeLeJeu: true } : null,
    );
    await router.navigateByUrl(url);
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideRouter(ROUTES)] });
    router = TestBed.inject(Router);
    session = TestBed.inject(Session);
    session.compte.set({ pseudo: 'thesam1798', uuid: '0123', possedeLeJeu: true });
  });

  /**
   * **Hors de tout backend, l'écran le dit plutôt que d'échouer** sur un
   * `invoke` qui n'existe pas. C'est le seul cas où cet avertissement paraît.
   */
  it('sans backend joignable, la fenêtre le dit', async () => {
    const fixture = await monter();

    expect(fixture.nativeElement.querySelector('[data-test="hors-tauri"]')).not.toBeNull();
  });

  it('la barre de titre est là dès la première image', async () => {
    const fixture = await monter();

    expect(fixture.nativeElement.querySelector('[data-test="barre-titre"]')).not.toBeNull();
  });

  /**
   * Spawn porte les trois rangs : la navigation, le contenu, et une barre du
   * bas qui tient le bouton de jeu ET le badge joueur.
   */
  it('sur Spawn, la page a trois rangs et le bouton de jeu', async () => {
    const fixture = await monter();
    await aller(fixture, '/spawn');

    const page = fixture.nativeElement.querySelector('[data-test="page"]');
    expect(page.classList.contains('hm-page--trois-rangs')).toBe(true);
    expect(fixture.nativeElement.querySelector('[data-test="nav"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="playbar"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="badge-joueur"]')).not.toBeNull();
  });

  /** Les Nouvelles gardent le badge joueur, et rien au centre. */
  it('sur les Nouvelles, le badge reste mais pas le bouton', async () => {
    const fixture = await monter();
    await aller(fixture, '/nouvelles');

    expect(fixture.nativeElement.querySelector('[data-test="barre-basse"]')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="playbar"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="badge-joueur"]')).not.toBeNull();
  });

  /**
   * **La Configuration laisse tomber la barre du bas**, et le corps prend la
   * hauteur : c'est une règle explicite du design system, et c'est ce qui donne
   * à la liste de réglages de quoi défiler.
   */
  it('sur la Configuration, la barre du bas disparaît', async () => {
    const fixture = await monter();
    await aller(fixture, '/configuration');

    const page = fixture.nativeElement.querySelector('[data-test="page"]');
    expect(page.classList.contains('hm-page--deux-rangs')).toBe(true);
    expect(fixture.nativeElement.querySelector('[data-test="barre-basse"]')).toBeNull();
  });

  /**
   * **La connexion n'a pas de coque du tout.**
   *
   * Ni navigation, ni bouton de jeu, ni badge joueur : montrer un menu et un
   * bouton « se déconnecter » à quelqu'un qui n'est pas connecté était le
   * premier reproche de la recette.
   */
  it('sur la connexion, il n’y a ni navigation ni barre du bas', async () => {
    session.compte.set(null);
    const fixture = await monter();
    await aller(fixture, '/connexion', false);

    expect(fixture.nativeElement.querySelector('[data-test="page"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="nav"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="barre-basse"]')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-test="feuille"]')).not.toBeNull();
  });

  /**
   * La fenêtre de connexion ne se redimensionne pas : laisser le curseur
   * promettre un geste que rien n'exécute est pire que de ne rien promettre.
   */
  it('sur la connexion, les bords ne promettent pas de redimensionnement', async () => {
    session.compte.set(null);
    const fixture = await monter();
    await aller(fixture, '/connexion', false);

    expect(fixture.nativeElement.querySelector('[data-test="bords"]')).toBeNull();
  });

  /** Ailleurs, ils sont là — c'est ce qui manquait au curseur. */
  it('ailleurs, les huit bords portent le curseur', async () => {
    const fixture = await monter();
    await aller(fixture, '/spawn');

    const bords = fixture.nativeElement.querySelector('[data-test="bords"]');
    expect(bords).not.toBeNull();
    expect(bords.children.length).toBe(8);
  });

  /** La scène porte l'image : c'est elle qui donne au verre quelque chose à flouter. */
  it('la scène est toujours là, sous tout le reste', async () => {
    const fixture = await monter();

    expect(fixture.nativeElement.querySelector('[data-test="scene"]')).not.toBeNull();
  });
});
