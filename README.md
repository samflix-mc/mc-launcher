# mc-launcher

Launcher de bureau pour un réseau Minecraft 1.21.1 / NeoForge.

| brique | état |
|---|---|
| `crates/mc-auth` — authentification Microsoft | écrite, en attente de l'approbation Azure |
| `crates/mc-java` — runtime Java | écrite |
| `crates/mc-mods` — résolution des mods | écrite |
| `crates/mc-instance` — jeu et chargeur | écrite |
| `crates/mc-pack` — manifeste et installation | écrite |
| ligne de commande JVM + Quick Play | à faire |
| interface | à faire |

## Installer un pack

```bash
cargo run -p mc-pack --release -- install packs/samflix.json
cargo run -p mc-pack --release -- install packs/samflix.json --locked   # rejoue le verrou
cargo run -p mc-pack --release -- lock   packs/samflix.json             # résout sans installer
cargo run -p mc-pack --release -- verify packs/samflix.json [--deep]
```

L'ordre des étapes découle des dépendances entre elles : la version du
chargeur est résolue d'abord pour que le verrou consigne un numéro et non le
mot `latest` ; les fichiers de Mojang donnent au passage la version de Java
exigée ; l'installateur NeoForge patche le client vanilla et a besoin de ce
Java ; les mods viennent ensuite ; le verrou est écrit en dernier, puisqu'il
décrit ce qui a réellement été fait.

### Le manifeste

```json
{
  "schema": 1,
  "name": "samflix",
  "minecraft": "1.21.1",
  "loader": { "type": "neoforge", "version": "latest" },
  "mods": [
    { "slug": "jei" },
    { "slug": "jade", "file": "eYz2YBGT" },
    { "slug": "embeddium", "side": "client", "channel": "beta" }
  ]
}
```

Seul `slug` est obligatoire ; chacun des autres champs consigne une décision.
`file` fige un build exact — c'est ce qui rend une installation reproductible —
`source` impose Modrinth ou CurseForge, `side` contredit ce que la plateforme
annonce, `channel` autorise une préversion.

### Le verrou

`packs/samflix.lock.json` est écrit à côté du manifeste et se versionne avec
lui. Le manifeste dit ce qu'on veut, le verrou dit ce qu'on a eu : la version
retenue quand aucune n'était imposée, et **les dépendances ajoutées
d'elles-mêmes**, chacune avec la raison de sa présence. `install --locked` le
rejoue à l'identique des mois plus tard.

## Où vivent les fichiers

```
~/.local/share/samflix-mc/
  shared/            versions/, libraries/, assets/ — forme d'un .minecraft
  instances/<nom>/minecraft/   mods, config, saves
  instances/<nom>/server/mods/ les mods du côté serveur
  runtime/temurin-21/
  cache/mods/
```

Bibliothèques et assets pèsent près d'un gigaoctet et ne dépendent que de la
version du jeu : ils sont partagés entre instances. `shared/` a la forme d'un
`.minecraft` parce que l'installateur NeoForge l'exige.

## mc-java

Minecraft 1.21.1 refuse de démarrer sous Java 21. Un joueur n'a aucune raison
d'en avoir un, et celui qu'il a est souvent un 8 ou un 17 laissé par un vieux
modpack.

```bash
cargo run -p mc-java --release            # détecte, installe au besoin
cargo run -p mc-java --release -- --check # détecte seulement
```

Les candidats sont sondés en exécutant `java -version`, jamais en lisant un
chemin. Sans résultat, un Temurin est installé depuis l'API Adoptium, son
SHA-256 vérifié, et l'installation confirmée en relançant le binaire. Ce
runtime n'est pas ajouté au `PATH` et ne touche pas au Java du système.

## mc-mods

Modrinth d'abord, CurseForge en repli. Modrinth a une API ouverte — rien à
distribuer avec le binaire — publie les empreintes et donne la répartition
client/serveur par projet. CurseForge exige une clé nominative, qu'un launcher
ne peut pas embarquer : elle serait extraite du binaire et révoquée.

```bash
export CURSEFORGE_API_KEY=…            # ou ~/.config/samflix-mc/curseforge.key
cargo run -p mc-mods --example resoudre -- --detail jei jade
```

### Les dépendances qu'aucune API ne déclare

Un manifeste nomme cinq mods, le dossier `mods` en contient sept. L'écart, ce
sont les dépendances — et les fiches de publication sont souvent incomplètes,
une bibliothèque ajoutée entre deux versions n'y étant jamais reportée.

Trois sources sont croisées, de la moins à la plus fiable : ce que le manifeste
demande, ce que l'API déclare, et **ce que le jar exige** dans
`META-INF/neoforge.mods.toml` — la seule que le jeu lise. Après téléchargement,
chaque jar est ouvert, ses `modId` obligatoires comparés à ceux que le pack
fournit, et tout manque relance un tour. La boucle s'arrête quand plus rien ne
manque, soit exactement la condition que NeoForge vérifiera au démarrage.

Deux pièges méritent d'être nommés, parce qu'ils produisent des faux positifs
coûteux :

- **JarJar** — un mod embarque ses bibliothèques dans `META-INF/jarjar/`. Elles
  fournissent leur `modId` sans exister comme fichier. Les ignorer ferait
  installer un doublon, et deux versions d'un même `modId` font échouer le
  chargement ;
- **le slug n'est pas le `modId`** — `bookshelf` est publié sous
  `bookshelf-lib`. C'est le `modId` du jar qui fait foi.

Un auteur qui a désactivé la redistribution voit son refus respecté : l'URL du
CDN n'est pas reconstruite, le cas devient un message citant la page du mod.

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
