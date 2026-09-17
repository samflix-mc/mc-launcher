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

La session vit dans `~/.config/samflix-mc/session.json`, en `0600` : elle
contient un jeton de rafraîchissement, qui rouvre le compte sans mot de passe
ni second facteur. Elle se renouvelle toute seule d'un lancement à l'autre.

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
