import { TestBed } from '@angular/core/testing';

import { App } from './app';
import {
  Launcher,
  messageDErreur,
  type Avancement,
  type Compte,
  type EtapeVue,
  type Installation,
} from './launcher';

const CHEMIN: EtapeVue[] = [
  { phase: 'connexion', libelle: 'Compte Microsoft', rang: 0 },
  { phase: 'licence', libelle: 'Licence Minecraft', rang: 1 },
  { phase: 'minecraft', libelle: 'Fichiers du jeu', rang: 2 },
  { phase: 'mods', libelle: 'Mods', rang: 3 },
  { phase: 'pret', libelle: 'Prêt à jouer', rang: 4 },
];

function avancement(partiel: Partial<Avancement> = {}): Avancement {
  return {
    phase: 'minecraft',
    note: null,
    fichier: null,
    octets: 0,
    total: 0,
    fichiers: 0,
    fichiersTotal: 0,
    debit: 0,
    restant: null,
    ...partiel,
  };
}

/**
 * Un `Launcher` qui ne parle à personne.
 *
 * Le vrai passe par `invoke`, qui n'existe pas hors de la fenêtre Tauri : la
 * suite vérifie ce que le composant fait de ce qu'il reçoit, pas le pont.
 */
class LauncherDEssai implements Partial<Launcher> {
  disponible = true;
  compte: Compte | null = null;
  refus: unknown = null;
  installation: Installation | null = null;

  /** De quoi pousser un avancement depuis le test. */
  pousser: (a: Avancement) => void = () => {};

  chemin(): Promise<EtapeVue[]> {
    return Promise.resolve(CHEMIN);
  }

  statut(): Promise<Compte | null> {
    return this.refus ? Promise.reject(this.refus) : Promise.resolve(this.compte);
  }

  surAvancement(recevoir: (a: Avancement) => void): Promise<() => void> {
    this.pousser = recevoir;
    return Promise.resolve(() => {});
  }

  installer(): Promise<Installation> {
    return this.installation
      ? Promise.resolve(this.installation)
      : Promise.reject('rien à installer');
  }
}

async function monter(launcher: LauncherDEssai) {
  TestBed.configureTestingModule({
    imports: [App],
    providers: [{ provide: Launcher, useValue: launcher }],
  });
  const fixture = TestBed.createComponent(App);
  await fixture.whenStable();
  return fixture;
}

function texte(fixture: { nativeElement: unknown }): string {
  return (fixture.nativeElement as HTMLElement).textContent ?? '';
}

