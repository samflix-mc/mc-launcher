import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Comment on parle à Rust.
 *
 * ## Pourquoi deux transports
 *
 * Dans la fenêtre, c'est `invoke`. Mais la fenêtre n'a ni rechargement à
 * chaud, ni inspecteur en production, ni moyen de se mettre dans un état
 * choisi : y travailler l'interface coûte un build complet par essai, et les
 * états rares — trois mods introuvables, une installation à mi-parcours — ne
 * se provoquent pas.
 *
 * Hors de la fenêtre, on parle donc au serveur de développement, qui sert les
 * MÊMES commandes sur HTTP. Le front tourne alors dans un navigateur
 * ordinaire. Voir `crates/mc-app/src/dev/`.
 *
 * ## Ce que ce fichier NE fait pas
 *
 * Il ne connaît aucune commande. Il transporte un nom et des arguments, et
 * rend ce qui revient. La liste des commandes vit dans `Pont`, et une seule
 * fois : c'est ce qui garantit que les deux transports servent le même
 * contrat.
 */

/** L'adresse du serveur de développement. Voir `dev::PORT`. */
const SERVEUR_DEV = 'http://127.0.0.1:1421';

/** Vrai dans la fenêtre Tauri, faux dans un navigateur ordinaire. */
export const DANS_TAURI = isTauri();

/**
 * Appelle une commande, par le transport qui convient.
 *
 * Les erreurs remontent de la même façon des deux côtés : `invoke` rejette
 * avec la valeur d'erreur — une chaîne — et le serveur de développement rend
 * cette même chaîne en JSON avec un code d'échec. `messageDErreur` les traite
 * donc toutes les deux sans rien savoir du transport.
 */
export async function appeler<T>(
  commande: string,
  arguments_?: Record<string, unknown>,
): Promise<T> {
  if (DANS_TAURI) {
    return invoke<T>(commande, arguments_);
  }

  const reponse = await fetch(`${SERVEUR_DEV}/commande/${commande}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(arguments_ ?? {}),
  });

  const charge = await reponse.json();
  if (!reponse.ok) {
    throw charge;
  }
  return charge as T;
}

/**
 * Le flux d'événements du serveur de développement — **un seul pour tous**.
 *
 * ## Pourquoi il est partagé, et ce que coûtait de ne pas l'être
 *
 * Chaque abonnement ouvrait sa propre connexion SSE, et une connexion SSE ne
 * se ferme jamais d'elle-même. Le launcher en pose trois — le code d'appareil,
 * l'avancement, l'ouverture de session — et un navigateur n'autorise que **six
 * connexions simultanées par origine** en HTTP/1.1.
 *
 * Deux onglets suffisaient donc à consommer les six, et toutes les requêtes
 * suivantes attendaient un créneau qui ne venait plus. Le symptôme : l'écran de
 * démarrage qui ne s'efface jamais, sans une ligne en console — les requêtes
 * n'échouaient pas, elles n'étaient pas encore parties.
 *
 * Un flux unique, démultiplexé par nom, ramène le coût à UNE connexion quel que
 * soit le nombre d'abonnés. C'est d'ailleurs ce que le serveur fait déjà de son
 * côté : il n'a qu'un canal de diffusion, et chaque message porte son nom.
 */
let flux: EventSource | null = null;

/** Qui écoute quoi. Un même nom peut avoir plusieurs abonnés. */
const abonnes = new Map<string, Set<(charge: unknown) => void>>();

function fluxPartage(): EventSource {
  if (flux) {
    return flux;
  }
  const source = new EventSource(`${SERVEUR_DEV}/evenements`);
  source.onmessage = (message) => {
    try {
      const recu = JSON.parse(message.data) as { evenement: string; charge: unknown };
      for (const recevoir of abonnes.get(recu.evenement) ?? []) {
        recevoir(recu.charge);
      }
    } catch {
      // Un message qu'on ne sait pas lire n'a pas à casser le flux : le
      // suivant porte l'état complet, pas un delta.
    }
  };
  flux = source;
  return source;
}

/**
 * S'abonne à un événement.
 *
 * Dans la fenêtre, `listen` ouvre un canal par nom. Sur HTTP, il n'y a qu'un
 * flux — voir ci-dessus — et chaque message porte le nom de son événement :
 * c'est ici qu'on démultiplexe, pour que l'appelant ne voie aucune différence.
 */
export async function ecouter<T>(
  evenement: string,
  recevoir: (charge: T) => void,
): Promise<UnlistenFn> {
  if (DANS_TAURI) {
    return listen<T>(evenement, (recu) => recevoir(recu.payload));
  }

  fluxPartage();
  const ecouteur = recevoir as (charge: unknown) => void;
  const pour = abonnes.get(evenement) ?? new Set();
  pour.add(ecouteur);
  abonnes.set(evenement, pour);

  return () => {
    pour.delete(ecouteur);
    // La connexion reste ouverte tant qu'il reste un abonné, et se ferme quand
    // le dernier part : la rouvrir coûte un aller-retour, la garder ouverte
    // pour personne coûte un des six créneaux du navigateur.
    if ([...abonnes.values()].every((ensemble) => ensemble.size === 0)) {
      flux?.close();
      flux = null;
    }
  };
}

/**
 * Ouvre une URL hors de l'application.
 *
 * Dans la fenêtre, le greffon de Tauri ; ailleurs, le navigateur lui-même —
 * où l'on est déjà.
 */
export async function ouvrirHorsApplication(url: string): Promise<void> {
  if (DANS_TAURI) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    return openUrl(url);
  }
  window.open(url, '_blank', 'noopener');
}
