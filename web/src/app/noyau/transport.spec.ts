import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { appeler, ecouter, ouvrirHorsApplication } from './transport';

/**
 * Un faux `EventSource`, qui compte ses instances.
 *
 * C'est le seul moyen d'éprouver ce que ce module a de subtil : combien de
 * connexions il ouvre. jsdom n'en fournit pas, et une vraie connexion
 * demanderait un serveur.
 */
class FauxFlux {
  static ouverts: FauxFlux[] = [];
  static vivants = 0;

  onmessage: ((message: { data: string }) => void) | null = null;
  ferme = false;

  constructor(readonly url: string) {
    FauxFlux.ouverts.push(this);
    FauxFlux.vivants += 1;
  }

  close() {
    this.ferme = true;
    FauxFlux.vivants -= 1;
  }

  /** Pousse un message, comme le ferait le serveur. */
  pousser(evenement: string, charge: unknown) {
    this.onmessage?.({ data: JSON.stringify({ evenement, charge }) });
  }
}

describe('transport', () => {
  /**
   * Les désabonnements du test en cours.
   *
   * Le flux est un état de MODULE — c'est tout l'intérêt : une connexion pour
   * tout le monde. Un test qui laisserait un abonné derrière lui garderait donc
   * le flux ouvert pour le suivant, qui compterait une connexion qu'il n'a pas
   * demandée. On les rend tous, à chaque fois.
   */
  let aQuitter: (() => void)[] = [];

  /** S'abonne, et retient le désabonnement. */
  async function abonner(evenement: string, recevoir: (charge: unknown) => void) {
    const quitter = await ecouter(evenement, recevoir);
    aQuitter.push(quitter);
    return quitter;
  }

  beforeEach(() => {
    FauxFlux.ouverts = [];
    FauxFlux.vivants = 0;
    aQuitter = [];
    vi.stubGlobal('EventSource', FauxFlux);
  });

  afterEach(() => {
    for (const quitter of aQuitter) {
      quitter();
    }
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  /**
   * **Le défaut qui a motivé la mutualisation.**
   *
   * Chaque abonnement ouvrait sa propre connexion SSE, laquelle ne se ferme
   * jamais d'elle-même. Le launcher en pose trois, et un navigateur n'autorise
   * que six connexions simultanées par origine en HTTP/1.1 : deux onglets
   * suffisaient à les consommer toutes, et les requêtes suivantes attendaient
   * un créneau qui ne venait plus.
   *
   * Le symptôme était muet — les requêtes n'échouaient pas, elles n'étaient pas
   * encore parties.
   */
  it('trois abonnements n’ouvrent QU’UNE connexion', async () => {
    await abonner('a', () => {});
    await abonner('b', () => {});
    await abonner('c', () => {});

    expect(FauxFlux.vivants).toBe(1);
  });

  /** Chaque message n'est remis qu'à ceux qui ont demandé ce nom-là. */
  it('le flux est démultiplexé par nom', async () => {
    const recuA: unknown[] = [];
    const recuB: unknown[] = [];
    await abonner('a', (charge) => recuA.push(charge));
    await abonner('b', (charge) => recuB.push(charge));

    FauxFlux.ouverts[0].pousser('a', { valeur: 1 });
    FauxFlux.ouverts[0].pousser('b', { valeur: 2 });
    FauxFlux.ouverts[0].pousser('inconnu', { valeur: 3 });

    expect(recuA).toEqual([{ valeur: 1 }]);
    expect(recuB).toEqual([{ valeur: 2 }]);
  });

  /** Deux abonnés au MÊME nom reçoivent tous les deux. */
  it('plusieurs abonnés au même événement sont tous servis', async () => {
    let premier = 0;
    let second = 0;
    await abonner('avancement', () => (premier += 1));
    await abonner('avancement', () => (second += 1));

    FauxFlux.ouverts[0].pousser('avancement', null);

    expect([premier, second]).toEqual([1, 1]);
  });

  /**
   * La connexion reste ouverte tant qu'il reste un abonné, et se ferme quand le
   * dernier part : la rouvrir coûte un aller-retour, la garder ouverte pour
   * personne coûte un des six créneaux du navigateur.
   */
  it('la connexion se ferme quand le dernier abonné part', async () => {
    const quitterA = await abonner('a', () => {});
    const quitterB = await abonner('b', () => {});

    quitterA();
    expect(FauxFlux.vivants).toBe(1);

    quitterB();
    expect(FauxFlux.vivants).toBe(0);
  });

  /** Et elle se rouvre proprement au prochain abonnement. */
  it('un abonnement après la fermeture rouvre le flux', async () => {
    const quitter = await abonner('a', () => {});
    quitter();
    await abonner('a', () => {});

    expect(FauxFlux.vivants).toBe(1);
    expect(FauxFlux.ouverts.length).toBe(2);
  });

  /**
   * Un message qu'on ne sait pas lire n'a pas à casser le flux : le suivant
   * porte l'état complet, pas un delta.
   */
  it('un message illisible ne casse pas le flux', async () => {
    const recu: unknown[] = [];
    await abonner('a', (charge) => recu.push(charge));

    FauxFlux.ouverts[0].onmessage?.({ data: 'pas du json' });
    FauxFlux.ouverts[0].pousser('a', 'après');

    expect(recu).toEqual(['après']);
  });

  /** Une réponse ordinaire rend sa charge utile, telle quelle. */
  it('une commande qui réussit rend ce que le serveur a écrit', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: true, json: async () => ({ pseudo: 'thesam1798' }) })),
    );

    await expect(appeler('statut')).resolves.toEqual({ pseudo: 'thesam1798' });
  });

  /**
   * **Une erreur rejette avec la valeur d'erreur telle quelle.**
   *
   * C'est ce que fait `invoke` : `messageDErreur` traite donc les deux
   * transports par le même chemin, sans rien savoir d'eux.
   */
  it('une commande qui échoue rejette avec la valeur du serveur', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: false, json: async () => 'commande inconnue : truc' })),
    );

    await expect(appeler('truc')).rejects.toBe('commande inconnue : truc');
  });

  /** Hors de la fenêtre, une URL s'ouvre par le navigateur lui-même. */
  it('hors de Tauri, une URL part dans un nouvel onglet', async () => {
    const ouvrir = vi.fn();
    vi.stubGlobal('window', { ...globalThis.window, open: ouvrir });

    await ouvrirHorsApplication('https://microsoft.com/link');

    expect(ouvrir).toHaveBeenCalledWith('https://microsoft.com/link', '_blank', 'noopener');
  });
});
