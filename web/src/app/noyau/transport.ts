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
 * S'abonne à un événement.
 *
 * Dans la fenêtre, `listen` ouvre un canal par nom. Sur HTTP, il n'y a qu'un
 * flux — SSE n'en a qu'un — et chaque message porte le nom de son événement :
 * c'est ici qu'on démultiplexe, pour que l'appelant ne voie aucune différence.
 */
export async function ecouter<T>(
  evenement: string,
  recevoir: (charge: T) => void,
): Promise<UnlistenFn> {
  if (DANS_TAURI) {
    return listen<T>(evenement, (recu) => recevoir(recu.payload));
  }

  const source = new EventSource(`${SERVEUR_DEV}/evenements`);
  source.onmessage = (message) => {
    try {
      const recu = JSON.parse(message.data) as { evenement: string; charge: T };
      if (recu.evenement === evenement) {
        recevoir(recu.charge);
      }
    } catch {
      // Un message qu'on ne sait pas lire n'a pas à casser le flux : le
      // suivant porte l'état complet, pas un delta.
    }
  };
  return () => source.close();
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
