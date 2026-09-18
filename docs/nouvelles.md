# Le fil de nouvelles

[← README](../README.md)

La page **Nouvelles** du launcher, et la carte « dernière news » de Spawn,
lisent un fichier JSON servi par le **même hôte que le pack**.

## Où le launcher va le chercher

L'adresse est **dérivée** de celle du pack, jamais recopiée :

```
https://mc-launcher.ggy.info/samflix.json   →  …/nouvelles.json
https://mc-launcher-dev.ggy.info/samflix.json → …/nouvelles.json
```

C'est le même motif que pour le verrou, et pour la même raison : recopier les
trois adresses publiées referait exactement le défaut que leur commentaire
raconte — une préproduction qui télécharge le contenu de la production, et
n'éprouve donc rien de ce qu'elle est censée éprouver.

**Un fil absent n'est pas une erreur** : la page s'affiche vide. Le launcher
doit s'ouvrir sur un hôte qui ne publie pas encore de nouvelles.

## Le contrat

```jsonc
{
  "schema": 1,
  "billets": [
    {
      "id": "2026-09-saison-3",        // stable : c'est la clé du front
      "titre": "La saison 3 ouvre",
      "date": "2026-09-18T18:00:00Z",  // RFC 3339, en UTC
      "epinglee": true,                // facultatif, défaut false
      "image": "images/saison3.webp",  // facultatif, relatif au fil
      "corps": "Texte **markdown**."
    }
  ]
}
```

| Champ | Obligatoire | Ce qui arrive s'il est fautif |
|---|---|---|
| `schema` | oui | un autre numéro **se lit quand même**, et le dit dans le repli « écartés » |
| `id` | oui | billet ignoré |
| `titre` | oui | vide ou blanc → billet écarté |
| `date` | oui | hors RFC 3339 UTC → billet écarté |
| `epinglee` | non | absent = `false` |
| `image` | non | hors de l'hôte du fil → image ignorée, **billet gardé** |
| `corps` | oui | ce qui n'est pas reconnu devient du texte |

**Un billet fautif est écarté seul.** C'est la différence entre « la page des
news a un trou » et « la page des news est vide », et la seconde se lit comme
une panne du launcher — alors que la cause est une virgule dans un fichier
qu'on ne contrôle pas au même rythme que le code.

Le tri est fait par le launcher, en Rust : **les épinglés d'abord, puis du plus
récent au plus ancien**. L'ordre du tableau n'a donc aucune importance.

## Le markdown reconnu, et pas un caractère de plus

Paragraphes, titres, listes à puces, séparateurs ; gras, italique, code,
liens. **Rien d'autre** — ni tableaux, ni HTML brut, ni images dans le corps,
ni blocs de code multilignes.

```markdown
## Un titre
Un paragraphe avec du **gras**, de l'*italique*, du `code`
et un [lien](https://mc-launcher.ggy.info/regles).

- une puce
- une autre

---
```

Ce n'est pas un sous-ensemble par paresse. Chaque construction reconnue est une
forme de plus que le front doit savoir rendre, et une occasion de plus qu'un
contenu distant a de produire quelque chose d'inattendu.

**La règle : ce qui n'est pas reconnu devient du TEXTE, jamais du balisage.**
Un `<script>` écrit dans un billet ressort comme les onze caractères qu'il est.
Le markdown est analysé en Rust vers un arbre typé, et le front le rend avec un
`@switch` : **aucun HTML distant n'atteint le DOM**. C'est la condition à
laquelle le CSP du launcher a été desserré — voir [interface.md](interface.md).

Les niveaux de titre sont ramenés dans 2..=4 : le titre du billet occupe le
niveau 1 de la page, et un `#` du corps ne doit pas le concurrencer — ni au
rendu, ni pour un lecteur d'écran, qui se sert de la hiérarchie pour naviguer.
Au-delà de six dièses, ce n'est plus un titre, c'est du texte.

## Les liens, et les images

Les deux passent par le même contrôle : **même schéma et même hôte que le
fil**.

Une URL refusée ne fait pas disparaître le texte — le libellé du lien devient
du texte ordinaire. Effacer une phrase parce que son lien déplaît serait pire
que de la garder sans lien. Chaque refus se journalise.

Le contrôle porte sur l'**hôte entier**, jamais sur un préfixe :
`https://mc-launcher.ggy.info.evil.com/` commence par le bon hôte et n'est pas
le bon hôte.

