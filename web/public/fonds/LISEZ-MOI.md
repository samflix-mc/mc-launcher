# Les fonds du launcher

Trois images, choisies depuis **Configuration → Apparence du launcher**. Le
service de réglages pose `data-fond` sur `<html>` ; `web/src/styles.css` fait
le reste.

## Ce qui est ici aujourd'hui est PROVISOIRE

Les trois `.webp` sont des dégradés générés à la main, pour que l'interface ne
soit pas cassée en attendant de vraies captures. Ils font leur travail — le
voile et le verre dépoli se règlent dessus — mais ce ne sont pas des images du
réseau.

**À remplacer** par des captures du serveur : le spawn, un paysage du Nether,
un de l'End. Même noms, même format.

## Les contraintes, et pourquoi elles existent

| Contrainte              | Raison                                                                                                                                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1920 × 1080 au moins    | En dessous, `background-size: cover` étire sur un écran ordinaire, et le flou du verre fait ressortir les artefacts.                                                                                       |
| WebP, qualité ~80       | Le launcher embarque ces fichiers : un PNG de 1920×1080 pèse plusieurs mégaoctets, contre quelques dizaines de kilooctets ici.                                                                             |
| Plutôt sombres          | Le voile est **borné par le bas** à 35 % — voir `mc-reglages` : en dessous, le texte ne tient plus le contraste. Une image claire consommerait toute cette marge et ne laisserait aucun réglage au joueur. |
| Peu de détail au centre | C'est là que passent les cartes et le bouton. Un détail chargé sous du texte le rend illisible, quel que soit le voile.                                                                                    |

## Après remplacement

Le plancher du voile (`VOILE_PLANCHER` dans `crates/mc-reglages/src/bornes.rs`)
est **une mesure, pas un choix** : c'est la valeur en dessous de laquelle le
texte cesse de tenir le contraste minimal sur l'image embarquée **la plus
claire**.

Changer le jeu d'images oblige donc à la remesurer. Une image plus claire que
celles d'aujourd'hui rendrait le plancher insuffisant — et aucun test ne le
verrait, parce qu'aucun test ne regarde un pixel.
