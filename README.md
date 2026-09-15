# mc-launcher

Launcher Minecraft sur mesure pour le réseau 1.21.1 / NeoForge
(`~/IdeaProject/mc-server/docker`).

## État

| brique | état |
|---|---|
| `crates/mc-auth` — chaîne d'authentification Microsoft → Minecraft | écrite, compile, **en attente d'un Client ID Azure** |
| installation du modpack NeoForge côté client | à faire |
| construction de la ligne de commande JVM + Quick Play | à faire |
| interface (Tauri) | à faire |

## `mc-auth`

Déroule la chaîne complète : device code Microsoft → Xbox Live → XSTS →
`login_with_xbox` → vérification de licence → profil.

Le *device code flow* est retenu plutôt que le flux « authorization code » :
pas d'URI de redirection à enregistrer, pas de serveur HTTP local à ouvrir,
et il est explicitement supporté pour l'API Minecraft. Tenant `consumers`
obligatoire (ni `common`, ni un tenant d'entreprise), scope `XboxLive.signin`.

```bash
cargo run --release -p mc-auth -- <CLIENT_ID>
# ou
AZURE_CLIENT_ID=<...> cargo run --release -p mc-auth
```

### À quoi sert-il tout de suite

Microsoft exige **une tentative de connexion réelle** avant d'accepter une
demande d'accès à l'API Minecraft. Ce binaire produit exactement cette
tentative. Tant que l'application n'est pas approuvée, la chaîne va jusqu'au
bout des étapes Xbox puis reçoit un **403** sur `api.minecraftservices.com` —
le message d'erreur le dit explicitement et renvoie vers le formulaire
<https://aka.ms/mce-reviewappid>.

Une fois l'approbation obtenue (compter jusqu'à 24 h de propagation), la même
commande doit afficher pseudo et UUID.

### Erreurs reconnues

Les refus XSTS sont traduits plutôt que renvoyés bruts : compte sans profil
Xbox (2148916233), compte banni (2148916227), pays non couvert (2148916235),
compte enfant sans famille (2148916238), vérification d'âge coréenne
(2148916236 / 2148916237). Les autres étapes renvoient l'étape, le code HTTP
et le corps de la réponse.

## Identifiants Azure

| | valeur |
|---|---|
| Client ID (`appId`) | `7da56647-e0e9-49d5-9aad-0c03997f3904` |
| Object ID | `2d99bfa2-6ec4-424c-835f-11aec5a59bc8` |
| Tenant ID (annuaire propriétaire) | `c3fe7412-ef6b-48ab-843b-6169e3b379cf` |
| Annuaire | `thesam1798gmail.onmicrosoft.com` |
| `signInAudience` | `AzureADandPersonalMicrosoftAccount` |
| Client public autorisé | oui (`isFallbackPublicClient`) |

Le Client ID **n'est pas un secret** : il est destiné à être embarqué dans le
binaire distribué aux joueurs. Aucun secret client n'est utilisé.

Créé en ligne de commande, le portail étant peu automatisable :

```bash
az ad app create --display-name mc-launcher \
  --sign-in-audience AzureADandPersonalMicrosoftAccount \
  --is-fallback-public-client true
```

## Prérequis Azure

1. Portail Azure → Microsoft Entra ID → Inscriptions d'applications → Nouvelle
   inscription.
2. Types de comptes : *comptes dans un annuaire organisationnel **et** comptes
   Microsoft personnels*.
3. Aucun secret client n'est utilisé par ce flux.
4. Authentification → **Autoriser les flux clients publics : Oui** (requis par
   le device code flow).
5. Lancer `mc-auth <CLIENT_ID>` une fois, aller jusqu'au 403.
6. Soumettre Client ID + Tenant ID sur <https://aka.ms/mce-reviewappid>.
