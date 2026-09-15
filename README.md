# mc-launcher

Launcher de bureau pour un réseau Minecraft 1.21.1 / NeoForge.

| brique | état |
|---|---|
| `crates/mc-auth` — authentification Microsoft | écrite, en attente de l'approbation Azure |
| installation du modpack côté client | à faire |
| ligne de commande JVM + Quick Play | à faire |
| interface | à faire |

## mc-auth

Device code Microsoft → Xbox Live → XSTS → `login_with_xbox` → licence → profil.

```bash
cargo run -p mc-auth --release -- <CLIENT_ID>        # chaîne complète
cargo run -p mc-auth --release -- --offline <PSEUDO> # profil local
```

Le *device code flow* évite une URI de redirection et un serveur HTTP local.
Tenant `consumers` obligatoire, scope `XboxLive.signin`, pas de secret client.

Les refus XSTS sont traduits : compte sans profil Xbox (2148916233), banni
(2148916227), pays non couvert (2148916235), compte enfant (2148916238).

## Mode hors-ligne

L'UUID suit la règle du serveur vanilla,
`UUID.nameUUIDFromBytes("OfflinePlayer:<pseudo>")` — UUID v3 sur MD5. Les
backends tournent en `online-mode=false` et le calculent ainsi ; un UUID
arbitraire donnerait un joueur différent à chaque connexion.

Aucun jeton produit : serveur hors-ligne uniquement.

> Ne pas emprunter le Client ID d'un launcher public. C'est l'identité d'une
> application : l'écran de consentement afficherait le nom de l'autre projet, et
> un usage inattendu ferait suspendre *son* inscription.

## Azure

| | |
|---|---|
| Client ID | `7da56647-e0e9-49d5-9aad-0c03997f3904` |
| Tenant ID | `c3fe7412-ef6b-48ab-843b-6169e3b379cf` |
| `signInAudience` | `AzureADandPersonalMicrosoftAccount` |

Le Client ID n'est pas un secret, il est destiné au binaire distribué.

```bash
az ad app create --display-name mc-launcher \
  --sign-in-audience AzureADandPersonalMicrosoftAccount \
  --is-fallback-public-client true
```

Tant que Microsoft n'a pas approuvé l'application, `login_with_xbox` répond
**403**. Cette tentative est l'activité exigée avant de soumettre
<https://aka.ms/mce-reviewappid>. Revue hebdomadaire.
