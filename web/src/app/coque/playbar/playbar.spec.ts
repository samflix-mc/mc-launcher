import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import type { Avancement, EtatDuPack } from '../../noyau/contrats';
import { Pack } from '../../noyau/pack';
import { Playbar, nomDeFichier } from './playbar';

/** Un état de pack plausible, dont chaque test ne change que ce qui l'intéresse. */
function etat(dessus: Partial<EtatDuPack> = {}): EtatDuPack {
  return {
    action: 'jouer',
    ecart: 'a-jour',
    horsLigne: false,
    installe: true,
    nom: 'samflix',
    version: '1.4.2',
    java: 21,
    mods: 128,
    generation: 1,
    ...dessus,
  };
}

/** Un avancement plausible, même principe. */
function avancement(dessus: Partial<Avancement> = {}): Avancement {
  return {
    phase: 'mods',
    achevee: false,
    note: null,
    fichier: 'sodium.jar',
    octets: 420_000_000,
    total: 840_000_000,
    fichiers: 64,
    fichiersTotal: 128,
    actif: true,
    debit: 8_400_000,
    restant: 30,
    ...dessus,
  };
}

/**
 * Le bouton, et la phrase au-dessus de lui.
 *
 * Les tests passent par le DOM et les attributs `data-test` : c'est la
 * convention du dépôt, et c'est aussi ce qui fait que ces tests décrivent ce
 * que le joueur LIT plutôt que la forme interne du composant.
 */
