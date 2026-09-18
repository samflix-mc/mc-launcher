# Les fonds du launcher

Trois images, choisies depuis **Configuration → Apparence du launcher**. Le
service de réglages pose `data-fond` sur `<html>` ; `web/src/styles.css` fait
le reste.

## Ce qu'il y a ici

`spawn.webp` est l'illustration que le design system livre comme **défaut, en
attendant qu'un serveur publie la sienne** : « Wilderness Bound », l'image clé
de Mojang, prise dans `docs/helm-design-system.zip` sous `assets/Artwork/` et
convertie en WebP 1920 × 1080. C'est elle qu'on voit au démarrage.

`nether.webp` et `fin.webp` sont **encore des dégradés générés à la main**, et
ils se remarquent maintenant : à côté d'une vraie illustration, ils font ce
qu'ils sont. À remplacer par des captures du serveur, mêmes noms, même format.

## Ce qui a changé avec le design system

Ces images ne sont plus posées sur `body` mais sur `.hm-stage__art`, à
l'intérieur de la coque. La scène porte par-dessus le dégradé de lisibilité du
design system — le fond à 52 % en haut, 36 % au milieu, 80 % en bas — qui n'est
pas réglable et qui fait, à lui seul, que la barre du bas se lit quelle que soit
l'image.

## Les contraintes, et pourquoi elles existent

| Contrainte                  | Raison                                                                                                                                                                                                                                           |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1920 × 1080 au moins        | En dessous, `background-size: cover` étire sur un écran ordinaire, et le flou du verre fait ressortir les artefacts.                                                                                                                             |
| WebP, qualité ~80           | Le launcher embarque ces fichiers : un PNG de 1920×1080 pèse plusieurs mégaoctets, contre quelques dizaines de kilooctets ici.                                                                                                                   |
| Pas de contrainte de clarté | Le voile n'est **plus** borné par le bas : ce qui tient le contraste est le dégradé de la scène et l'épaisseur du verre, qui s'épaissit de lui-même quand le voile s'amincit. Une image claire est donc permise — c'en est même une aujourd'hui. |
| Peu de détail au centre     | C'est là que passent les cartes et le bouton. Un détail chargé sous du texte le rend illisible, quel que soit le voile.                                                                                                                          |

## Après remplacement : rien à remesurer

Le plancher du voile (`VOILE_PLANCHER` dans `crates/mc-reglages/src/bornes.rs`)
vaut zéro, et il ne dépend d'aucune image. La lisibilité est tenue par
`--glass-epaisseur` dans `web/src/styles.css` : à voile nul et sur une image
BLANCHE, le texte le plus clair du design system tient encore 4,5:1 sur un
panneau. Remplacer ces fichiers n'oblige donc à remesurer quoi que ce soit.
