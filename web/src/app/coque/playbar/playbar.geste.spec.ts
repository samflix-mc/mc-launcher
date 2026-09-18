import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { EtatDuPack } from '../../noyau/contrats';
import { Pack } from '../../noyau/pack';
import { Pont } from '../../noyau/pont';
import { Playbar } from './playbar';

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

/**
 * **Ce que le clic déclenche réellement.**
 *
 * Le libellé et le geste doivent dire la même chose, et c'est tout l'objet du
 * retour de recette : le bouton annonçait « Mettre à jour et jouer » et lançait
 * Minecraft dans la foulée. Un test sur le seul libellé n'aurait rien attrapé —
 * c'est l'appel qui compte.
 */
describe('Playbar, le geste', () => {
  let installer: ReturnType<typeof vi.fn>;
  let jouer: ReturnType<typeof vi.fn>;
  let arreterLeJeu: ReturnType<typeof vi.fn>;
  let pack: Pack;

  const rendu = {
    verdict: 'Le pack est installé.',
    rattrapee: true,
    introuvables: [],
    ecarts: [],
    horsLigne: false,
    purge: [],
  };

  beforeEach(() => {
    TestBed.resetTestingModule();
    installer = vi.fn(async () => rendu);
    jouer = vi.fn(async () => ({ ...rendu, verdict: 'Partie terminée.' }));
    arreterLeJeu = vi.fn(async () => {});
    TestBed.configureTestingModule({
      providers: [
        {
          provide: Pont,
          // `disponible: false` neutralise le rafraîchissement qui suit le
          // geste : ce test porte sur l'appel, pas sur la relecture du disque.
          useValue: { disponible: true, dansLaFenetre: false, installer, jouer, arreterLeJeu },
        },
      ],
    });
    pack = TestBed.inject(Pack);
  });

  async function cliquer(pose: Partial<EtatDuPack>) {
    pack.etat.set(etat(pose));
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    fixture.nativeElement.querySelector('[data-test="bouton"]').click();
    // Un tour de boucle suffit : le geste est une promesse déjà résolue.
    // `whenStable()` attendrait, lui, la stabilité de toute l'application —
    // qu'un composant de coque à l'écoute d'un flux ne lui rend jamais.
    await new Promise((suite) => setTimeout(suite, 0));
  }

  it('quand il y a quelque chose à poser, il POSE — et ne joue pas', async () => {
    await cliquer({ action: 'installer', ecart: 'mise-a-jour' });

    expect(installer).toHaveBeenCalledOnce();
    expect(jouer).not.toHaveBeenCalled();
  });

  it('quand tout est à jour, il joue', async () => {
    await cliquer({ action: 'jouer', ecart: 'a-jour' });

    expect(jouer).toHaveBeenCalledOnce();
    expect(installer).not.toHaveBeenCalled();
  });

  /** Rien de posé : on installe, évidemment — et là non plus on ne joue pas. */
  it('sur un disque vide, il installe', async () => {
    await cliquer({ action: 'installer', ecart: 'absent', installe: false });

    expect(installer).toHaveBeenCalledOnce();
    expect(jouer).not.toHaveBeenCalled();
  });

  /**
   * **Le second clic seul arrête le jeu.**
   *
   * Le premier pose la question. Tuer Minecraft fait perdre ce qui n'a pas été
   * sauvegardé, et ce bouton occupe le centre de la barre du bas : sans ce
   * temps d'arrêt, un clic distrait coûterait une session de jeu.
   */
  it('l’arrêt du jeu demande deux clics', async () => {
    pack.etat.set(etat());
    pack.avancement.set({
      phase: 'lancement',
      achevee: false,
      note: null,
      fichier: null,
      octets: 0,
      total: 0,
      fichiers: 0,
      fichiersTotal: 0,
      actif: false,
      debit: 0,
      restant: null,
    });
    const fixture = TestBed.createComponent(Playbar);
    fixture.detectChanges();
    const bouton = fixture.nativeElement.querySelector('[data-test="bouton"]');

    bouton.click();
    fixture.detectChanges();
    expect(arreterLeJeu).not.toHaveBeenCalled();

    bouton.click();
    await new Promise((suite) => setTimeout(suite, 0));
    expect(arreterLeJeu).toHaveBeenCalledOnce();
  });
});