describe('App', () => {
  it("propose la connexion quand personne n'est connecté", async () => {
    const fixture = await monter(new LauncherDEssai());

    expect(texte(fixture)).toContain('Se connecter avec Microsoft');
    expect(texte(fixture)).not.toContain('Se déconnecter');
  });

  it('affiche le profil de la session reprise', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abcdef', possedeLeJeu: true };

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('Sam');
    expect(texte(fixture)).toContain('abcdef');
    expect(texte(fixture)).toContain('Se déconnecter');
  });

  it("n'offre pas d'installer à un compte sans licence", async () => {
    // Installer huit cents mégaoctets pour un compte qu'aucun serveur
    // n'acceptera est précisément ce qu'on veut éviter.
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abcdef', possedeLeJeu: false };

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('ne possède pas Minecraft');
    expect(texte(fixture)).not.toContain('Installer le pack');
  });

  it('montre une session expirée comme un message, pas comme un écran vide', async () => {
    const launcher = new LauncherDEssai();
    launcher.refus = "la session enregistrée n'est plus valable — reconnecte-toi";

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain("n'est plus valable");
    // Et le bouton reste utilisable, sinon il n'y a plus de sortie.
    expect(texte(fixture)).toContain('Se connecter avec Microsoft');
  });

  it('avertit quand la fenêtre Tauri est absente', async () => {
    const launcher = new LauncherDEssai();
    launcher.disponible = false;

    const fixture = await monter(launcher);
    const element = fixture.nativeElement as HTMLElement;

    expect(element.textContent).toContain('hors de la fenêtre Tauri');
    expect(element.querySelector<HTMLButtonElement>('.bouton--principal')?.disabled).toBe(true);
  });

  it('dessine le chemin entier dès le départ', async () => {
    // C'est ce qui distingue « on en est à la moitié » de « il se passe
    // quelque chose » : le joueur voit ce qui reste.
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };

    const fixture = await monter(launcher);
    const etapes = (fixture.nativeElement as HTMLElement).querySelectorAll('.etape');

    expect(etapes.length).toBe(CHEMIN.length);
  });

  it('éclaire la phase en cours et marque celles qui sont passées', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };
    const fixture = await monter(launcher);

    launcher.pousser(avancement({ phase: 'minecraft' }));
    await fixture.whenStable();

    const etats = Array.from(
      (fixture.nativeElement as HTMLElement).querySelectorAll('.etape'),
    ).map((etape) => etape.getAttribute('data-etat'));

    expect(etats).toEqual(['faite', 'faite', 'en-cours', 'a-venir', 'a-venir']);
  });

  it('montre la connexion et la licence acquises avant toute installation', async () => {
    // Le compte est là et rien ne tourne encore : laisser ces deux phases
    // grises ferait croire qu'il reste à les faire.
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };

    const fixture = await monter(launcher);
    const etats = Array.from(
      (fixture.nativeElement as HTMLElement).querySelectorAll('.etape'),
    ).map((etape) => etape.getAttribute('data-etat'));

    // Aucune n'est « en cours » : rien ne travaille.
    expect(etats).toEqual(['faite', 'faite', 'a-venir', 'a-venir', 'a-venir']);
  });

  it('affiche la taille, le débit et le temps restant pendant un téléchargement', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };
    const fixture = await monter(launcher);

    launcher.pousser(
      avancement({
        phase: 'minecraft',
        octets: 415_000_000,
        total: 830_000_000,
        fichiers: 1_200,
        fichiersTotal: 2_500,
        debit: 8_200_000,
        restant: 51,
        fichier: 'a3f1b2c4',
      }),
    );
    await fixture.whenStable();

    const affiche = texte(fixture);
    expect(affiche).toContain('415 Mo');
    expect(affiche).toContain('830 Mo');
    expect(affiche).toContain('1200');
    expect(affiche).toContain('2500');
    expect(affiche).toContain('8,2 Mo/s');
    expect(affiche).toContain('51 s');
    expect(affiche).toContain('a3f1b2c4');
  });

  it('remplit la barre à la proportion reçue', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };
    const fixture = await monter(launcher);

    launcher.pousser(avancement({ octets: 250, total: 1_000 }));
    await fixture.whenStable();

    const barre = (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(
      '.jauge__barre',
    );
    expect(barre?.style.width).toBe('25%');
  });

  it("n'affiche aucune jauge une fois l'installation finie", async () => {
    // « Prêt » n'est pas un téléchargement : une barre figée à cent pour cent
    // laisserait croire qu'il reste quelque chose en cours.
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };
    const fixture = await monter(launcher);

    launcher.pousser(avancement({ phase: 'pret', octets: 1_000, total: 1_000 }));
    await fixture.whenStable();

    expect((fixture.nativeElement as HTMLElement).querySelector('.jauge')).toBeNull();
  });

  it('laisse « Jouer » éteint tant que rien n’est installé', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: true };

    const fixture = await monter(launcher);
    const boutons = Array.from(
      (fixture.nativeElement as HTMLElement).querySelectorAll<HTMLButtonElement>('button'),
    );
    const jouer = boutons.find((bouton) => bouton.textContent?.includes('Lancer le jeu'));

    expect(jouer?.disabled).toBe(true);
  });
});

describe('messageDErreur', () => {
  it('rend telle quelle la chaîne sérialisée par Rust', () => {
    expect(messageDErreur('connexion Microsoft : le code a expiré')).toBe(
      'connexion Microsoft : le code a expiré',
    );
  });

  it("n'affiche jamais [object Object]", () => {
    // Ce qui vient du pont lui-même n'est pas notre `Erreur`.
    expect(messageDErreur(new Error('pont rompu'))).toBe('pont rompu');
    expect(messageDErreur({ inattendu: true })).not.toContain('object Object');
  });
});
