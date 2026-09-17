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

/** Un chemin réduit, mais de même forme que le vrai : 3 étapes + 2 aboutissements. */
const CHEMIN: EtapeVue[] = [
  { phase: 'connexion', libelle: 'Compte Microsoft', rang: 0 },
  { phase: 'licence', libelle: 'Licence Minecraft', rang: 1 },
  { phase: 'mods', libelle: 'Mods', rang: 2 },
  { phase: 'pret', libelle: 'Prêt à jouer', rang: 3 },
  { phase: 'lancement', libelle: 'Jeu lancé', rang: 4 },
];

function avancement(partiel: Partial<Avancement> = {}): Avancement {
  return {
    phase: 'mods',
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
    ...partiel,
  };
}

function pose(partiel: Partial<Installation> = {}): Installation {
  return {
    instance: 'samflix',
    minecraft: '1.21.1',
    neoforge: '21.1.250',
    java: '21.0.5+11',
    mods: 128,
    introuvables: [],
    horsLigne: false,
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
  installee: Installation | null = null;

  /** De quoi pousser un avancement depuis le test. */
  pousser: (a: Avancement) => void = () => {};

  chemin(): Promise<EtapeVue[]> {
    return Promise.resolve(CHEMIN);
  }

  statut(): Promise<Compte | null> {
    return this.refus ? Promise.reject(this.refus) : Promise.resolve(this.compte);
  }

  installation(): Promise<Installation | null> {
    return Promise.resolve(this.installee);
  }

  surAvancement(recevoir: (a: Avancement) => void): Promise<() => void> {
    this.pousser = recevoir;
    return Promise.resolve(() => {});
  }

  surCodeAppareil(): Promise<() => void> {
    return Promise.resolve(() => {});
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

function element(fixture: { nativeElement: unknown }): HTMLElement {
  return fixture.nativeElement as HTMLElement;
}

function texte(fixture: { nativeElement: unknown }): string {
  return element(fixture).textContent ?? '';
}

function etats(fixture: { nativeElement: unknown }): (string | null)[] {
  return Array.from(element(fixture).querySelectorAll('.etape')).map((etape) =>
    etape.getAttribute('data-etat'),
  );
}

function connecte(): Compte {
  return { pseudo: 'Sam', uuid: 'abcdef', possedeLeJeu: true };
}

describe('App', () => {
  it("propose la connexion quand personne n'est connecté", async () => {
    const fixture = await monter(new LauncherDEssai());

    expect(texte(fixture)).toContain('Se connecter avec Microsoft');
    expect(texte(fixture)).toContain('Aucun compte connecté');
  });

  it('affiche le pseudo de la session reprise', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('Sam');
    expect(texte(fixture)).toContain('Se déconnecter');
  });

  it("n'offre pas d'installer à un compte sans licence", async () => {
    // Installer huit cents mégaoctets pour un compte qu'aucun serveur
    // n'acceptera est précisément ce qu'on veut éviter.
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abc', possedeLeJeu: false };

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('ne possède pas Minecraft');
    expect(texte(fixture)).not.toContain('INSTALLER');
  });

  it('propose INSTALLER tant que rien n’est posé', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('INSTALLER');
    expect(texte(fixture)).not.toContain('JOUER');
  });

  it('propose JOUER une fois le pack posé', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    launcher.installee = pose();

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('JOUER');
    // Et le rappel du pack posé, en haut de la fenêtre.
    expect(texte(fixture)).toContain('1.21.1');
    expect(texte(fixture)).toContain('21.1.250');
  });

  it('montre la connexion et la licence acquises avant toute installation', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();

    const fixture = await monter(launcher);

    // Aucune n'est « en cours » : rien ne travaille.
    expect(etats(fixture)).toEqual(['faite', 'faite', 'a-venir', 'a-venir', 'a-venir']);
  });

  it('éclaire la phase en cours et marque celles qui sont passées', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    const fixture = await monter(launcher);

    launcher.pousser(avancement({ phase: 'mods' }));
    await fixture.whenStable();

    expect(etats(fixture)).toEqual(['faite', 'faite', 'en-cours', 'a-venir', 'a-venir']);
  });

  it("n'éclaire pas « prêt à jouer » comme une étape en cours", async () => {
    // C'est un état, pas un travail : le montrer « en cours » laisserait
    // croire qu'il reste quelque chose à attendre.
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    const fixture = await monter(launcher);

    launcher.pousser(avancement({ phase: 'pret', achevee: true }));
    await fixture.whenStable();

    expect(etats(fixture)).toEqual(['faite', 'faite', 'faite', 'faite', 'a-venir']);
  });

  it('affiche la taille, le débit et le temps restant pendant un téléchargement', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    const fixture = await monter(launcher);
    // La barre ne paraît que pendant une installation déclenchée d'ici.
    (fixture.componentInstance as unknown as { installe: { set(v: boolean): void } }).installe.set(
      true,
    );

    launcher.pousser(
      avancement({
        phase: 'mods',
        actif: true,
        octets: 415_000_000,
        total: 830_000_000,
        debit: 8_200_000,
        restant: 51,
        fichier: 'jei-1.21.1.jar',
      }),
    );
    await fixture.whenStable();

    const affiche = texte(fixture);
    expect(affiche).toContain('415 Mo');
    expect(affiche).toContain('830 Mo');
    expect(affiche).toContain('8,2 Mo/s');
    expect(affiche).toContain('51 s');
    expect(affiche).toContain('jei-1.21.1.jar');
  });

  it('passe en progression indéterminée entre deux lots', async () => {
    // Le défaut observé : la barre restait pleine pendant que la résolution
    // des mods travaillait encore.
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    const fixture = await monter(launcher);
    (fixture.componentInstance as unknown as { installe: { set(v: boolean): void } }).installe.set(
      true,
    );

    launcher.pousser(avancement({ phase: 'mods', actif: false, note: 'Résolution des mods…' }));
    await fixture.whenStable();

    const barre = element(fixture).querySelector('.jauge__barre');
    expect(barre?.classList.contains('jauge__barre--indeterminee')).toBe(true);
    expect(texte(fixture)).toContain('Résolution des mods…');
  });

  it('compte la progression sur toute l’installation, pas sur le lot', async () => {
    // Trois étapes utiles : être au début de la troisième vaut deux tiers.
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    const fixture = await monter(launcher);
    (fixture.componentInstance as unknown as { installe: { set(v: boolean): void } }).installe.set(
      true,
    );

    launcher.pousser(avancement({ phase: 'mods', actif: true, octets: 0, total: 1_000 }));
    await fixture.whenStable();

    expect(texte(fixture)).toContain('67 %');
  });

  it('signale les mods introuvables plutôt que de les taire', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = connecte();
    launcher.installee = pose({ introuvables: ['sophisticatedcore (exigé par storage)'] });

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('1 mods');
    expect(texte(fixture)).toContain('sophisticatedcore');
  });

  it('présente une erreur en overlay, et floute ce qu’il y a derrière', async () => {
    // Elle s'affichait en bas de page et passait inaperçue dès qu'on avait
    // fait défiler l'écran.
    const launcher = new LauncherDEssai();
    launcher.refus = "la session enregistrée n'est plus valable";

    const fixture = await monter(launcher);
    const racine = element(fixture);

    expect(racine.querySelector('.overlay')).not.toBeNull();
    expect(racine.querySelector('.app')?.hasAttribute('data-flou')).toBe(true);
    expect(texte(fixture)).toContain("n'est plus valable");
  });

  it('avertit quand la fenêtre Tauri est absente', async () => {
    const launcher = new LauncherDEssai();
    launcher.disponible = false;

    const fixture = await monter(launcher);

    expect(texte(fixture)).toContain('hors de la fenêtre Tauri');
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
