# Authentification

[← README](../README.md)

```bash
mc-auth login                     # ouvre une session Microsoft et l'enregistre
mc-auth whoami                    # qui est connecté
mc-auth logout                    # oublier la session
mc-auth --offline <PSEUDO>        # profil local, sans Microsoft

mc-pack launch                    # joue avec le compte connecté
mc-pack launch --pseudo <NOM>     # joue hors ligne, sans compte
```

La chaîne est un *device code flow* : Microsoft → Xbox Live → XSTS →
`login_with_xbox` → licence → profil. Il évite une URI de redirection et un
serveur HTTP local ; le joueur ouvre une page et saisit un code.

La session vit dans le **trousseau du système** — Secret Service, Keychain,
Credential Manager — sous le service `samflix-mc`. Elle contient un jeton de
rafraîchissement, qui rouvre le compte sans mot de passe ni second facteur, et
se renouvelle toute seule d'un lancement à l'autre.

Une machine sans trousseau retombe sur `~/.config/samflix-mc/session.json`, en
`0600`, avec un `warn` dans le journal. `SAMFLIX_SANS_TROUSSEAU=1` force ce
second chemin — sur un poste où le portefeuille redemande sa phrase à chaque
accès, ou dans une suite de tests : le fichier s'isole en déplaçant
`XDG_CONFIG_HOME`, le trousseau non.

### Où ce fichier se trouve, plateforme par plateforme

Le chemin ne se devine plus : il vient de `mc-chemins`, et l'application lui
impose au démarrage les racines que Tauri connaît, pour que les crates et la
fenêtre ne tombent jamais sur deux arborescences différentes.

| Système | Session | Données installées |
|---|---|---|
| Linux | `~/.config/samflix-mc/` | `~/.local/share/samflix-mc/` |
| macOS | `~/Library/Application Support/samflix-mc/` | **le même répertoire** |
| Windows | `%APPDATA%\samflix-mc\` | **le même répertoire** |

**La réserve, et elle compte.** Sous macOS et Windows, configuration et données
sont le même répertoire — c'est ce que rend le résolveur de Tauri
(`path/desktop.rs:62-63,73-74`), et s'en écarter ferait diverger l'application
de ses propres crates, ce qui serait bien pire.

Conséquence : la promesse « supprimer les données installées sans perdre ses
préférences ni sa session » **ne vaut que sous Linux**. Ailleurs, effacer le
répertoire pour repartir de zéro efface aussi la session — le joueur devra se
reconnecter. Ce n'est pas grave, puisque le jeton vit d'abord dans le trousseau,
qui n'est pas dans ce répertoire ; mais il faut le dire à qui donne la consigne
« supprime le dossier et relance ».

Le segment est `samflix-mc` et non l'identifiant `mc.samflix.launcher` : on
prend de Tauri les racines NUES — `data_dir()`, et non `app_data_dir()` — pour
y joindre notre segment. C'est ce qui fait qu'aucune migration n'est nécessaire :
le chemin obtenu est, au caractère près, celui que les crates calculaient déjà.

Le choix du mode est explicite. `--pseudo` demande une session hors-ligne ; son
absence demande le compte enregistré. Aucun repli silencieux de l'un vers
l'autre : entrer sur un serveur sous une identité qu'on n'a pas choisie est
exactement ce qu'on veut éviter.

Les refus XSTS sont traduits : compte sans profil Xbox (2148916233), banni
(2148916227), pays non couvert (2148916235), compte enfant (2148916238).

## Mode hors-ligne

L'UUID suit la règle du serveur vanilla,
`UUID.nameUUIDFromBytes("OfflinePlayer:<pseudo>")` — UUID v3 sur MD5. Les
backends tournent en `online-mode=false` et le calculent ainsi ; un UUID
arbitraire donnerait un joueur différent à chaque connexion, inventaire et
permissions perdus.

Aucun jeton produit : serveur hors-ligne uniquement.

## Ce que ce launcher présente à Microsoft, et pourquoi

Le 16 septembre 2026, Mojang Enforcement a refusé l'inscription Azure de ce
launcher pour la liste blanche de l'API Minecraft — sans motif, sans recours :

> *Your application(s) in this batch did not meet the required criteria and
> were not approved for the allow list. […] We do not provide consultation or
> troubleshoot API access.*

Le diagnostic a été fait : la chaîne Microsoft → Xbox Live → XSTS fonctionnait,
le jeton XSTS était délivré, et seul `api.minecraftservices.com` répondait
**403** — sur la seule foi de l'identifiant d'application. Le même code avec
l'identifiant d'un launcher enregistré avant la mise en place du filtrage passe
sans rien changer d'autre. Il n'y a donc rien à corriger dans le code, et aucun
critère public à satisfaire.

Ce launcher présente désormais l'identité du **launcher officiel**
(`00000000402b5328`), via [`minecraft-auth`](https://github.com/CCBlueX/minecraft-auth-rs),
comme [LiquidBounce](https://liquidbounce.net/blog/article/WhKoUYc3) après le
même refus.

> **À savoir avant de se connecter.** Ce n'est pas une approbation obtenue,
> c'est un filtrage contourné. Cela enfreint les conditions d'utilisation de
> Microsoft et de Mojang. Aucune sanction liée à cette méthode n'est documentée
> à ce jour, mais s'il devait y en avoir une, elle viserait le compte du
> joueur — pas seulement celui du mainteneur. `--pseudo` reste disponible pour
> jouer sans connexion sur les serveurs qui l'acceptent, dont ceux du réseau.

> Ne pas emprunter le Client ID d'un launcher public pour un autre projet.
> C'est l'identité d'une application : l'écran de consentement afficherait le
> nom de l'autre projet, et un usage inattendu ferait suspendre *son*
> inscription.

La bibliothèque a été auditée : elle ne contacte que Microsoft, Xbox et Mojang
— `login.live.com`, `login.microsoftonline.com`, `auth.xboxlive.com`,
`device.auth.xboxlive.com`, `sisu.xboxlive.com`, `api.minecraftservices.com`.
Aucun serveur tiers, aucune télémétrie ; les jetons ne quittent pas la machine.

Elle est sous **LGPL-3.0-or-later**, quand le reste du dépôt est en MIT. Lier
du LGPL dans un binaire distribué engage la §4 de cette licence : le
destinataire doit pouvoir relier le binaire avec sa propre version de la
bibliothèque. Tant que le launcher est distribué avec ses sources, recompiler
suffit. Diffuser un binaire seul demanderait d'y joindre de quoi refaire
l'édition de liens.