Les liens externes des billets n'ouvrent donc rien dans la fenêtre. Pour
pointer ailleurs, il faut passer par une page de l'hôte.

### Les bornes, qui ne sont pas négociables

| | |
|---|---|
| Taille d'une image | **512 kio** |
| Images par fil | **8** |
| Format | PNG, JPEG, GIF ou WebP, **déduit des octets** |

Le téléchargeur n'a aucune limite propre et accumule le corps entier en
mémoire, sous un délai de trois cents secondes : un hôte fautif — ou compromis
— servirait un fichier de plusieurs gigaoctets, et le launcher grossirait
jusqu'à se faire tuer par le système, sans un message.

Le type est déduit des **nombres magiques**, pas de l'extension : une extension
vient de l'URL, donc de l'hôte, donc de quelque chose qu'on ne contrôle pas au
même rythme que le code.

Les images sont rangées en fichiers sous `cache/news/<hôte>/images/`, nommées
par l'empreinte de leur URL — et non par leur dernier segment, puisque deux
billets peuvent porter une `image.webp` chacun. Le `data:` que la fenêtre
consomme n'est fabriqué qu'à la demande : garder les images en base64 dans le
cache JSON ferait qu'un fil de huit billets illustrés pèserait trois mégaoctets
et demi, relus et resérialisés à chaque ouverture de la page.

## Absent, injoignable, retiré : trois choses différentes

| Ce qui se passe | Ce que la page montre |
|---|---|
| l'hôte rend **404** | rien — une page vide, et aucun message |
| l'hôte **ne répond pas**, et une copie existe | le fil d'hier, marqué hors ligne |
| l'hôte **ne répond pas**, et aucune copie | une erreur franche |
| l'hôte rend 404 **alors qu'une copie existe** | rien — le fil a été retiré exprès |

La première ligne n'est pas une subtilité : au moment où cette page a été
écrite, **les trois hôtes rendaient 404**. Traiter ce cas comme une panne
affichait donc une erreur à tous les joueurs, sur une page où il n'y avait
simplement rien à dire.

La dernière ligne est celle qui demande à être défendue : on pourrait servir
l'ancien fil. Mais un billet retiré de l'hôte l'a été par quelqu'un, et le
réafficher parce qu'on en garde une copie serait désobéir à ce geste.

La copie n'est écrite qu'**après relecture** : une réponse tronquée ou une page
d'erreur HTML remplaceraient sinon un fil valide par du vide.

## Ce qui est attendu de mc-content

Deux choses, indépendantes l'une de l'autre :

1. **un `nouvelles.json` à côté de `samflix.json`**, sur les trois hôtes. Un
   fichier à un seul billet suffit à ouvrir la page ; celui qui suit est valide
   et se colle tel quel ;
2. **un verrou de développement à `generation` incrémentée**, pour éprouver la
   chaîne de purge de bout en bout. C'est le seul moyen de vérifier qu'un
   changement de génération efface bien `mods/`, `shaderpacks/`,
   `resourcepacks/` et `libraries/` — et rien d'autre. Voir
   [packs.md](packs.md).

```json
{
  "schema": 1,
  "billets": [
    {
      "id": "2026-09-ouverture",
      "titre": "Le launcher est là",
      "date": "2026-09-18T18:00:00Z",
      "epinglee": true,
      "corps": "Le launcher installe le pack et lance le jeu **en un seul geste**.\n\nCe qu'il faut savoir :\n\n- il installe ce que le serveur charge, à la version près\n- il pose le Java qu'il faut, sans toucher à celui du système\n- il ne réinstalle rien tant que le pack n'a pas bougé\n\nLes réglages sont dans `Configuration`."
    },
    {
      "id": "2026-09-regles",
      "titre": "Les règles du serveur",
      "date": "2026-09-17T12:00:00Z",
      "corps": "Trois règles, et elles tiennent en une ligne chacune.\n\n## Le respect\n\nPas d'insulte, pas de harcèlement. C'est la seule qui mène à un bannissement immédiat.\n\n## Les constructions\n\nOn ne casse pas chez les autres. Un `/back` mal placé n'est pas une excuse.\n\n## Les mods\n\nCeux du pack, et eux seuls — le serveur vérifie de toute façon."
    }
  ]
}
```
