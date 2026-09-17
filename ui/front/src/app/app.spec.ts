import { TestBed } from '@angular/core/testing';

import { App } from './app';
import { Launcher, messageDErreur, type Compte } from './launcher';

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

  statut(): Promise<Compte | null> {
    return this.refus ? Promise.reject(this.refus) : Promise.resolve(this.compte);
  }

  lancerJeu(): Promise<string> {
    return Promise.resolve('pas encore branché — mc-pack launch');
  }
}

function monter(launcher: LauncherDEssai) {
  TestBed.configureTestingModule({
    imports: [App],
    providers: [{ provide: Launcher, useValue: launcher }],
  });
  return TestBed.createComponent(App);
}

describe('App', () => {
  it("propose la connexion quand personne n'est connecté", async () => {
    const fixture = monter(new LauncherDEssai());
    await fixture.whenStable();

    const texte = (fixture.nativeElement as HTMLElement).textContent ?? '';
    expect(texte).toContain('Se connecter avec Microsoft');
    expect(texte).not.toContain('Se déconnecter');
  });

  it('affiche le profil de la session reprise', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abcdef', possedeLeJeu: true };

    const fixture = monter(launcher);
    await fixture.whenStable();

    const texte = (fixture.nativeElement as HTMLElement).textContent ?? '';
    expect(texte).toContain('Sam');
    expect(texte).toContain('abcdef');
    expect(texte).toContain('Se déconnecter');
  });

  it('signale un compte sans licence sans refuser la session', async () => {
    const launcher = new LauncherDEssai();
    launcher.compte = { pseudo: 'Sam', uuid: 'abcdef', possedeLeJeu: false };

    const fixture = monter(launcher);
    await fixture.whenStable();

    const texte = (fixture.nativeElement as HTMLElement).textContent ?? '';
    expect(texte).toContain('ne possède pas Minecraft');
    // La session reste ouverte : c'est un avertissement, pas un échec.
    expect(texte).toContain('Se déconnecter');
  });

  it('montre une session expirée comme un message, pas comme un écran vide', async () => {
    const launcher = new LauncherDEssai();
    launcher.refus = "la session enregistrée n'est plus valable — reconnecte-toi";

    const fixture = monter(launcher);
    await fixture.whenStable();

    const texte = (fixture.nativeElement as HTMLElement).textContent ?? '';
    expect(texte).toContain("n'est plus valable");
    // Et le bouton reste utilisable, sinon il n'y a plus de sortie.
    expect(texte).toContain('Se connecter avec Microsoft');
  });

  it('avertit quand la fenêtre Tauri est absente', async () => {
    const launcher = new LauncherDEssai();
    launcher.disponible = false;

    const fixture = monter(launcher);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    expect(element.textContent).toContain('hors de la fenêtre Tauri');
    expect(element.querySelector<HTMLButtonElement>('.bouton--principal')?.disabled).toBe(true);
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