describe('Playbar', () => {
  let pack: Pack;

  function monter() {
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    return fixture;
  }

  function lire(fixture: ReturnType<typeof monter>, test: string): string | null {
    const element = fixture.nativeElement.querySelector(`[data-test="${test}"]`);
    return element ? element.textContent.replace(/\s+/g, ' ').trim() : null;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    pack = TestBed.inject(Pack);
  });

  /**
   * **Entre l'ouverture et la réponse de `etat_du_pack()`, le bouton dit qu'il
   * regarde.**
   *
   * Un défaut « Installer » produirait un clignotement INSTALLER → JOUER à
   * chaque démarrage, sur le seul élément de l'écran qui compte.
   */
  it('avant de savoir, il ne dit pas « Installer »', () => {
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Vérification…');
    expect(lire(fixture, 'indication')).toContain('Lecture de ce qui est installé');
  });

  it('rien d’installé : il propose d’installer, et dit pourquoi', () => {
    pack.etat.set(etat({ action: 'installer', ecart: 'absent', installe: false }));
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Installer');
    expect(lire(fixture, 'indication')).toContain('Pas encore installé');
    expect(lire(fixture, 'indication')).toContain('1.4.2');
  });

  it('pack conforme : il propose de jouer', () => {
    pack.etat.set(etat());
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Jouer');
    expect(lire(fixture, 'indication')).toContain('À jour');
  });

  /**
   * **Le libellé DIT ce qui va se passer, et rien de plus.**
   *
   * Il disait « Mettre à jour et jouer », et il faisait les deux : Sam a cliqué
   * pour poser un modpack et Minecraft a démarré. Le bouton pose, s'arrête, et
   * devient « Jouer » — c'est un second clic, quand le joueur le décide.
   *
   * L'`action` vient de Rust et vaut « installer » dès qu'il y a quelque chose
   * à poser ; ce test la pose telle que `mc_pack::comparaison` la rendrait.
   */
  it('mise à jour en attente : le bouton POSE, il ne joue pas', () => {
    pack.etat.set(etat({ action: 'installer', ecart: 'mise-a-jour' }));
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Mettre à jour');
    expect(lire(fixture, 'bouton')).not.toContain('jouer');
    expect(lire(fixture, 'indication')).toContain('Mise à jour disponible');
  });

  it('génération changée : il annonce une réinstallation, et rassure', () => {
    pack.etat.set(etat({ action: 'installer', ecart: 'reinstallation' }));
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Réinstaller');
    expect(lire(fixture, 'bouton')).not.toContain('jouer');
    expect(lire(fixture, 'indication')).toContain('vos mondes');
  });

  /**
   * **Hors ligne ET rien d'installé : il n'y a rien à lancer et rien à
   * télécharger.** Hors ligne avec un pack posé, en revanche, se joue très
   * bien — le launcher sait seulement qu'il n'a pas pu vérifier.
   */
  it('hors ligne sans rien d’installé, le bouton est inerte et dit pourquoi', () => {
    pack.etat.set(etat({ horsLigne: true, installe: false, action: 'installer' }));
    const fixture = monter();

    const bouton = fixture.nativeElement.querySelector('[data-test="bouton"]');
    expect(bouton.disabled).toBe(true);
    expect(bouton.classList.contains('hm-play--disabled')).toBe(true);
    expect(lire(fixture, 'indication')).toContain('Hors ligne');
  });

  it('hors ligne avec un pack posé, on joue quand même', () => {
    pack.etat.set(etat({ horsLigne: true, installe: true }));
    const fixture = monter();

    const bouton = fixture.nativeElement.querySelector('[data-test="bouton"]');
    expect(bouton.disabled).toBe(false);
    expect(lire(fixture, 'indication')).toContain('les mises à jour ne sont pas vérifiées');
  });

  /**
   * Pendant le travail, le bouton porte le remplissage, le pourcentage et le
   * débit — et c'est précisément ce qui a permis de retirer la liste des
   * étapes de la page Spawn.
   */
  it('pendant l’installation, le bouton porte l’avancement', () => {
    pack.etat.set(etat());
    pack.occupe.set(true);
    pack.chemin.set([
      { phase: 'mods', libelle: 'Mods', rang: 0 },
      { phase: 'verrou', libelle: 'Finalisation', rang: 1 },
    ]);
    pack.avancement.set(avancement());
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('Installation…');
    expect(lire(fixture, 'meta')).toContain('Mo/s');
    expect(lire(fixture, 'indication')).toContain('Mods');

    const remplissage = fixture.nativeElement.querySelector('[data-test="remplissage"]');
    expect(remplissage.style.getPropertyValue('--p')).not.toBe('');
  });

  /**
   * **Une progression qu'on ne connaît pas se dit comme telle.**
   *
   * La variante indéterminée glisse ; une barre à zéro pour cent prétendrait
   * savoir qu'on n'a rien fait, ce qui est faux — on n'en sait rien.
   */
  it('quand la progression est inconnue, la barre est indéterminée', () => {
    const fixture = monter();

    const remplissage = fixture.nativeElement.querySelector('[data-test="remplissage"]');
    expect(remplissage.classList.contains('hm-play__fill--indeterminate')).toBe(true);
  });

  /**
   * La partie est le plus long état de la session. Confondre « occupé » et
   * « en jeu » ferait répondre « une installation est déjà en cours » à
   * quelqu'un qui clique pendant qu'il joue.
   */
  it('pendant la partie, le bouton le dit et ne se clique pas', () => {
    pack.etat.set(etat());
    pack.avancement.set(avancement({ phase: 'lancement', actif: false }));
    const fixture = monter();

    expect(lire(fixture, 'bouton')).toContain('En jeu');
    expect(fixture.nativeElement.querySelector('[data-test="bouton"]').disabled).toBe(true);
    expect(lire(fixture, 'indication')).toContain('Le jeu tourne');
  });

  /**
   * Sans version publiée, la phrase ne doit pas porter un point médian
   * orphelin — « Pas encore installé · » se lit comme une valeur manquante.
   */
  it('sans version connue, la phrase ne traîne pas de séparateur', () => {
    pack.etat.set(etat({ ecart: 'absent', version: null, installe: false }));
    const fixture = monter();

    expect(lire(fixture, 'indication')).toBe('Pas encore installé');
  });

  /**
   * **Ce qu'on télécharge, et à quelle vitesse.**
   *
   * Le retour de recette était : « on sait à peu près à quelle étape on est,
   * mais on ne sait pas ce qu'on télécharge, quelle est la vitesse, ni
   * l'avancée en pourcentage ». Toutes ces valeurs étaient émises cinq fois par
   * seconde par `Suivi` — aucune n'était affichée.
   */
  it('pendant l’installation, la seconde ligne dit tout ce qu’on sait', () => {
    pack.etat.set(etat({ action: 'installer', ecart: 'mise-a-jour' }));
    pack.occupe.set(true);
    pack.avancement.set(avancement({ fichier: '/long/chemin/vers/sodium.jar' }));
    const fixture = monter();

    const ligne = lire(fixture, 'detail') ?? '';
    expect(ligne).toContain('sodium.jar');
    expect(ligne).not.toContain('/long/chemin');
    expect(ligne).toContain('64 sur 128 fichiers');
    expect(ligne).toContain('sur 840');
    expect(ligne).toContain('/s');
    expect(ligne).toContain('restantes');
  });

  /**
   * **Le pourcentage survit aux creux entre deux lots.**
   *
   * `actif` retombe à faux pendant la résolution des mods, l'inspection des
   * jars et l'installateur NeoForge — soit une bonne part du temps. Le bouton
   * n'affichait alors plus rien du tout, ce qui se lit comme un blocage.
   *
   * Le débit, lui, disparaît : il n'a aucun sens quand rien ne descend, et un
   * chiffre figé y ressemblerait à une mesure.
   */
  it('entre deux lots, le pourcentage reste et le débit s’en va', () => {
    pack.etat.set(etat({ action: 'installer', ecart: 'mise-a-jour' }));
    pack.occupe.set(true);
    pack.chemin.set([{ phase: 'mods', libelle: 'Mods', rang: 0 }]);
    pack.avancement.set(avancement({ actif: false, debit: 0 }));
    const fixture = monter();

    expect(lire(fixture, 'meta')).toMatch(/\d+ %/);
    expect(lire(fixture, 'meta')).not.toContain('/s');
  });

  /**
   * Pendant la VÉRIFICATION, il n'y a ni pourcentage ni détail à montrer : on
   * n'a encore rien mesuré. La phrase du haut le dit, et c'est tout.
   */
  it('pendant la vérification, aucun chiffre n’est inventé', () => {
    const fixture = monter();

    expect(lire(fixture, 'indication')).toContain('Lecture de ce qui est installé');
    expect(lire(fixture, 'meta')).toBeNull();
    expect(lire(fixture, 'detail')).toBeNull();
  });
});

/**
 * Le nom seul d'un chemin.
 *
 * Testé à part parce que c'est une fonction libre : elle n'a besoin ni d'un
 * composant, ni d'un DOM.
 */
describe('nomDeFichier', () => {
  it('ne garde que la dernière portion', () => {
    expect(nomDeFichier('/a/b/c/sodium.jar')).toBe('sodium.jar');
    expect(nomDeFichier('sodium.jar')).toBe('sodium.jar');
    expect(nomDeFichier('C:\\jeux\\mods\\iris.jar')).toBe('iris.jar');
  });

  /** Un chemin qui finit par un séparateur ne doit pas rendre une chaîne vide. */
  it('un chemin qui finit par un séparateur rend le chemin', () => {
    expect(nomDeFichier('/a/b/')).toBe('/a/b/');
  });
});
